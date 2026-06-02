//! Real test: event stream processor with conservation laws and degradation detection.
//!
//! Simulates processing 1000 events in batches, applying conservation checks
//! between batches, and detecting when the system is degrading.
//!
//! This tests crackle-runtime in a scenario closer to production — event streams
//! with throughput, latency, and error metrics.

use crackle_runtime::{CrackleTask, Kiln, ThermalProfile, TaskOutput};
use std::time::{Duration, Instant};

/// An event being processed through the stream processor.
struct ProcessedEvent {
    event_id: u64,
    batch_id: u32,
    processing_time_ms: f64,
    throughput: f64,         // events/sec for this batch
    memory_mb: f64,          // memory used during processing
    error_rate: f64,         // 0.0–1.0
    records_in: f64,
    records_out: f64,
    cpu_temp_c: f64,         // system temperature proxy
    latency_p99_ms: f64,     // p99 latency
    consumer_lag: f64,       // kafka-style consumer lag
    gc_pause_ms: f64,        // GC pause time
}

impl CrackleTask for ProcessedEvent {
    type Output = f64;

    fn fire(&self) -> TaskOutput<Self::Output> {
        TaskOutput::new(
            self.processing_time_ms,
            vec![
                ("processing_time_ms".into(),     self.processing_time_ms),
                ("throughput".into(),              self.throughput),
                ("memory_mb".into(),               self.memory_mb),
                ("error_rate".into(),              self.error_rate),
                ("records_in".into(),              self.records_in),
                ("records_out".into(),             self.records_out),
                ("cpu_temp_c".into(),              self.cpu_temp_c),
                ("latency_p99_ms".into(),          self.latency_p99_ms),
                ("consumer_lag".into(),            self.consumer_lag),
                ("gc_pause_ms".into(),             self.gc_pause_ms),
                ("event_id".into(),                self.event_id as f64),
                ("batch_id".into(),                self.batch_id as f64),
            ],
        )
    }

    fn label(&self) -> String {
        format!("batch-{}-event-{}", self.batch_id, self.event_id)
    }
}

/// Simulate a batch of events with given characteristics.
fn simulate_batch(
    batch_id: u32,
    batch_size: u32,
    base_latency_ms: f64,
    degradation_factor: f64,  // 0.0 = healthy, increases over time
) -> Vec<ProcessedEvent> {
    let mut events = Vec::with_capacity(batch_size as usize);
    for i in 0..batch_size {
        let event_id = batch_id as u64 * 1000 + i as u64;
        let noise: f64 = (i as f64 * 7.0).sin() * 0.1; // small sine wave noise
        let degradation = degradation_factor * (i as f64 / batch_size as f64);

        let processing_time_ms = base_latency_ms * (1.0 + degradation + noise);
        let throughput = 1000.0 / processing_time_ms * 1000.0;
        let memory_mb = 128.0 + degradation * 64.0 + noise * 10.0;
        let error_rate = (degradation * 0.5 + noise.abs() * 0.05).min(0.5);
        let records_in = 1000.0;
        let records_out = records_in * (1.0 - error_rate);
        let cpu_temp_c = 65.0 + degradation * 30.0 + noise * 5.0;
        let latency_p99_ms = processing_time_ms * 2.5;
        let consumer_lag = degradation * 10000.0 + (i as f64 * 3.0).sin().abs() * 100.0;
        let gc_pause_ms = 5.0 + degradation * 50.0;

        events.push(ProcessedEvent {
            event_id,
            batch_id,
            processing_time_ms,
            throughput,
            memory_mb,
            error_rate,
            records_in,
            records_out,
            cpu_temp_c,
            latency_p99_ms,
            consumer_lag,
            gc_pause_ms,
        });
    }
    events
}

