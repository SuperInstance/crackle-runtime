# TEST-DIARY.md — Honest review of `crackle-runtime` v0.2.0

**Tester:** Lin (distributed systems engineer)
**Date:** 2026-06-01
**Repo:** https://github.com/SuperInstance/crackle-runtime
**Crate:** https://crates.io/crates/crackle-runtime
**Test setup:** WSL2, Rust 1.85+, Intel Core Ultra 7

---

## 1. First Impression — README

**Verdict:** Strong. Above-average for a small Rust crate.

The README leads with a clear 30-second example that actually compiles and works (I verified). The pottery metaphor is consistent throughout — "firing" tasks, "cooling" the kiln, "crackle glaze" patterns. It's memorable and gives the library character without being pretentious.

The table of pattern types (clustering, phase transition, conservation, correlation) is exactly what I need as a potential user: one glance tells me what this crate *does*. The real-world use cases at the bottom (CI/CD, load testing, API monitoring) are grounded and realistic — not the usual "AI-powered" vaporware claims.

**What's missing:**
- No performance benchmarks anywhere. If I'm processing 1000 events, what's the overhead? How does the kiln scale with task count? The README should have a ballpark.
- The "30-Second Example" is actually ~35 lines with the struct impl — close enough, but I'd suggest trimming for impact.
- The `information` module (entropy, KL divergence, mutual information) isn't mentioned in the README at all. This is a real selling point — it's the most sophisticated part of the crate!

## 2. Build — cargo test & clippy

**All 135 tests pass.** 0 failures. 3 doctests also pass.

Clippy emits zero errors or warnings against the library code. However, two warnings in the test file:

1. `unused_must_use` on `kiln.fire_and_record(...)` in a test — the Result is dropped without `let _ =`. Minor, but visible.
2. `patterns.len() >= 0` which is trivially true for `usize`.

**Clean build time:** ~1.07s for test profile. Rust edition 2021, no heavyweight dependencies (serde is optional).

Dependency footprint is minimal — only `serde` + `serde_json` as optional. No tokio, no async. This is deliberate and good.

## 3. Architecture

### Layout (7 source files + examples)

```
src/
  lib.rs           — re-exports, module declarations, docstring
  task.rs          — CrackleTask trait, TaskOutput, TaskMetadata, Timestamp
  kiln.rs          — Kiln runtime (the core), TaskEntry
  patterns.rs      — CracklePattern + 4 detectors
  profile.rs       — ThermalProfile, CoolingRate
  error.rs         — CrackleError, Result type
  glaze.rs         — GlazeLayer decorator, GlazeBatch builder
  information.rs   — entropy, MI, KL, JSD, transfer entropy, permutation entropy
```

### Event processing model

The lifecycle is: **Fire → Record → Cool**.

- **Fire phase:** Tasks execute independently via `CrackleTask::fire()`, producing a `TaskOutput<T>` with a primary value and named `Vec<(String, f64)>` metrics. Synchronous, no async or background threads.
- **Record phase:** The `Kiln` stores all task outputs and their metric vectors. Nothing clever here — just a `Vec<TaskEntry>`.
- **Cool phase:** `kiln.cool()` runs all four detectors across the stored metrics. Results sorted by descending confidence. After cool, the kiln is sealed (`self.cooled = true`).

### Pattern detectors

| Detector | Algorithm | Weaknesses |
|----------|-----------|------------|
| **Clustering** | Greedy pairwise distance (Euclidean on shared metric names) with threshold. Visits each unvisited task, forms a cluster with any task within threshold distance. | O(n²) worst-case. Greedy means cluster membership depends on iteration order. Only matches *shared* metric names — tasks with disjoint metric sets get infinite distance. |
| **Phase transition** | Split tasks into first-half/second-half, compare mean values via relative shift: `|μ₂ - μ₁| / |global_μ|`. Thresholds from CoolingRate. | Only detects mean shifts — not variance shifts, distribution shape changes, or other phenomena. The `Kiln::distribution_shift()` and `Kiln::jsd_shift()` methods are superior but not used by the detector. This is an acknowledged gap (the `information.rs` module has the right tools, but `PhaseTransitionPattern` uses the old heuristic). |
| **Conservation** | Checks coefficient of variation (`σ/μ`) below a tolerance. Catches near-constant metrics. | Confuses "constant by design" (e.g., `records_in = 1000`) with "interesting conservation" (e.g., energy balancing). Tends to generate many false positives on numeric IDs and batch counters. |
| **Correlation** | Pearson's r on metric pairs with threshold. | Pearson only captures *linear* correlation. The `information.rs` module provides mutual information which handles non-linear dependencies, but `CorrelationPattern` doesn't use it. Also: `involved_tasks` includes *all* tasks, not just the correlated ones. |

