use crate::task::{CrackleTask, TaskMetadata, TaskOutput, Timestamp};
use crate::patterns::{
    ClusteringPattern, ConservationPattern, CorrelationPattern, CracklePattern, PhaseTransitionPattern,
};
use crate::profile::ThermalProfile;

/// A completed task entry stored in the kiln.
#[derive(Debug, Clone)]
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
/// struct MyTask { x: f64 }
/// impl CrackleTask for MyTask {
///     type Output = f64;
///     fn fire(&self) -> TaskOutput<Self::Output> {
///         TaskOutput::new(self.x, vec![("value".into(), self.x)])
///     }
/// }
///
/// let mut kiln = Kiln::new(ThermalProfile::default());
/// kiln.fire_task(MyTask { x: 1.0 });
/// kiln.fire_task(MyTask { x: 2.0 });
/// kiln.fire_task(MyTask { x: 3.0 });
///
/// let patterns = kiln.cool();
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

    /// Fire a single task and record its output.
    ///
    /// Returns the task's output value.
    ///
    /// # Panics
    ///
    /// Panics if called after `cool()`.
    pub fn fire_task<T: CrackleTask>(&self, task: T) -> TaskOutput<T::Output> {
        assert!(!self.cooled, "cannot fire tasks after cooling");

        task.fire()
    }

    /// Fire a task and record it in the kiln for later cooling.
    ///
    /// This stores the task's metrics internally so patterns can be detected
    /// during the cooling phase.
    pub fn fire_and_record<T: CrackleTask>(&mut self, task: T) -> TaskOutput<T::Output> {
        assert!(!self.cooled, "cannot fire tasks after cooling");

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
        output
    }

    /// Fire multiple tasks in sequence and record them all.
    pub fn fire_all<T: CrackleTask>(&mut self, tasks: Vec<T>) -> Vec<TaskOutput<T::Output>> {
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
}
