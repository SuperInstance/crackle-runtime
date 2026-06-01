use crate::task::{CrackleTask, TaskOutput};

/// A decorator that adds pattern-detection capability to any task.
///
/// In pottery, a glaze layer is applied to the surface of a vessel before firing.
/// The glaze doesn't change what the vessel *is* — it changes what it *reveals*.
/// During cooling, the glaze crackles, making visible patterns that the plain clay
/// would never show.
///
/// `GlazeLayer` wraps any task and enriches its metrics, making the underlying
/// patterns more visible to the kiln's pattern detectors.
///
/// # Example
///
/// ```
/// use crackle_runtime::{CrackleTask, GlazeLayer, TaskOutput};
///
/// struct SimpleTask { value: f64 }
/// impl CrackleTask for SimpleTask {
///     type Output = f64;
///     fn fire(&self) -> TaskOutput<Self::Output> {
///         TaskOutput::simple(self.value)
///     }
/// }
///
/// let task = SimpleTask { value: 42.0 };
/// let glazed = GlazeLayer::new(task)
///     .with_derived_metric("squared", |out| out.value * out.value)
///     .with_derived_metric("log", |out| out.value.ln());
///
/// let output = glazed.fire();
/// assert!(output.metrics.len() >= 2);
/// ```
pub struct GlazeLayer<T: CrackleTask> {
    inner: T,
    derived_metrics: Vec<DerivedMetric<T::Output>>,
    label_override: Option<String>,
}

type DerivedMetric<T> = (String, Box<dyn Fn(&TaskOutput<T>) -> f64>);

impl<T: CrackleTask> GlazeLayer<T> {
    /// Create a new glaze layer wrapping the given task.
    pub fn new(task: T) -> Self {
        GlazeLayer {
            inner: task,
            derived_metrics: Vec::new(),
            label_override: None,
        }
    }

    /// Add a derived metric computed from the task output.
    ///
    /// Derived metrics are computed after firing and added to the output's
    /// metric list, giving pattern detectors more signal to work with.
    pub fn with_derived_metric(
        mut self,
        name: impl Into<String>,
        compute: impl Fn(&TaskOutput<T::Output>) -> f64 + 'static,
    ) -> Self {
        self.derived_metrics
            .push((name.into(), Box::new(compute)));
        self
    }

    /// Override the task's label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label_override = Some(label.into());
        self
    }
}

impl<T: CrackleTask> CrackleTask for GlazeLayer<T> {
    type Output = T::Output;

    fn fire(&self) -> TaskOutput<Self::Output> {
        let mut output = self.inner.fire();
        for (name, compute) in &self.derived_metrics {
            let value = compute(&output);
            output.metrics.push((name.clone(), value));
        }
        output
    }

    fn cool(
        &self,
        output: &TaskOutput<Self::Output>,
        all_metrics: &[(String, Vec<(String, f64)>)],
    ) -> Vec<(String, f64)> {
        let mut results = self.inner.cool(output, all_metrics);
        // Add derived cooling metrics
        for (name, compute) in &self.derived_metrics {
            let value = compute(output);
            results.push((name.clone(), value));
        }
        results
    }

    fn label(&self) -> String {
        self.label_override
            .clone()
            .unwrap_or_else(|| self.inner.label())
    }
}

/// Builder for constructing glazed task batches.
///
/// Helps create multiple glazed variants of tasks with consistent metric derivation.
#[allow(dead_code)]
pub struct GlazeBatch<T: CrackleTask> {
    tasks: Vec<GlazeLayer<T>>,
}

impl<T: CrackleTask> GlazeBatch<T> {
    #[allow(dead_code)]
    /// Create a new empty batch.
    pub fn new() -> Self {
        GlazeBatch { tasks: Vec::new() }
    }

    /// Add a task to the batch with standard glaze metrics applied.
    #[allow(dead_code)]
    pub fn add(mut self, task: T) -> Self {
        self.tasks.push(GlazeLayer::new(task));
        self
    }

    #[allow(dead_code)]
    pub fn add_glazed(mut self, glazed: GlazeLayer<T>) -> Self {
        self.tasks.push(glazed);
        self
    }

    #[allow(dead_code)]
    pub fn tasks(&self) -> &[GlazeLayer<T>] {
        &self.tasks
    }

    #[allow(dead_code)]
    pub fn into_tasks(self) -> Vec<GlazeLayer<T>> {
        self.tasks
    }
}

impl<T: CrackleTask> Default for GlazeBatch<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NumTask { v: f64 }
    impl CrackleTask for NumTask {
        type Output = f64;
        fn fire(&self) -> TaskOutput<Self::Output> {
            TaskOutput::simple(self.v)
        }
    }

    #[test]
    fn glaze_adds_derived_metrics() {
        let glazed = GlazeLayer::new(NumTask { v: 4.0 })
            .with_derived_metric("squared", |o| o.value * o.value);
        let output = glazed.fire();
        assert_eq!(output.value, 4.0);
        let sq = output.metrics.iter().find(|(n, _)| n == "squared").unwrap();
        assert!((sq.1 - 16.0).abs() < 0.001);
    }

    #[test]
    fn glaze_label_override() {
        let glazed = GlazeLayer::new(NumTask { v: 1.0 }).with_label("special");
        assert_eq!(glazed.label(), "special");
    }

    #[test]
    fn glaze_batch_builder() {
        let batch = GlazeBatch::new()
            .add(NumTask { v: 1.0 })
            .add(NumTask { v: 2.0 });
        assert_eq!(batch.tasks().len(), 2);
    }
}
