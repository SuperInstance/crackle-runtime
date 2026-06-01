#![deny(unsafe_code)]
//! # crackle-runtime
//!
//! *The crackle glaze forms in the cooling, not the firing.*
//!
//! A task execution framework where tasks have a **firing phase** (hot execution)
//! and a **cooling phase** (deferred pattern detection). During cooling, the runtime
//! detects emergent patterns across completed tasks that weren't visible during execution.
//!
//! ## Core Metaphor
//!
//! In pottery, the most beautiful moment is not the firing — it's the cooling. The glaze
//! and clay contract at different rates, producing craze lines: unique, unrepeatable
//! patterns that record the history of the transformation. You cannot design these cracks.
//! You can only create the conditions for them and get out of the way.
//!
//! Similarly, `crackle-runtime` executes tasks (firing) and then observes what patterns
//! emerge across completed tasks during a cooling phase — patterns that were invisible
//! during the heat of execution.
//!
//! ## Quick Start
//!
//! ```
//! use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, GlazeLayer, TaskOutput};
//! use std::time::Duration;
//!
//! /// A simple task that produces a number
//! struct NumberTask { value: f64 }
//!
//! impl CrackleTask for NumberTask {
//!     type Output = f64;
//!
//!     fn fire(&self) -> TaskOutput<Self::Output> {
//!         TaskOutput::new(self.value, vec![("magnitude".into(), self.value.abs())])
//!     }
//! }
//!
//! // Build a kiln with fast cooling (more cracks, more patterns)
//! let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
//!
//! // Fire tasks
//! kiln.fire_task(NumberTask { value: 3.14 }).unwrap();
//! kiln.fire_task(NumberTask { value: 2.71 }).unwrap();
//!
//! // Let the kiln cool and detect patterns
//! let patterns = kiln.cool();
//!
//! // Inspect what emerged
//! for pattern in &patterns {
//!     println!("{:?}: {}", pattern.kind(), pattern.description());
//! }
//! ```

mod error;
mod glaze;
mod information;
mod kiln;
mod patterns;
mod profile;
mod task;

pub use error::{CrackleError, Result};
pub use glaze::GlazeLayer;
pub use kiln::{Kiln, TaskEntry};
pub use patterns::{
    ClusteringPattern, ConservationPattern, CorrelationPattern, CracklePattern, PatternKind,
    PhaseTransitionPattern,
};
pub use profile::{CoolingRate, ThermalProfile};
pub use task::{CrackleTask, TaskMetadata, TaskOutput, Timestamp};
pub use information::{
    entropy, joint_entropy, jsd, kl_divergence, mutual_information, permutation_entropy,
    transfer_entropy,
};

#[cfg(test)]
mod tests;