### Information-theoretic module (`information.rs`)

This is the most impressive part. Full implementations of:
- Shannon entropy
- Joint entropy
- Mutual information (with discretization via equal-width bins)
- KL divergence (returns ∞ for disjoint support — correct behavior)
- Jensen-Shannon divergence (symmetric, always finite)
- Transfer entropy (directional information flow, lag-aware)
- Permutation entropy (temporal structure via ordinal patterns)
- Helper: histogram, histogram_2d, entropy_3d, factorial, ordinal pattern index

The tests are thorough — causal direction tests for TE, non-linearity detection for MI, symmetry checks for JSD. This module alone justifies the crate's existence as a research tool.

**However:** The `Kiln` methods `mi_matrix()`, `distribution_shift()`, `jsd_shift()`, and `permutation_entropies()` are powerful but feel bolted on. They're not integrated into the `cool()` pipeline. The phase transition detector still uses the old mean-shift heuristic instead of KL/JSD. This means the cool-phase patterns are weaker than what the kiln is *capable of*.

### GlazeLayer

A task decorator that adds derived metrics after firing. Clean design — wraps any `CrackleTask`, computes closure-based metrics post-hoc. Useful for enriching signal before pattern detection. The `GlazeBatch` builder is marked `#[allow(dead_code)]` — it compiles but isn't used anywhere.

### Surface area: `metrics.len()` as a hashed feature

The phase transition and correlation detectors include metrics like `metric_name_hash` (actually just `name.len() as f64`) and `metric_a_len`, `metric_b_len`. This caught my eye — it looks like debugging artifacts or shape metadata leaking into pattern output. These aren't meaningful metrics for a consumer.

## 4. Real Test — Event Stream Processor

I built a custom test simulating an event stream processor: 1000 events across 10 batches of 100, with throughput, latency, memory, error rate, records_in/records_out, CPU temp, consumer lag, and GC pause metrics. Three phases:

1. **Healthy system** — low variance, ~10ms latency baseline
2. **Degrading system** — latency ramps 10ms → 85ms, error rates climb, consumer lag grows
3. **Conservation violation** — records_in=1000, records_out drops from 995 to 700

### What it found

**Phase 1 (Healthy):** 39 patterns detected. Most are correct:
- `records_in` properly flagged as conserved (sum=400000, σ=0)
- `processing_time_ms` and `latency_p99_ms` correctly correlated (r=1.000)
- `throughput` anti-correlated with `processing_time_ms` (r=-0.998)

False positives: `event_id` flagged as "conserved" (sum=1019800, σ=1118) — obviously meaningless. `batch_id` also conserved. The conservation detector is too aggressive on monotonically increasing values with small noise.

**Phase 2 (Degrading):** 69 patterns. The phase transition detector correctly flags *everything* that shifts:
- throughput shifted 109.2% (correct — it drops as latency rises)
- error_rate shifted 102.7% (correct)
- gc_pause_ms shifted 87.0% (correct)
- consumer_lag shifted 107.9% (correct)
- event_id shifted 30.5% (false positive! event_id is monotonic by design)

The conservation detector again flags `records_in` — correct but trivial.

JSD shifts catch distribution shape changes that the phase transition detector misses. `throughput` JSD=0.1765 tells me the *distribution shape* changed, not just the mean. This is genuinely useful.

Permutation entropy: `processing_time_ms` PE=0.6543, `error_rate` PE=0.4478. These measure temporal complexity. Low PE on `consumer_lag` (0.0419) and `gc_pause_ms` (0.0419) means those metrics have simple temporal structure (linear increase) — useful signal for a human reading the tea leaves.

