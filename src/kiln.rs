use crate::task::{CrackleTask, TaskMetadata, TaskOutput, Timestamp};
use crate::patterns::{
    ClusteringPattern, ConservationPattern, CorrelationPattern, CracklePattern, PhaseTransitionPattern,
};
use crate::profile::ThermalProfile;
use crate::error::CrackleError;
use crate::information::{entropy, jsd, kl_divergence, mutual_information, permutation_entropy};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

/// A completed task entry stored in the kiln.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TaskEntry {
    /// The task label.
    pub label: String,
    /// Metrics produced during firing.
    pub metrics: Vec<(String, f64)>,
    /// Metrics produced during cooling (may differ from firing).
    pub cooled_metrics: Vec<(String, f64)>,
    /// Task metadata.
    pub metadata: TaskMetadata,
}

impl TaskEntry {
    /// All metrics: cooled metrics override firing metrics of the same name.
    pub fn all_metrics(&self) -> Vec<(String, f64)> {
        let mut result = self.cooled_metrics.clone();
        for (name, val) in &self.metrics {
            if !result.iter().any(|(n, _)| n == name) {
                result.push((name.clone(), *val));
            }
        }
        result
    }
}

/// The kiln — the runtime that fires tasks and cools them to detect patterns.
///
/// Like a pottery kiln, this runtime has two distinct phases:
///
/// 1. **Firing**: Tasks execute (`fire()`), producing outputs and metrics.
///    This is the hot phase — the work gets done.
///
/// 2. **Cooling**: After all tasks have fired, the runtime examines the completed
///    tasks for emergent patterns. The crackle glaze forms in the cooling, not the firing.
///
/// # Example
///
/// ```
/// use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput};
///
/// # fn main() -> crackle_runtime::Result<()> {
/// struct MyTask { x: f64 }
/// impl CrackleTask for MyTask {
///     type Output = f64;
///     fn fire(&self) -> TaskOutput<Self::Output> {
///         TaskOutput::new(self.x, vec![("value".into(), self.x)])
///     }
/// }
///
/// let mut kiln = Kiln::new(ThermalProfile::default());
/// kiln.fire_task(MyTask { x: 1.0 })?;
/// kiln.fire_task(MyTask { x: 2.0 })?;
/// kiln.fire_task(MyTask { x: 3.0 })?;
///
/// let patterns = kiln.cool();
/// # Ok(())
/// # }
/// ```
pub struct Kiln {
    profile: ThermalProfile,
    entries: Vec<TaskEntry>,
    cooled: bool,
}

impl Kiln {
    /// Create a new kiln with the given thermal profile.
    pub fn new(profile: ThermalProfile) -> Self {
        Kiln {
            profile,
            entries: Vec::new(),
            cooled: false,
        }
    }

    /// Create a kiln with default thermal profile.
    pub fn default_profile() -> Self {
        Kiln::new(ThermalProfile::default())
    }

    /// Fire a single task and return its output (without recording).
    ///
    /// Returns the task's output value.
    ///
    /// # Errors
    ///
    /// Returns [`CrackleError::KilnCooled`] if called after `cool()`.
    pub fn fire_task<T: CrackleTask>(&self, task: T) -> crate::Result<TaskOutput<T::Output>> {
        if self.cooled {
            return Err(CrackleError::KilnCooled);
        }

        Ok(task.fire())
    }

    /// Fire a task and record it in the kiln for later cooling.
    ///
    /// This stores the task's metrics internally so patterns can be detected
    /// during the cooling phase.
    ///
    /// # Errors
    ///
    /// Returns [`CrackleError::KilnCooled`] if called after `cool()`.
    pub fn fire_and_record<T: CrackleTask>(&mut self, task: T) -> crate::Result<TaskOutput<T::Output>> {
        if self.cooled {
            return Err(CrackleError::KilnCooled);
        }

        let label = task.label();
        let fired_at = Timestamp::now();
        let start = std::time::Instant::now();

        let output = task.fire();

        let fire_duration = start.elapsed();
        let metadata = TaskMetadata {
            fired_at,
            cooled_at: None,
            fire_duration,
            label: label.clone(),
        };

        let entry = TaskEntry {
            label,
            metrics: output.metrics.clone(),
            cooled_metrics: vec![],
            metadata,
        };

        self.entries.push(entry);
        Ok(output)
    }

