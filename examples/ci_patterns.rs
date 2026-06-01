//! CI build time anomaly detection: find clustering and phase shifts in build durations.

use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput};

struct CiBuild {
    duration_secs: f64,
    test_count: f64,
    branch: String,
}

impl CrackleTask for CiBuild {
    type Output = f64;

    fn fire(&self) -> TaskOutput<Self::Output> {
        TaskOutput::new(
            self.duration_secs,
            vec![
                ("duration".into(), self.duration_secs),
                ("test_count".into(), self.test_count),
                ("duration_per_test".into(), self.duration_secs / self.test_count.max(1.0)),
            ],
        )
    }

    fn label(&self) -> String {
        self.branch.clone()
    }
}

fn main() {
    let mut kiln = Kiln::new(ThermalProfile::default());

    // Normal builds
    kiln.fire_and_record(CiBuild { duration_secs: 45.0, test_count: 200.0, branch: "feature/auth".into() });
    kiln.fire_and_record(CiBuild { duration_secs: 42.0, test_count: 195.0, branch: "fix/typo".into() });
    kiln.fire_and_record(CiBuild { duration_secs: 48.0, test_count: 210.0, branch: "feature/ui".into() });

    // Something changed — builds got slower
    kiln.fire_and_record(CiBuild { duration_secs: 95.0, test_count: 200.0, branch: "feature/cache".into() });
    kiln.fire_and_record(CiBuild { duration_secs: 102.0, test_count: 198.0, branch: "chore/deps".into() });

    let patterns = kiln.cool();
    println!("CI Build Pattern Analysis");
    println!("=========================\n");

    for p in &patterns {
        println!("[{}] {}", p.kind().to_uppercase(), p.description());
        println!("  confidence: {:.2}", p.confidence());
        println!("  branches: {:?}\n", p.involved_tasks());
    }

    // Check for phase transition in build duration
    let phase_shifts: Vec<_> = patterns.iter()
        .filter(|p| format!("{}", p.kind()) == "phase transition")
        .collect();

    if !phase_shifts.is_empty() {
        println!("⚠️  Build duration shifted significantly — investigate recent changes!");
    }
}
