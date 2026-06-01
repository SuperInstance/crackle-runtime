//! Basic usage: fire tasks with metrics, detect patterns.

use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput};

struct NumberTask {
    value: f64,
    name: String,
}

impl CrackleTask for NumberTask {
    type Output = f64;

    fn fire(&self) -> TaskOutput<Self::Output> {
        TaskOutput::new(
            self.value,
            vec![
                ("value".into(), self.value),
                ("abs".into(), self.value.abs()),
                ("squared".into(), self.value * self.value),
            ],
        )
    }

    fn label(&self) -> String {
        self.name.clone()
    }
}

fn main() {
    let mut kiln = Kiln::new(ThermalProfile::fast_cooling());

    kiln.fire_and_record(NumberTask { value: 2.0, name: "a".into() });
    kiln.fire_and_record(NumberTask { value: 2.1, name: "b".into() });
    kiln.fire_and_record(NumberTask { value: 9.8, name: "c".into() });
    kiln.fire_and_record(NumberTask { value: 10.0, name: "d".into() });
    kiln.fire_and_record(NumberTask { value: 2.2, name: "e".into() });

    println!("Tasks in kiln: {}", kiln.task_count());

    let patterns = kiln.cool();
    println!("\nDetected {} pattern(s):\n", patterns.len());

    for p in &patterns {
        println!("[{}] {}", p.kind(), p.description());
        println!("  confidence: {:.2}", p.confidence());
        println!("  tasks: {:?}\n", p.involved_tasks());
    }
}
