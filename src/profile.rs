use std::time::Duration;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

/// How fast the kiln cools — controls pattern sensitivity.
///
/// Fast cooling produces many fine cracks (more patterns detected, lower threshold).
/// Slow cooling produces fewer, larger patterns (higher confidence).
///
/// Like pottery: the rate of cooling determines the character of the crackle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoolingRate {
    /// Fast cooling: many fine cracks, lower thresholds, more patterns.
    /// Like quenching hot pottery — dramatic, lots of crackle lines.
    Fast,
    /// Normal cooling: balanced pattern detection.
    #[default]
    Normal,
    /// Slow cooling: fewer but more significant patterns.
    /// Like letting the kiln cool naturally — fewer cracks, but the ones that form
    /// are large and meaningful.
    Slow,
}

impl CoolingRate {
    /// The clustering distance threshold for this cooling rate.
    pub fn cluster_threshold(&self) -> f64 {
        match self {
            CoolingRate::Fast => 1.5,
            CoolingRate::Normal => 2.5,
            CoolingRate::Slow => 4.0,
        }
    }

    /// The correlation threshold for this cooling rate.
    pub fn correlation_threshold(&self) -> f64 {
        match self {
            CoolingRate::Fast => 0.5,
            CoolingRate::Normal => 0.7,
            CoolingRate::Slow => 0.9,
        }
    }

    /// The minimum number of tasks needed before pattern detection activates.
    pub fn min_tasks_for_detection(&self) -> usize {
        match self {
            CoolingRate::Fast => 2,
            CoolingRate::Normal => 3,
            CoolingRate::Slow => 5,
        }
    }

    /// Phase transition sensitivity — how much shift triggers detection.
    pub fn phase_transition_sensitivity(&self) -> f64 {
        match self {
            CoolingRate::Fast => 0.3,
            CoolingRate::Normal => 0.5,
            CoolingRate::Slow => 0.8,
        }
    }

    /// The conservation law tolerance — how close to zero a sum must be.
    pub fn conservation_tolerance(&self) -> f64 {
        match self {
            CoolingRate::Fast => 0.5,
            CoolingRate::Normal => 0.3,
            CoolingRate::Slow => 0.1,
        }
    }
}

/// The thermal profile controls how the kiln cools.
///
/// Just as a potter controls the cooling rate to influence crackle patterns,
/// the thermal profile controls the sensitivity and character of pattern detection.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ThermalProfile {
    /// The cooling rate.
    pub rate: CoolingRate,
    /// Maximum time to spend in cooling phase per task.
    pub max_cooling_duration: Duration,
    /// Whether to enable clustering pattern detection.
    pub detect_clustering: bool,
    /// Whether to enable phase transition detection.
    pub detect_phase_transitions: bool,
    /// Whether to enable conservation law detection.
    pub detect_conservation: bool,
    /// Whether to enable correlation detection.
    pub detect_correlations: bool,
}

impl Default for ThermalProfile {
    fn default() -> Self {
        ThermalProfile {
            rate: CoolingRate::Normal,
            max_cooling_duration: Duration::from_secs(60),
            detect_clustering: true,
            detect_phase_transitions: true,
            detect_conservation: true,
            detect_correlations: true,
        }
    }
}

impl ThermalProfile {
    /// Create a profile optimized for fast cooling — many fine patterns.
    pub fn fast_cooling() -> Self {
        ThermalProfile {
            rate: CoolingRate::Fast,
            max_cooling_duration: Duration::from_secs(10),
            ..ThermalProfile::default()
        }
    }

    /// Create a profile optimized for slow cooling — fewer, larger patterns.
    pub fn slow_cooling() -> Self {
        ThermalProfile {
            rate: CoolingRate::Slow,
            max_cooling_duration: Duration::from_secs(300),
            ..ThermalProfile::default()
        }
    }

    /// Create a profile with all detection disabled (useful for benchmarking).
    pub fn no_detection() -> Self {
        ThermalProfile {
            detect_clustering: false,
            detect_phase_transitions: false,
            detect_conservation: false,
            detect_correlations: false,
            ..ThermalProfile::default()
        }
    }

    /// Set the cooling rate.
    pub fn with_rate(mut self, rate: CoolingRate) -> Self {
        self.rate = rate;
        self
    }

    /// Disable clustering detection.
    pub fn without_clustering(mut self) -> Self {
        self.detect_clustering = false;
        self
    }

    /// Disable phase transition detection.
    pub fn without_phase_transitions(mut self) -> Self {
        self.detect_phase_transitions = false;
        self
    }

    /// Disable conservation law detection.
    pub fn without_conservation(mut self) -> Self {
        self.detect_conservation = false;
        self
    }

    /// Disable correlation detection.
    pub fn without_correlations(mut self) -> Self {
        self.detect_correlations = false;
        self
    }
}