**Phase 3 (Conservation test):** The MI matrix confirms everything is perfectly correlated — which is expected since the simulation is deterministic with few datapoints (6 tasks). MI=1.0 bits across the board. The conservation detector correctly flags `records_in` (perfectly conserved) but also says `cpu_temp_c` is conserved (σ=7.5, which isn't small for a metric ranging 65–80). 

### Performance

1000 events, 3 separate kiln cycles including all four detectors + distribution shift + JSD + PE + MI matrix: **0.27 seconds total**. That's ~3700 events/s including analysis overhead. In a real streaming application where analysis happens less frequently (every N batches), this is negligible overhead.

## 5. Comparison to tokio + custom conservation

Would I use this instead of tokio + custom logic?

**For an actual production event stream:** No. Here's why.

The crate calls itself a "runtime" but it's purely synchronous. You don't get backpressure, stream processing, async pipelines, or any of the infrastructure you'd need for real event processing. The "runtime" is just the lifecycle metaphor (fire→cool). It's a pattern detection toolkit, not a data processing framework.

**For pattern detection specifically:** Maybe yes, with reservations.

What you'd build with tokio + custom logic:
- A loop that reads from Kafka/event bus
- Accumulates metrics in a Vec<HashMap<String, f64>>
- After N events, computes correlations, clusters, etc.

crackle-runtime gives you that last step for free. The `information.rs` module is genuinely hard to implement correctly (KL divergence edge cases, proper discretization, transfer entropy). If I needed distribution shift detection or mutual information, I'd reach for this crate.

**What I'd still build custom:**
- The pipeline (source → transform → analyze → alert). crackle-runtime only does the "analyze" step.
- Alerting/threshold management. The crate returns patterns but doesn't tell you what to do about them.
- Time-series-specific analysis. The crate treats tasks as unordered — time is only captured in `fired_at`/`cooled_at` timestamps, not used in analysis.

**What I'd use this for:**
- Offline batch analysis of event data. Dump processed metrics into the kiln, call `cool()`, get a pattern report.
- Research/exploration — trying out conservation law detection or transfer entropy on a new dataset.
- CI/CD quality gates — run after a test suite, detect anomalies in build metrics.

## 6. Score: ★★★☆☆ (3/5)

### What's genuinely good

1. **Clean, consistent API.** The kiln metaphor carries through every module. `fire()`, `cool()`, `Kiln`, `ThermalProfile`, `GlazeLayer` — it's internally consistent and easy to learn.
2. **Excellent information theory module.** Proper KL divergence (with infinite handling), JSD, transfer entropy, permutation entropy. This is research-grade and well-tested.
3. **Zero async, zero dependencies.** No tokio, no thread pools, no runtime overhead. Fits in a small crate with `serde` as optional.
4. **135 passing tests.** Good coverage of edge cases (empty metrics, single task, negative values, NaN-adjacent states).
5. **The `GlazeLayer` decorator** is a genuinely good pattern — pun intended. Lets you enrich metrics without modifying task code.

### What needs work

1. **Phase transition detector is weaker than the tools it has.** The detector uses a naive mean-shift heuristic while `Kiln::jsd_shift()` and `Kiln::distribution_shift()` use proper information-theoretic measures (KL divergence, JSD). The old heuristic should either be replaced or the cool() pipeline should expose both.
2. **Conservation detector generates too many false positives.** It flags any near-constant metric including IDs, batch numbers, and monotonically increasing counters. These aren't "conservation laws" in an interesting sense. The detector needs to distinguish "trivially constant" from "interestingly conserved."
3. **`involved_tasks` in correlation patterns is misleading.** It includes *all* tasks, not just the tasks most responsible for the correlation. For 400 tasks, correlation between `error_rate` and `records_out` reports all 400 tasks as "involved" — which is not useful.
4. **No time-aware analysis.** The crate captures `fired_at` timestamps but never uses them. Correlation analysis is cross-sectional only. For event streams, time-delayed correlations and lagged detection are essential.
5. **Misleading metric features.** `metric_name_hash` (uses `name.len() as f64`) in phase transition reports is uninformative. These should be removed or renamed.
6. **`fire_task()` vs `fire_and_record()` API asymmetry.** `fire_task()` takes `&self` and doesn't record — it's essentially useless because you'd always use `fire_and_record()`. The error is called `KilnCooled` but `fire_task()` is `&self` so it can never check `self.cooled`... wait, it does. This is fine but confusing. `fire_task()` should probably be private or merged.
7. **No `Send + Sync` bounds.** The crate isn't thread-safe by default. For a "runtime" that processes tasks, this is a limitation — though the synchronous design makes it less critical.

### Would I actually use this?

**For a production system:** No — I'd use a proper event stream processor (Kafka Streams, Flink, or even just tokio) and pull in a statistics crate for the analysis layer. crackle-runtime doesn't give me the pipeline.

**For a research project / prototype / exploratory analysis:** Yes. The information theory module is the best I've seen in a Rust crate of this size. Transfer entropy, permutation entropy, and mutual information with proper discretization are things I'd otherwise have to implement myself.

**For CI/CD quality gates or build analysis:** Yes, with the caveat that I'd need to filter out the false positives from the conservation detector.

### Final word

crackle-runtime is a **pattern detection library** wearing a **runtime** hat. The naming is aspirational — it's not a runtime in the tokio/smol sense. But as a pattern detection toolkit with a beautiful API, solid information theory foundations, and zero dependency overhead, it's genuinely useful for anyone doing batch analysis of metric data.

The pottery metaphor gives it soul. The information theory gives it substance. The gap between the two is where the next version should focus.

---

*"The crackle glaze forms in the cooling, not the firing."*
*— The library's tagline, which is also its truest design insight.*
