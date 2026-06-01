# crackle-runtime 🏺

> *The crackle glaze forms in the cooling, not the firing. Beauty is not made in the heat. It arrives in the descent.*

A Rust task execution framework where tasks have a **firing phase** (hot execution) and a **cooling phase** (deferred pattern detection). During cooling, the runtime detects emergent patterns across completed tasks that weren't visible during execution.

[![crates.io](https://img.shields.io/crates/v/crackle-runtime.svg)](https://crates.io/crates/crackle-runtime)
[![docs.rs](https://docs.rs/crackle-runtime/badge.svg)](https://docs.rs/crackle-runtime)

## The Pottery Metaphor

In pottery, the most beautiful moment is not the firing — it's the cooling. When a kiln cools, the glaze and clay contract at different rates. The tension produces *craze lines*: fine, wandering, unrepeatable cracks that record the history of the transformation. You cannot design these cracks. You can only create the conditions for them and get out of the way.

**crackle-runtime** applies this insight to task execution:

| Pottery | crackle-runtime |
|---------|----------------|
| Firing the kiln | Executing tasks (`fire()`) |
| Glaze and clay | Task outputs and metrics |
| Cooling | Pattern detection across completed tasks (`cool()`) |
| Craze lines | Emergent patterns invisible during execution |
| Thermal profile | `ThermalProfile` — controls detection sensitivity |
| Glaze layer | `GlazeLayer` — decorator that enriches task metrics |

The key insight: **patterns that are invisible during execution become visible in retrospect**. Tasks that cluster together, conservation laws that hold across groups, unexpected correlations — these emerge only when you step back and look at the whole.

## Quick Start

```rust
use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput, GlazeLayer};

// 1. Define a task
struct TemperatureSensor { reading: f64, id: String }

impl CrackleTask for TemperatureSensor {
    type Output = f64;

    fn fire(&self) -> TaskOutput<Self::Output> {
        TaskOutput::new(
            self.reading,
            vec![
                ("temperature".into(), self.reading),
                ("is_hot".into(), if self.reading > 30.0 { 1.0 } else { 0.0 }),
            ],
        )
    }

    fn label(&self) -> String { self.id.clone() }
}

// 2. Build a kiln and fire tasks
let mut kiln = Kiln::new(ThermalProfile::fast_cooling());

kiln.fire_and_record(TemperatureSensor { reading: 22.5, id: "sensor-1".into() });
kiln.fire_and_record(TemperatureSensor { reading: 23.1, id: "sensor-2".into() });
kiln.fire_and_record(TemperatureSensor { reading: 45.2, id: "sensor-3".into() });

// 3. Cool the kiln — patterns emerge
let patterns = kiln.cool();

for pattern in &patterns {
    println!("[{}] {}", pattern.kind(), pattern.description());
}
// [clustering] 2 tasks clustered together in metric space
// [phase transition] metric 'temperature' shifted between first and second half
```

## Core Concepts

### `CrackleTask` — The Task Trait

Every task implements two phases:

- **`fire()`** — The hot phase. Execute the work, produce output with named metrics.
- **`cool()`** — Optional post-completion reflection. Examine all completed tasks.

```rust
pub trait CrackleTask {
    type Output;
    fn fire(&self) -> TaskOutput<Self::Output>;
    fn cool(&self, output: &TaskOutput<Self::Output>, all_metrics: &[(String, Vec<(String, f64)>)]) -> Vec<(String, f64)>;
    fn label(&self) -> String;
}
```

### `Kiln` — The Runtime

The kiln fires tasks and then cools them to detect patterns:

```rust
let mut kiln = Kiln::new(ThermalProfile::default());

// Fire individual tasks
kiln.fire_and_record(my_task);

// Or fire a batch
kiln.fire_all(tasks);

// Cool and detect patterns
let patterns = kiln.cool();

// Reset for another cycle
kiln.reset();
```

### `ThermalProfile` — Cooling Control

Controls the sensitivity and character of pattern detection:

| Profile | Behavior |
|---------|----------|
| `fast_cooling()` | Many fine patterns, low thresholds (like quenching) |
| `default()` | Balanced detection |
| `slow_cooling()` | Fewer, higher-confidence patterns |
| `no_detection()` | No pattern detection (benchmarking) |

```rust
let profile = ThermalProfile::default()
    .with_rate(CoolingRate::Fast)
    .without_clustering()       // disable specific detectors
    .without_correlations();
```

### `GlazeLayer` — Task Decorator

Enriches any task with derived metrics, making patterns more visible:

```rust
let glazed = GlazeLayer::new(my_task)
    .with_derived_metric("magnitude", |out| out.value.abs())
    .with_derived_metric("log_value", |out| out.value.ln());
```

Like a glaze layer in pottery — it doesn't change what the vessel *is*, it changes what it *reveals*.

## Pattern Detectors

crackle-runtime detects four types of emergent patterns:

### 1. Clustering (`ClusteringPattern`)

Tasks that cluster together in metric space. Like tasks that were fired near each other in the kiln and contracted similarly during cooling.

```rust
// These two sensors will cluster — their metrics are close
kiln.fire_and_record(Sensor { temp: 22.0 });
kiln.fire_and_record(Sensor { temp: 22.1 });
// This one is far away — different cluster
kiln.fire_and_record(Sensor { temp: 95.0 });
```

### 2. Phase Transitions (`PhaseTransitionPattern`)

Shifts in output distributions during execution. Like a glaze that changes character mid-cooling.

Detects when the first half of tasks produces significantly different metric values than the second half.

### 3. Conservation Laws (`ConservationPattern`)

Metrics that remain approximately constant across a group of tasks. Like a physical conservation law — energy, momentum, or crackle-runtime's equivalent.

Detects metrics with low variance relative to their mean.

### 4. Correlations (`CorrelationPattern`)

Unexpected correlations between different metrics across tasks. Like two glazes that crack in parallel despite being on opposite sides of the kiln.

Uses Pearson correlation to find metrics that move together.

## API Reference

### `TaskOutput<T>`

```rust
let output = TaskOutput::new(42.0, vec![("x".into(), 1.0)])
    .with_metric("y", 2.0);
```

### `CracklePattern`

```rust
for pattern in kiln.cool() {
    println!("Kind: {:?}", pattern.kind());        // Clustering, PhaseTransition, etc.
    println!("Description: {}", pattern.description());
    println!("Confidence: {:.2}", pattern.confidence());
    println!("Tasks: {:?}", pattern.involved_tasks());
}
```

### `CoolingRate`

```rust
let rate = CoolingRate::Fast;
let threshold = rate.cluster_threshold();        // 1.5
let sensitivity = rate.phase_transition_sensitivity();  // 0.3
```

## Design Principles

1. **Beauty in the cooling.** The most interesting patterns emerge *after* execution, not during it.
2. **You cannot design the cracks.** Pattern detection is observational, not prescriptive. Create conditions, then get out of the way.
3. **The craze line is a record.** Each detected pattern records something that actually happened — a genuine emergent property of the task group.
4. **Resistance is the vessel.** Constraints (thermal profile, detection thresholds) shape the patterns. Without constraints, there's no form.
5. **Forgetting is the architecture.** The kiln doesn't remember why it fired. It only observes what emerged.

## Inspiration

This crate was inspired by ["The Potter Who Cracked the Glaze"](https://github.com/SuperInstance/ai-writings/tree/main/ford-creative-wheel) from the *Ford Creative Wheel* collection, which explores the idea that the most beautiful moment in pottery is not the firing but the cooling — the long, slow descent where craze lines form as the system returns to ordinary temperature.

> *The firing was the transformation. The crack was the autobiography.*

## License

MIT