    /// Fire multiple tasks in sequence and record them all.
    ///
    /// # Errors
    ///
    /// Returns the first error encountered. Already-fired tasks are still recorded.
    pub fn fire_all<T: CrackleTask>(&mut self, tasks: Vec<T>) -> crate::Result<Vec<TaskOutput<T::Output>>> {
        tasks
            .into_iter()
            .map(|task| self.fire_and_record(task))
            .collect()
    }

    /// Add a pre-computed task entry directly (useful for testing).
    pub fn add_entry(&mut self, label: impl Into<String>, metrics: Vec<(String, f64)>) {
        let label = label.into();
        let metadata = TaskMetadata::new(&label);
        self.entries.push(TaskEntry {
            label,
            metrics,
            cooled_metrics: vec![],
            metadata,
        });
    }

    /// The number of tasks currently in the kiln.
    pub fn task_count(&self) -> usize {
        self.entries.len()
    }

    /// Get all task entries.
    pub fn entries(&self) -> &[TaskEntry] {
        &self.entries
    }

    /// Cool the kiln: run pattern detection across all completed tasks.
    ///
    /// This is where the beauty emerges. Just as a pottery kiln's crackle glaze
    /// forms during cooling, the patterns that crackle-runtime detects are only
    /// visible after the heat of execution has passed.
    ///
    /// Returns all detected patterns.
    pub fn cool(&mut self) -> Vec<CracklePattern> {
        self.cooled = true;
        let mut patterns = Vec::new();

        if self.entries.len() < self.profile.rate.min_tasks_for_detection() {
            return patterns;
        }

        let labels: Vec<String> = self.entries.iter().map(|e| e.label.clone()).collect();
        let metrics: Vec<Vec<(String, f64)>> = self.entries.iter().map(|e| e.all_metrics()).collect();

        // Run each task's cool() phase — set cooled timestamps
        let cooled_ts = Timestamp::now();
        for entry in &mut self.entries {
            entry.metadata.cooled_at = Some(cooled_ts);
        }

        if self.profile.detect_clustering {
            let p = ClusteringPattern::detect(
                &labels,
                &metrics,
                self.profile.rate.cluster_threshold(),
            );
            patterns.extend(p);
        }

        if self.profile.detect_phase_transitions {
            let p = PhaseTransitionPattern::detect(
                &labels,
                &metrics,
                self.profile.rate.phase_transition_sensitivity(),
            );
            patterns.extend(p);
        }

        if self.profile.detect_conservation {
            let p = ConservationPattern::detect(
                &labels,
                &metrics,
                self.profile.rate.conservation_tolerance(),
            );
            patterns.extend(p);
        }

        if self.profile.detect_correlations {
            let p = CorrelationPattern::detect(
                &labels,
                &metrics,
                self.profile.rate.correlation_threshold(),
            );
            patterns.extend(p);
        }

        // Sort by confidence descending
        patterns.sort_by(|a, b| b.confidence().partial_cmp(&a.confidence()).unwrap_or(std::cmp::Ordering::Equal));

        patterns
    }

    /// Check if the kiln has been cooled.
    pub fn is_cooled(&self) -> bool {
        self.cooled
    }

    /// Get the thermal profile.
    pub fn profile(&self) -> &ThermalProfile {
        &self.profile
    }

