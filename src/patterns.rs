use std::fmt;

/// The kind of pattern detected during cooling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatternKind {
    /// Tasks that cluster together in metric space.
    Clustering,
    /// A shift in output distribution during cooling.
    PhaseTransition,
    /// A conservation law that holds across a group of tasks.
    Conservation,
    /// An unexpected correlation between seemingly unrelated tasks.
    Correlation,
}

impl fmt::Display for PatternKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternKind::Clustering => write!(f, "clustering"),
            PatternKind::PhaseTransition => write!(f, "phase transition"),
            PatternKind::Conservation => write!(f, "conservation law"),
            PatternKind::Correlation => write!(f, "correlation"),
        }
    }
}

/// A pattern detected during the cooling phase.
///
/// Like a craze line in pottery glaze, each pattern is a record of something
/// that wasn't designed — it emerged from the interaction of many tasks
/// as the system cooled.
#[derive(Debug, Clone)]
pub struct CracklePattern {
    kind: PatternKind,
    description: String,
    involved_tasks: Vec<String>,
    confidence: f64,
    metrics: Vec<(String, f64)>,
}

impl CracklePattern {
    /// Create a new detected pattern.
    pub fn new(
        kind: PatternKind,
        description: impl Into<String>,
        involved_tasks: Vec<String>,
        confidence: f64,
    ) -> Self {
        CracklePattern {
            kind,
            description: description.into(),
            involved_tasks,
            confidence: confidence.clamp(0.0, 1.0),
            metrics: vec![],
        }
    }

    /// The kind of pattern detected.
    pub fn kind(&self) -> &PatternKind {
        &self.kind
    }

    /// Human-readable description of the pattern.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Labels of tasks involved in this pattern.
    pub fn involved_tasks(&self) -> &[String] {
        &self.involved_tasks
    }

    /// Confidence score (0.0 to 1.0).
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Additional metrics associated with this pattern.
    pub fn metrics(&self) -> &[(String, f64)] {
        &self.metrics
    }

    /// Add a metric to this pattern.
    pub fn with_metric(mut self, name: impl Into<String>, value: f64) -> Self {
        self.metrics.push((name.into(), value));
        self
    }

    /// Create a pattern with additional metrics.
    pub fn with_metrics(mut self, metrics: Vec<(String, f64)>) -> Self {
        self.metrics = metrics;
        self
    }
}

/// Detector for clustering patterns — tasks that complete near each other in metric space.
#[derive(Debug, Clone)]
pub struct ClusteringPattern;

impl ClusteringPattern {
    /// Detect clusters among task metrics.
    ///
    /// Returns groups of task labels that cluster together based on metric proximity.
    pub fn detect(
        task_labels: &[String],
        task_metrics: &[Vec<(String, f64)>],
        threshold: f64,
    ) -> Vec<CracklePattern> {
        if task_labels.len() < 2 {
            return vec![];
        }

        let mut patterns = Vec::new();
        let n = task_labels.len();
        let mut visited = vec![false; n];

        for i in 0..n {
            if visited[i] {
                continue;
            }
            let mut cluster = vec![i];
            visited[i] = true;

            for j in (i + 1)..n {
                if visited[j] {
                    continue;
                }
                if Self::metric_distance(&task_metrics[i], &task_metrics[j]) < threshold {
                    cluster.push(j);
                    visited[j] = true;
                }
            }

            if cluster.len() > 1 {
                let labels: Vec<String> = cluster.iter().map(|&idx| task_labels[idx].clone()).collect();
                let avg_dist = Self::avg_cluster_distance(&cluster, task_metrics);
                patterns.push(
                    CracklePattern::new(
                        PatternKind::Clustering,
                        format!(
                            "{} tasks clustered together in metric space (avg distance: {:.3})",
                            labels.len(),
                            avg_dist
                        ),
                        labels,
                        1.0 - (avg_dist / threshold).min(1.0),
                    )
                    .with_metric("avg_distance", avg_dist)
                    .with_metric("cluster_size", cluster.len() as f64),
                );
            }
        }

        patterns
    }