fn main() {
    let total_events = 1000;
    let batch_size = 100;
    let num_batches = total_events / batch_size;

    println!("═══════════════════════════════════════════════════════════════");
    println!("  CRACKLE-RUNTIME: EVENT STREAM PROCESSOR TEST");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("Processing {} events across {} batches of {}...\n",
             total_events, num_batches, batch_size);

    let start = Instant::now();

    // Phase 1: Healthy system (batches 1-4)
    println!("─── Phase 1: Healthy System ───");
    let mut healthy_kiln = Kiln::new(
        ThermalProfile::fast_cooling()
    );

    // Batch 1-4: all healthy, 10ms baseline latency
    for batch_id in 1..=4 {
        let events = simulate_batch(batch_id, batch_size as u32, 10.0, 0.05);
        for event in events {
            healthy_kiln.fire_and_record(event).unwrap();
        }

        let elapsed = start.elapsed();
        println!("  Batch {}/{} fired ({} events, {:.0}ms)",
                 batch_id, num_batches, batch_size, elapsed.as_millis());
    }

    // Cool healthy phase
    let healthy_patterns = healthy_kiln.cool();
    println!("\n  Healthy system patterns ({} detected):", healthy_patterns.len());
    for p in &healthy_patterns {
        println!("    [{:?}] {} (conf: {:.2})", p.kind(), p.description(), p.confidence());
    }
    // Also check conservation on records_in/records_out
    let conservation = healthy_kiln.distribution_shift(10);
    println!("    Distribution shifts:");
    for (name, shift) in &conservation[..conservation.len().min(3)] {
        println!("      {} → KL = {:.4}", name, shift);
    }

    println!();

    // Phase 2: Degrading system (batches 5-8)
    println!("─── Phase 2: Degrading System ───");

    let mut degrading_kiln = Kiln::new(
        ThermalProfile::fast_cooling()
    );

    // Batches where degradation ramps up
    let degradation_levels = [0.2, 0.5, 0.9, 1.5];
    for (i, &degradation) in degradation_levels.iter().enumerate() {
        let batch_id = (5 + i) as u32;
        // latency increases with degradation
        let base_latency = 10.0 + degradation * 50.0;

        let events = simulate_batch(batch_id, batch_size as u32, base_latency, degradation);
        for event in events {
            degrading_kiln.fire_and_record(event).unwrap();
        }

        let elapsed = start.elapsed();
        println!("  Batch {}/{} [degradation={:.1}x] fired (latency baseline: {:.0}ms, {:.0}ms total)",
                 batch_id, num_batches, degradation, base_latency, elapsed.as_millis());
    }

    let degrading_patterns = degrading_kiln.cool();
    println!("\n  Degrading system patterns ({} detected):", degrading_patterns.len());
    for p in &degrading_patterns {
        println!("    [{:?}] {} (conf: {:.2})", p.kind(), p.description(), p.confidence());
    }

    // JSD shift (distribution shape changes)
    let jsd_shifts = degrading_kiln.jsd_shift(10);
    println!("    JSD shifts (distribution shape changes):");
    for (name, shift) in &jsd_shifts[..jsd_shifts.len().min(5)] {
        println!("      {} → JSD = {:.4}", name, shift);
    }

    // Permutation entropy (temporal structure)
    let pes = degrading_kiln.permutation_entropies(4);
    println!("    Permutation entropies (temporal structure):");
    for (name, pe) in &pes {
        println!("      {} → PE = {:.4}", name, pe);
    }

    println!();

    // Phase 3: Conservation law test — records in = records out should be violated
    // as errors increase
    println!("─── Phase 3: Conservation Law Verification ───");
    let mut conservation_kiln = Kiln::new(
        ThermalProfile::fast_cooling()
    );

    // Force records_in = records_out for first 3 batches (healthy conservation)
    for batch_id in 1..=3 {
        let event = ProcessedEvent {
            event_id: batch_id as u64,
            batch_id: batch_id as u32,
            processing_time_ms: 10.0,
            throughput: 100.0,
            memory_mb: 128.0,
            error_rate: 0.01,
            records_in: 1000.0,
            records_out: 995.0,  // nearly conserved
            cpu_temp_c: 65.0,
            latency_p99_ms: 25.0,
            consumer_lag: 10.0,
            gc_pause_ms: 5.0,
        };
        conservation_kiln.fire_and_record(event).unwrap();
    }

    // Force violation of records conservation (errors spike)
    for batch_id in 4..=6 {
        let event = ProcessedEvent {
            event_id: batch_id as u64,
            batch_id: batch_id as u32,
            processing_time_ms: 10.0 + batch_id as f64 * 10.0,
            throughput: 50.0,
            memory_mb: 256.0,
            error_rate: 0.3,
            records_in: 1000.0,
            records_out: 700.0,  // 30% loss — conservation violated
            cpu_temp_c: 80.0,
            latency_p99_ms: 100.0,
            consumer_lag: 500.0,
            gc_pause_ms: 20.0,
        };
        conservation_kiln.fire_and_record(event).unwrap();
    }

    let conservation_patterns = conservation_kiln.cool();
    println!("  Conservation-specific patterns ({} detected):", conservation_patterns.len());
    for p in &conservation_patterns {
        println!("    [{:?}] {} (conf: {:.2})", p.kind(), p.description(), p.confidence());
    }

    // MI matrix to show nonlinear dependencies
    let mi = conservation_kiln.mi_matrix(8);
    println!("  Mutual Information matrix (8 bins):");
    let metric_names = ["processing_time_ms", "error_rate", "records_in", "records_out", "memory_mb"];
    for (i, name_i) in metric_names.iter().enumerate() {
        for (j, name_j) in metric_names.iter().enumerate() {
            if i == j {
                println!("    I({:25}; {:25}) = H({})", name_i, name_j, name_i);
            } else {
                println!("    I({:25}; {:25}) = {:.4} bits", name_i, name_j, mi[i][j]);
            }
        }
    }

    println!();
    println!("───────────────────────────────────────────────────────────────");
    println!("  SUMMARY");
    println!("───────────────────────────────────────────────────────────────\n");

    // Degradation detection analysis
    println!("  Degradation Metrics:");
    let pt = degrading_patterns.iter().filter(|p| matches!(p.kind(), crackle_runtime::PatternKind::PhaseTransition));
    let pt_count = pt.count();
    println!("    Phase transitions detected in degraded system: {} alerts",
             pt_count);

    let corr = degrading_patterns.iter().filter(|p| matches!(p.kind(), crackle_runtime::PatternKind::Correlation));
    let corr_count = corr.count();
    println!("    Correlations in degraded system: {} pairs", corr_count);

    let cons = degrading_patterns.iter().filter(|p| matches!(p.kind(), crackle_runtime::PatternKind::Conservation));
    let cons_count = cons.count();
    println!("    Conservation laws in degraded system: {} found", cons_count);

    let clust = degrading_patterns.iter().filter(|p| matches!(p.kind(), crackle_runtime::PatternKind::Clustering));
    let clust_count = clust.count();
    println!("    Clusters in degraded system: {} groups", clust_count);

    println!();
    let total_duration = start.elapsed();
    println!("  Total test duration: {:.2}s", total_duration.as_secs_f64());
    println!("  Events processed: {}", total_events);
    println!("  Avg throughput: {:.0} events/s",
             total_events as f64 / total_duration.as_secs_f64());
}