    /// Reset the kiln for a new firing cycle.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.cooled = false;
    }

    /// Compute the full mutual information matrix for all metric pairs.
    ///
    /// Returns a symmetric matrix where entry (i,j) is the mutual information
    /// between metric i and metric j. Captures non-linear dependencies that
    /// Pearson correlation misses.
    ///
    /// # Arguments
    ///
    /// * `bins` - Number of bins for discretization (typically 10)
    ///
    /// # Panics
    ///
    /// Panics if there are no metric names (empty kiln).
    pub fn mi_matrix(&self, bins: usize) -> Vec<Vec<f64>> {
        let metric_names = self.collect_metric_names();
        let n = metric_names.len();
        if n == 0 {
            return vec![];
        }

        // Extract values for each metric
        let metric_values: Vec<Vec<f64>> = metric_names
            .iter()
            .map(|name| {
                self.entries
                    .iter()
                    .filter_map(|e| {
                        e.all_metrics()
                            .iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, v)| *v)
                    })
                    .collect()
            })
            .collect();

        let mut matrix = vec![vec![0.0f64; n]; n];

        for i in 0..n {
            matrix[i][i] = entropy(&metric_values[i], bins);
            for j in (i + 1)..n {
                let mi = mutual_information(&metric_values[i], &metric_values[j], bins);
                matrix[i][j] = mi;
                matrix[j][i] = mi;
            }
        }

        matrix
    }

    /// Compute KL divergence between first-half and second-half metric distributions.
    ///
    /// Principled replacement for the old "phase transition" heuristic.
    /// Returns the KL divergence for each metric name.
    pub fn distribution_shift(&self, bins: usize) -> Vec<(String, f64)> {
        let metric_names = self.collect_metric_names();
        let n = self.entries.len();
        if n < 2 {
            return vec![];
        }

        let mid = n / 2;
        let mut results = Vec::new();

        for name in &metric_names {
            let first_half: Vec<f64> = self.entries[..mid]
                .iter()
                .filter_map(|e| {
                    e.all_metrics()
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, v)| *v)
                })
                .collect();

            let second_half: Vec<f64> = self.entries[mid..]
                .iter()
                .filter_map(|e| {
                    e.all_metrics()
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, v)| *v)
                })
                .collect();

            if !first_half.is_empty() && !second_half.is_empty() {
                let kl = kl_divergence(&second_half, &first_half, bins);
                results.push((name.clone(), kl));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Compute Jensen-Shannon divergence between first-half and second-half metric distributions.
    ///
    /// Symmetric version of KL divergence. Returns JSD for each metric name.
    pub fn jsd_shift(&self, bins: usize) -> Vec<(String, f64)> {
        let metric_names = self.collect_metric_names();
        let n = self.entries.len();
        if n < 2 {
            return vec![];
        }

        let mid = n / 2;
        let mut results = Vec::new();

        for name in &metric_names {
            let first_half: Vec<f64> = self.entries[..mid]
                .iter()
                .filter_map(|e| {
                    e.all_metrics()
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, v)| *v)
                })
                .collect();

            let second_half: Vec<f64> = self.entries[mid..]
                .iter()
                .filter_map(|e| {
                    e.all_metrics()
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, v)| *v)
                })
                .collect();

            if !first_half.is_empty() && !second_half.is_empty() {
                let js = jsd(&first_half, &second_half, bins);
                results.push((name.clone(), js));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Compute permutation entropy for each metric's time series.
    ///
    /// Captures temporal structure in metric values.
    pub fn permutation_entropies(&self, order: usize) -> Vec<(String, f64)> {
        let metric_names = self.collect_metric_names();

        metric_names
            .iter()
            .map(|name| {
                let values: Vec<f64> = self
                    .entries
                    .iter()
                    .filter_map(|e| {
                        e.all_metrics()
                            .iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, v)| *v)
                    })
                    .collect();
                (name.clone(), permutation_entropy(&values, order))
            })
            .collect()
    }

    /// Collect all unique metric names across all entries.
    fn collect_metric_names(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for entry in &self.entries {
            for (name, _) in entry.all_metrics() {
                names.insert(name.clone());
            }
        }
        names.into_iter().collect()
    }
}