    /// Compute Euclidean-like distance between two metric sets.
    pub fn metric_distance(a: &[(String, f64)], b: &[(String, f64)]) -> f64 {
        let mut sum_sq = 0.0;
        let mut matched = 0;

        for (name_a, val_a) in a {
            if let Some((_, val_b)) = b.iter().find(|(name_b, _)| name_b == name_a) {
                sum_sq += (val_a - val_b).powi(2);
                matched += 1;
            }
        }

        if matched == 0 {
            f64::MAX
        } else {
            sum_sq.sqrt()
        }
    }

    fn avg_cluster_distance(indices: &[usize], metrics: &[Vec<(String, f64)>]) -> f64 {
        if indices.len() < 2 {
            return 0.0;
        }
        let mut total = 0.0;
        let mut count = 0;
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                total += Self::metric_distance(&metrics[indices[i]], &metrics[indices[j]]);
                count += 1;
            }
        }
        total / count as f64
    }
}

/// Detector for phase transitions — shifts in output distributions.
#[derive(Debug, Clone)]
pub struct PhaseTransitionPattern;

impl PhaseTransitionPattern {
    /// Detect phase transitions by looking for significant shifts in metric values.
    ///
    /// Works by comparing the first half of tasks to the second half.
    pub fn detect(
        task_labels: &[String],
        task_metrics: &[Vec<(String, f64)>],
        sensitivity: f64,
    ) -> Vec<CracklePattern> {
        let n = task_labels.len();
        if n < 2 {
            return vec![];
        }

        let mut patterns = Vec::new();
        let all_metric_names = Self::collect_metric_names(task_metrics);

        for metric_name in &all_metric_names {
            let values: Vec<(usize, f64)> = task_metrics
                .iter()
                .enumerate()
                .filter_map(|(i, m)| {
                    m.iter()
                        .find(|(n, _)| n == metric_name)
                        .map(|(_, v)| (i, *v))
                })
                .collect();

            if values.len() < 2 {
                continue;
            }

            let mid = values.len() / 2;
            let first_half_avg = values[..mid].iter().map(|(_, v)| v).sum::<f64>() / mid as f64;
            let second_half_avg =
                values[mid..].iter().map(|(_, v)| v).sum::<f64>() / (values.len() - mid) as f64;

            let global_avg = values.iter().map(|(_, v)| v).sum::<f64>() / values.len() as f64;
            if global_avg.abs() < f64::EPSILON {
                continue;
            }

            let shift = (second_half_avg - first_half_avg).abs() / global_avg.abs();
            if shift > sensitivity {
                let involved: Vec<String> = values
                    .iter()
                    .map(|(idx, _)| task_labels[*idx].clone())
                    .collect();
                patterns.push(
                    CracklePattern::new(
                        PatternKind::PhaseTransition,
                        format!(
                            "metric '{}' shifted by {:.1}% between first and second half of tasks",
                            metric_name,
                            shift * 100.0
                        ),
                        involved,
                        (shift / sensitivity).min(1.0),
                    )
                    .with_metric("metric_name_hash", metric_name.len() as f64)
                    .with_metric("shift_magnitude", shift)
                    .with_metric("first_half_avg", first_half_avg)
                    .with_metric("second_half_avg", second_half_avg),
                );
            }
        }

        patterns
    }

    fn collect_metric_names(metrics: &[Vec<(String, f64)>]) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for m in metrics {
            for (name, _) in m {
                names.insert(name.clone());
            }
        }
        names.into_iter().collect()
    }
}

/// Detector for conservation laws — metrics whose sum stays near-constant across tasks.
#[derive(Debug, Clone)]
pub struct ConservationPattern;

