# crackle-runtime

**Detect emergent patterns across task outputs in Rust.** Fire tasks, cool the kiln, discover clustering, correlations, phase transitions, and conservation laws you didn't design.

[![crates.io](https://img.shields.io/crates/v/crackle-runtime.svg)](https://crates.io/crates/crackle-runtime)
[![docs.rs](https://docs.rs/crackle-runtime/badge.svg)](https://docs.rs/crackle-runtime)

```toml
cargo add crackle-runtime
```

## 30-Second Example

```rust
use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput};

struct Sensor { reading: f64, id: String }

impl CrackleTask for Sensor {
    type Output = f64;
    fn fire(&self) -> TaskOutput<Self::Output> {
        TaskOutput::new(self.reading, vec![
            ("value".into(), self.reading),
            ("is_anomaly".into(), if self.reading > 40.0 { 1.0 } else { 0.0 }),
        ])
    }
    fn label(&self) -> String { self.id.clone() }
}

let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
kiln.fire_and_record(Sensor { reading: 22.5, id: "s1".into() });
kiln.fire_and_record(Sensor { reading: 23.1, id: "s2".into() });
kiln.fire_and_record(Sensor { reading: 45.2, id: "s3".into() }); // outlier

let patterns = kiln.cool();
for p in &patterns {
    println!("[{}] {} (confidence: {:.2})", p.kind(), p.description(), p.confidence());
}
// [clustering] 2 tasks clustered together in metric space (avg distance: 0.600)
// [phase transition] metric 'value' shifted by 87.5% between first and second half of tasks
```

## What It Does

crackle-runtime runs tasks that produce **named metrics**, then analyzes all metrics together to find patterns that were invisible during individual execution:

| Pattern | What it detects |
|---------|----------------|
| **Clustering** | Tasks whose metrics are close in Euclidean space |
| **Phase Transition** | Metrics that shifted significantly between the first and second half of tasks |
| **Conservation** | Metrics that stay near-constant across a group (low coefficient of variation) |
| **Correlation** | Pairs of metrics that move together (Pearson correlation) |

All detection runs after execution completes — no overhead during your hot path.

## Real-World Use Cases

- **CI/CD anomaly detection** — Feed build durations and test counts as tasks, detect when build times cluster oddly or shift unexpectedly
- **Load test analysis** — Run hundreds of request tasks, let the runtime find response-time clustering and throughput correlations
- **API response monitoring** — Track latency, status codes, payload sizes across endpoints; detect emerging patterns before they become incidents
- **Data pipeline quality** — Process records as tasks, detect conservation laws (row counts in = row counts out) and unexpected metric correlations

## API Reference

### `CrackleTask` — The Task Trait

Implement `fire()` to produce output with named metrics:

```rust
pub trait CrackleTask {
    type Output;
    fn fire(&self) -> TaskOutput<Self::Output>;
    fn cool(&self, output: &TaskOutput<Self::Output>, all_metrics: &[(String, Vec<(String, f64)>)]) -> Vec<(String, f64)>;
    fn label(&self) -> String;
}
```

- `fire()` — Execute and return `TaskOutput` with your value and metrics
- `cool()` — Optional post-completion hook (default is no-op)
- `label()` — Human-readable task name for pattern reports

### `Kiln` — The Runtime

```rust
let mut kiln = Kiln::new(ThermalProfile::default());
kiln.fire_and_record(task);          // Execute and store metrics
kiln.fire_all(tasks);                // Batch execute
let patterns = kiln.cool();          // Run pattern detection
kiln.reset();                        // Clear for next cycle
```

### `TaskOutput<T>`

```rust
let output = TaskOutput::new(42.0, vec![("x".into(), 1.0)])
    .with_metric("y", 2.0);
```

### `CracklePattern`

```rust
for p in kiln.cool() {
    p.kind();           // PatternKind::Clustering | PhaseTransition | Conservation | Correlation
    p.description();    // Human-readable explanation
    p.confidence();     // 0.0–1.0
    p.involved_tasks(); // Which task labels are involved
    p.metrics();        // Additional detected metrics
}
```

### `ThermalProfile` — Sensitivity Control

| Profile | Behavior |
|---------|----------|
| `fast_cooling()` | Low thresholds, more patterns detected (good for exploration) |
| `default()` | Balanced |
| `slow_cooling()` | High thresholds, only strong patterns (good for production) |
| `no_detection()` | Skip detection entirely (benchmarking) |

```rust
let profile = ThermalProfile::default()
    .with_rate(CoolingRate::Fast)
    .without_clustering()      // disable specific detectors
    .without_correlations();
```

### `GlazeLayer` — Task Decorator

Enriches any task with derived metrics for better pattern detection:

```rust
let glazed = GlazeLayer::new(my_task)
    .with_derived_metric("magnitude", |out| out.value.abs())
    .with_derived_metric("log_value", |out| out.value.ln());
```

## How It Works

1. **Fire phase** — Each task executes independently, producing a value and a set of named metrics
2. **Record** — The kiln stores all task outputs and their metrics
3. **Cool phase** — After all tasks complete, the runtime runs four detectors across the full metric set:
   - Clustering: pairwise Euclidean distance with configurable threshold
   - Phase transitions: first-half vs second-half mean comparison
   - Conservation: coefficient of variation below tolerance
   - Correlation: Pearson correlation between metric pairs
4. **Results** — Patterns sorted by confidence, with involved tasks and explanatory descriptions

No background threads. No async runtime. Pure synchronous detection that runs when you call `cool()`.

## Philosophy 🏺

The name comes from pottery: *crackle glaze* forms during cooling, not firing. The glaze and clay contract at different rates, producing fine cracks you can't design — only create conditions for. Similarly, the patterns crackle-runtime detects emerge from the aggregate behavior of your tasks, visible only in retrospect.

Inspired by ["The Potter Who Cracked the Glaze"](https://github.com/SuperInstance/ai-writings/tree/main/ford-creative-wheel) from the *Ford Creative Wheel* collection.

> *The firing was the transformation. The crack was the autobiography.*

## License

MIT
