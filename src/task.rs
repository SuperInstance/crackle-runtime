use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A timestamp for when a task was fired or cooled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Create a timestamp from milliseconds since epoch.
    pub fn from_millis(millis: u64) -> Self {
        Timestamp(millis)
    }

    /// Create a timestamp representing "now".
    pub fn now() -> Self {
        Timestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        )
    }

    /// Get the milliseconds since epoch.
    pub fn as_millis(&self) -> u64 {
        self.0
    }

    /// Duration elapsed since this timestamp.
    pub fn elapsed(&self) -> Duration {
        let now = Timestamp::now();
        Duration::from_millis(now.0.saturating_sub(self.0))
    }
}

/// Metadata about a task's execution.
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// When the task was fired.
    pub fired_at: Timestamp,
    /// When the task's cooling phase completed.
    pub cooled_at: Option<Timestamp>,
    /// How long the firing phase took.
    pub fire_duration: Duration,
    /// A user-defined label for this task.
    pub label: String,
}

impl TaskMetadata {
    /// Create new metadata with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        TaskMetadata {
            fired_at: Timestamp::now(),
            cooled_at: None,
            fire_duration: Duration::ZERO,
            label: label.into(),
        }
    }

    /// Create metadata with a specific fired-at timestamp.
    pub fn fired_at(label: impl Into<String>, ts: Timestamp) -> Self {
        TaskMetadata {
            fired_at: ts,
            cooled_at: None,
            fire_duration: Duration::ZERO,
            label: label.into(),
        }
    }
}

/// The output of firing a task, including named metrics for pattern detection.
#[derive(Debug, Clone)]
pub struct TaskOutput<T> {
    /// The primary output value.
    pub value: T,
    /// Named metrics extracted during firing, used by pattern detectors.
    pub metrics: Vec<(String, f64)>,
}

impl<T> TaskOutput<T> {
    /// Create a new task output with a value and metrics.
    pub fn new(value: T, metrics: Vec<(String, f64)>) -> Self {
        TaskOutput { value, metrics }
    }

    /// Create a task output with no metrics.
    pub fn simple(value: T) -> Self {
        TaskOutput {
            value,
            metrics: vec![],
        }
    }

    /// Add a named metric.
    pub fn with_metric(mut self, name: impl Into<String>, value: f64) -> Self {
        self.metrics.push((name.into(), value));
        self
    }
}

/// The core trait for tasks in the crackle runtime.
///
/// Every task has two phases:
/// - **Firing** (`fire`): Hot execution. Do the work. Produce output.
/// - **Cooling** (`cool`): Post-completion reflection. Examine what happened
///   across all completed tasks and contribute observations.
///
/// The crackle glaze forms in the cooling, not the firing. Beauty arrives in the descent.
pub trait CrackleTask {
    /// The type of value produced by firing.
    type Output;

    /// Execute the task (the hot phase).
    ///
    /// This is where the primary work happens. Return the output along with
    /// named metrics that pattern detectors can analyze during cooling.
    fn fire(&self) -> TaskOutput<Self::Output>;

    /// Reflect on the task during cooling (optional).
    ///
    /// Receive the output from firing and a snapshot of all completed task metrics.
    /// Return any additional observations or modified metrics.
    ///
    /// The default implementation does nothing — many tasks don't need to
    /// participate actively in cooling. The runtime detects patterns regardless.
    fn cool(
        &self,
        _output: &TaskOutput<Self::Output>,
        _all_metrics: &[(String, Vec<(String, f64)>)],
    ) -> Vec<(String, f64)> {
        vec![]
    }

    /// A human-readable label for this task (used in pattern descriptions).
    fn label(&self) -> String {
        "anonymous task".to_string()
    }
}