impl ConservationPattern {
    /// Detect conservation laws: metrics that sum to approximately the same value across task groups.
    pub fn detect(
        task_labels: &[String],
        task_metrics: &[Vec<(String, f64)>],
        tolerance: f64,
    ) -> Vec<CracklePattern> {
        let n = task_labels.len();
        if n < 2 {
            return vec![];
        }

        let mut patterns = Vec::new();
        let all_metric_names = PhaseTransitionPattern::collect_metric_names(task_metrics);

        for metric_name in &all_metric_names {
            let values: Vec<(usize, f64)> = task_metrics
                .iter()
                .enumerate()
                .filter_map(|(i, m)| {
                    m.iter()
                        .find(|(n, _)| n == metric_name)
                        .map(|(_, v)| (i, *v))
                })
                .collect();

            if values.len() < 2 {
                continue;
            }

            let total: f64 = values.iter().map(|(_, v)| v).sum();
            let avg = total / values.len() as f64;

            // Check if values hover around a constant (low variance = conservation)
            let variance =
                values.iter().map(|(_, v)| (v - avg).powi(2)).sum::<f64>() / values.len() as f64;
            let std_dev = variance.sqrt();

            if avg.abs() > f64::EPSILON && std_dev / avg.abs() < tolerance {
                let involved: Vec<String> = values
                    .iter()
                    .map(|(idx, _)| task_labels[*idx].clone())
                    .collect();
                patterns.push(
                    CracklePattern::new(
                        PatternKind::Conservation,
                        format!(
                            "metric '{}' is conserved across {} tasks (sum: {:.3}, std_dev: {:.3})",
                            metric_name,
                            involved.len(),
                            total,
                            std_dev
                        ),
                        involved,
                        1.0 - (std_dev / avg.abs()).min(1.0),
                    )
                    .with_metric("total", total)
                    .with_metric("std_dev", std_dev)
                    .with_metric("coefficient_of_variation", std_dev / avg.abs()),
                );
            }
        }

        patterns
    }
}

/// Detector for unexpected correlations between tasks.
#[derive(Debug, Clone)]
pub struct CorrelationPattern;

impl CorrelationPattern {
    /// Detect correlations between different metrics across tasks.
    pub fn detect(
        task_labels: &[String],
        task_metrics: &[Vec<(String, f64)>],
        threshold: f64,
    ) -> Vec<CracklePattern> {
        let n = task_labels.len();
        if n < 3 {
            return vec![];
        }

        let metric_names = PhaseTransitionPattern::collect_metric_names(task_metrics);
        if metric_names.len() < 2 {
            return vec![];
        }

        let mut patterns = Vec::new();

        for i in 0..metric_names.len() {
            for j in (i + 1)..metric_names.len() {
                let name_a = &metric_names[i];
                let name_b = &metric_names[j];

                let pairs: Vec<(f64, f64)> = task_metrics
                    .iter()
                    .filter_map(|m| {
                        let a = m.iter().find(|(n, _)| n == name_a).map(|(_, v)| *v);
                        let b = m.iter().find(|(n, _)| n == name_b).map(|(_, v)| *v);
                        match (a, b) {
                            (Some(a), Some(b)) => Some((a, b)),
                            _ => None,
                        }
                    })
                    .collect();

                if pairs.len() < 3 {
                    continue;
                }

                let corr = Self::pearson_correlation(&pairs);
                if corr.abs() >= threshold {
                    let involved: Vec<String> = task_labels
                        .iter()
                        .take(pairs.len())
                        .cloned()
                        .collect();
                    patterns.push(
                        CracklePattern::new(
                            PatternKind::Correlation,
                            format!(
                                "strong {} correlation between '{}' and '{}' (r = {:.3})",
                                if corr > 0.0 { "positive" } else { "negative" },
                                name_a,
                                name_b,
                                corr
                            ),
                            involved,
                            corr.abs(),
                        )
                        .with_metric("correlation", corr)
                        .with_metric("metric_a_len", name_a.len() as f64)
                        .with_metric("metric_b_len", name_b.len() as f64),
                    );
                }
            }
        }

        patterns
    }

    /// Compute Pearson correlation coefficient.
    pub fn pearson_correlation(pairs: &[(f64, f64)]) -> f64 {
        let n = pairs.len() as f64;
        if n < 2.0 {
            return 0.0;
        }

        let sum_x: f64 = pairs.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = pairs.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = pairs.iter().map(|(x, y)| x * y).sum();
        let sum_x2: f64 = pairs.iter().map(|(x, _)| x * x).sum();
        let sum_y2: f64 = pairs.iter().map(|(_, y)| y * y).sum();

        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

        if denominator.abs() < f64::EPSILON {
            0.0
        } else {
            numerator / denominator
        }
    }
}
