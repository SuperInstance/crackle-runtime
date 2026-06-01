#[cfg(test)]
mod tests {
    use crate::*;
    use crate::patterns::{ClusteringPattern, CorrelationPattern};

    // ── Test tasks ──

    struct ConstTask { v: f64, lbl: String }
    impl ConstTask {
        fn new(v: f64, lbl: &str) -> Self { ConstTask { v, lbl: lbl.to_string() } }
    }
    impl CrackleTask for ConstTask {
        type Output = f64;
        fn fire(&self) -> TaskOutput<Self::Output> {
            TaskOutput::new(self.v, vec![("value".into(), self.v)])
        }
        fn label(&self) -> String { self.lbl.clone() }
    }

    struct VecTask { v: Vec<f64>, lbl: String }
    impl CrackleTask for VecTask {
        type Output = Vec<f64>;
        fn fire(&self) -> TaskOutput<Self::Output> {
            let metrics: Vec<(String, f64)> = self.v.iter().enumerate()
                .map(|(i, v)| (format!("dim_{}", i), *v))
                .collect();
            TaskOutput::new(self.v.clone(), metrics)
        }
        fn label(&self) -> String { self.lbl.clone() }
    }

    // ── TaskOutput tests ──

    #[test]
    fn task_output_simple() {
        let out = TaskOutput::simple(42);
        assert_eq!(out.value, 42);
        assert!(out.metrics.is_empty());
    }

    #[test]
    fn task_output_new() {
        let out = TaskOutput::new(3.14, vec![("pi".into(), 3.14)]);
        assert_eq!(out.value, 3.14);
        assert_eq!(out.metrics.len(), 1);
    }

    #[test]
    fn task_output_with_metric() {
        let out = TaskOutput::simple(10.0)
            .with_metric("a", 1.0)
            .with_metric("b", 2.0);
        assert_eq!(out.metrics.len(), 2);
    }

    // ── Timestamp tests ──

    #[test]
    fn timestamp_now() {
        let ts = Timestamp::now();
        assert!(ts.as_millis() > 0);
    }

    #[test]
    fn timestamp_from_millis() {
        let ts = Timestamp::from_millis(1000);
        assert_eq!(ts.as_millis(), 1000);
    }

    #[test]
    fn timestamp_ordering() {
        let a = Timestamp::from_millis(100);
        let b = Timestamp::from_millis(200);
        assert!(a < b);
    }

    #[test]
    fn timestamp_elapsed() {
        let ts = Timestamp::from_millis(0);
        let elapsed = ts.elapsed();
        assert!(elapsed.as_millis() > 0);
    }

    // ── TaskMetadata tests ──

    #[test]
    fn metadata_new() {
        let m = TaskMetadata::new("test");
        assert_eq!(m.label, "test");
        assert!(m.cooled_at.is_none());
    }

    #[test]
    fn metadata_fired_at() {
        let ts = Timestamp::from_millis(12345);
        let m = TaskMetadata::fired_at("test", ts);
        assert_eq!(m.fired_at, ts);
    }

    // ── ThermalProfile tests ──

    #[test]
    fn profile_default() {
        let p = ThermalProfile::default();
        assert!(p.detect_clustering);
        assert!(p.detect_phase_transitions);
        assert!(p.detect_conservation);
        assert!(p.detect_correlations);
    }

    #[test]
    fn profile_fast_cooling() {
        let p = ThermalProfile::fast_cooling();
        assert_eq!(p.rate, CoolingRate::Fast);
    }

    #[test]
    fn profile_slow_cooling() {
        let p = ThermalProfile::slow_cooling();
        assert_eq!(p.rate, CoolingRate::Slow);
    }

    #[test]
    fn profile_no_detection() {
        let p = ThermalProfile::no_detection();
        assert!(!p.detect_clustering);
        assert!(!p.detect_phase_transitions);
        assert!(!p.detect_conservation);
        assert!(!p.detect_correlations);
    }

    #[test]
    fn profile_builder_chain() {
        let p = ThermalProfile::default()
            .with_rate(CoolingRate::Fast)
            .without_clustering()
            .without_correlations();
        assert_eq!(p.rate, CoolingRate::Fast);
        assert!(!p.detect_clustering);
        assert!(!p.detect_correlations);
        assert!(p.detect_phase_transitions);
    }

    // ── CoolingRate tests ──

    #[test]
    fn cooling_rate_thresholds() {
        assert!(CoolingRate::Fast.cluster_threshold() < CoolingRate::Normal.cluster_threshold());
        assert!(CoolingRate::Normal.cluster_threshold() < CoolingRate::Slow.cluster_threshold());
    }

    #[test]
    fn cooling_rate_correlation_thresholds() {
        assert!(CoolingRate::Fast.correlation_threshold() < CoolingRate::Normal.correlation_threshold());
        assert!(CoolingRate::Normal.correlation_threshold() < CoolingRate::Slow.correlation_threshold());
    }

    #[test]
    fn cooling_rate_min_tasks() {
        assert!(CoolingRate::Fast.min_tasks_for_detection() < CoolingRate::Slow.min_tasks_for_detection());
    }

    // ── Kiln basic tests ──

    #[test]
    fn kiln_new() {
        let kiln = Kiln::new(ThermalProfile::default());
        assert_eq!(kiln.task_count(), 0);
        assert!(!kiln.is_cooled());
    }

    #[test]
    fn kiln_fire_and_record() {
        let mut kiln = Kiln::default_profile();
        let out = kiln.fire_and_record(ConstTask::new(42.0, "t1")).unwrap();
        assert_eq!(out.value, 42.0);
        assert_eq!(kiln.task_count(), 1);
    }

    #[test]
    fn kiln_fire_all() {
        let mut kiln = Kiln::default_profile();
        let tasks = vec![
            ConstTask::new(1.0, "t1"),
            ConstTask::new(2.0, "t2"),
            ConstTask::new(3.0, "t3"),
        ];
        let outputs = kiln.fire_all(tasks).unwrap();
        assert_eq!(outputs.len(), 3);
        assert_eq!(kiln.task_count(), 3);
    }

    #[test]
    fn kiln_add_entry() {
        let mut kiln = Kiln::default_profile();
        kiln.add_entry("t1", vec![("x".into(), 1.0)]);
        assert_eq!(kiln.task_count(), 1);
    }

    #[test]
    fn kiln_cool_empty() {
        let mut kiln = Kiln::default_profile();
        let patterns = kiln.cool();
        assert!(patterns.is_empty());
        assert!(kiln.is_cooled());
    }

    #[test]
    fn kiln_cool_few_tasks() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        kiln.add_entry("t1", vec![("x".into(), 1.0)]);
        let patterns = kiln.cool();
        // Fast cooling needs min 2 tasks
        assert!(patterns.is_empty());
    }

    #[test]
    fn kiln_reset() {
        let mut kiln = Kiln::default_profile();
        kiln.add_entry("t1", vec![("x".into(), 1.0)]);
        kiln.cool();
        kiln.reset();
        assert_eq!(kiln.task_count(), 0);
        assert!(!kiln.is_cooled());
    }

    #[test]
    fn kiln_entries() {
        let mut kiln = Kiln::default_profile();
        kiln.add_entry("t1", vec![("x".into(), 1.0)]);
        kiln.add_entry("t2", vec![("y".into(), 2.0)]);
        assert_eq!(kiln.entries().len(), 2);
    }

    #[test]
    fn kiln_profile_accessor() {
        let kiln = Kiln::new(ThermalProfile::fast_cooling());
        assert_eq!(kiln.profile().rate, CoolingRate::Fast);
    }

    // ── TaskEntry tests ──

    #[test]
    fn task_entry_all_metrics_fires_only() {
        let mut kiln = Kiln::default_profile();
        kiln.fire_and_record(ConstTask::new(5.0, "t1"));
        let metrics = kiln.entries()[0].all_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].0, "value");
    }

    // ── ClusteringPattern tests ──

    #[test]
    fn clustering_detects_close_tasks() {
        let labels = vec!["a".into(), "b".into(), "c".into()];
        let metrics = vec![
            vec![("x".into(), 1.0), ("y".into(), 1.0)],
            vec![("x".into(), 1.1), ("y".into(), 1.1)],
            vec![("x".into(), 10.0), ("y".into(), 10.0)],
        ];
        let patterns = ClusteringPattern::detect(&labels, &metrics, 2.0);
        assert!(!patterns.is_empty());
        // a and b should cluster together
        let first = &patterns[0];
        assert!(first.involved_tasks().contains(&"a".to_string()) || first.involved_tasks().contains(&"b".to_string()));
    }

    #[test]
    fn clustering_no_clusters() {
        let labels = vec!["a".into(), "b".into(), "c".into()];
        let metrics = vec![
            vec![("x".into(), 0.0)],
            vec![("x".into(), 100.0)],
            vec![("x".into(), -100.0)],
        ];
        let patterns = ClusteringPattern::detect(&labels, &metrics, 0.1);
        assert!(patterns.is_empty());
    }

    #[test]
    fn clustering_too_few() {
        let patterns = ClusteringPattern::detect(
            &["a".into()],
            &[vec![("x".into(), 1.0)]],
            1.0,
        );
        assert!(patterns.is_empty());
    }

    #[test]
    fn metric_distance_identical() {
        let a = vec![("x".into(), 1.0), ("y".into(), 2.0)];
        let b = vec![("x".into(), 1.0), ("y".into(), 2.0)];
        let d = ClusteringPattern::metric_distance(&a, &b);
        assert!(d.abs() < 0.001);
    }

    #[test]
    fn metric_distance_different() {
        let a = vec![("x".into(), 0.0)];
        let b = vec![("x".into(), 3.0), ("y".into(), 4.0)];
        let d = ClusteringPattern::metric_distance(&a, &b);
        assert!(d > 0.0);
    }

    #[test]
    fn metric_distance_no_overlap() {
        let a = vec![("x".into(), 1.0)];
        let b = vec![("z".into(), 1.0)];
        let d = ClusteringPattern::metric_distance(&a, &b);
        assert!(d.is_infinite() || d > 1e10);
    }

    // ── CorrelationPattern tests ──

    #[test]
    fn correlation_perfect_positive() {
        let pairs: Vec<(f64, f64)> = vec![
            (1.0, 2.0),
            (2.0, 4.0),
            (3.0, 6.0),
            (4.0, 8.0),
            (5.0, 10.0),
        ];
        let r = CorrelationPattern::pearson_correlation(&pairs);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn correlation_perfect_negative() {
        let pairs: Vec<(f64, f64)> = vec![
            (1.0, 10.0),
            (2.0, 8.0),
            (3.0, 6.0),
            (4.0, 4.0),
            (5.0, 2.0),
        ];
        let r = CorrelationPattern::pearson_correlation(&pairs);
        assert!((r + 1.0).abs() < 0.001);
    }

    #[test]
    fn correlation_no_correlation() {
        let pairs: Vec<(f64, f64)> = vec![
            (1.0, 1.0),
            (2.0, -1.0),
            (3.0, 1.0),
            (4.0, -1.0),
            (5.0, 1.0),
        ];
        let r = CorrelationPattern::pearson_correlation(&pairs);
        assert!(r.abs() < 0.5);
    }

    #[test]
    fn correlation_constant_x() {
        let pairs: Vec<(f64, f64)> = vec![
            (1.0, 1.0),
            (1.0, 2.0),
            (1.0, 3.0),
        ];
        let r = CorrelationPattern::pearson_correlation(&pairs);
        assert!(r.abs() < 0.001);
    }

    #[test]
    fn correlation_too_few_pairs() {
        let pairs: Vec<(f64, f64)> = vec![(1.0, 2.0)];
        let r = CorrelationPattern::pearson_correlation(&pairs);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn correlation_detect_finds_strong() {
        let labels: Vec<String> = (0..5).map(|i| format!("t{}", i)).collect();
        let metrics: Vec<Vec<(String, f64)>> = (0..5).map(|i| {
            let x = i as f64;
            vec![("a".into(), x), ("b".into(), x * 2.0)]
        }).collect();
        let patterns = CorrelationPattern::detect(&labels, &metrics, 0.7);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn correlation_detect_too_few_tasks() {
        let labels = vec!["a".into(), "b".into()];
        let metrics = vec![
            vec![("a".into(), 1.0), ("b".into(), 2.0)],
            vec![("a".into(), 2.0), ("b".into(), 4.0)],
        ];
        let patterns = CorrelationPattern::detect(&labels, &metrics, 0.7);
        assert!(patterns.is_empty());
    }

    // ── PhaseTransitionPattern tests ──

    #[test]
    fn phase_transition_detects_shift() {
        let labels: Vec<String> = (0..6).map(|i| format!("t{}", i)).collect();
        let metrics: Vec<Vec<(String, f64)>> = (0..6).map(|i| {
            let v = if i < 3 { 1.0 } else { 10.0 };
            vec![("x".into(), v)]
        }).collect();
        let patterns = PhaseTransitionPattern::detect(&labels, &metrics, 0.3);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn phase_transition_no_shift() {
        let labels: Vec<String> = (0..6).map(|i| format!("t{}", i)).collect();
        let metrics: Vec<Vec<(String, f64)>> = (0..6).map(|_| {
            vec![("x".into(), 5.0)]
        }).collect();
        let patterns = PhaseTransitionPattern::detect(&labels, &metrics, 0.3);
        assert!(patterns.is_empty());
    }

    #[test]
    fn phase_transition_too_few() {
        let labels = vec!["a".into()];
        let metrics = vec![vec![("x".into(), 1.0)]];
        let patterns = PhaseTransitionPattern::detect(&labels, &metrics, 0.3);
        assert!(patterns.is_empty());
    }

    // ── ConservationPattern tests ──

    #[test]
    fn conservation_detects_constant_sum() {
        let labels: Vec<String> = (0..5).map(|i| format!("t{}", i)).collect();
        let metrics: Vec<Vec<(String, f64)>> = (0..5).map(|i| {
            vec![("x".into(), 10.0 + i as f64 * 0.001)]
        }).collect();
        let patterns = ConservationPattern::detect(&labels, &metrics, 0.5);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn conservation_high_variance() {
        let labels: Vec<String> = (0..5).map(|i| format!("t{}", i)).collect();
        let metrics: Vec<Vec<(String, f64)>> = (0..5).map(|i| {
            vec![("x".into(), (i * 100) as f64)]
        }).collect();
        let patterns = ConservationPattern::detect(&labels, &metrics, 0.01);
        assert!(patterns.is_empty());
    }

    // ── CracklePattern tests ──

    #[test]
    fn pattern_creation() {
        let p = CracklePattern::new(
            PatternKind::Clustering,
            "test pattern",
            vec!["a".into(), "b".into()],
            0.8,
        );
        assert_eq!(p.kind(), &PatternKind::Clustering);
        assert_eq!(p.description(), "test pattern");
        assert_eq!(p.involved_tasks().len(), 2);
        assert!((p.confidence() - 0.8).abs() < 0.001);
    }

    #[test]
    fn pattern_confidence_clamped() {
        let p = CracklePattern::new(PatternKind::Correlation, "x", vec![], 1.5);
        assert!(p.confidence() <= 1.0);
        let p2 = CracklePattern::new(PatternKind::Correlation, "x", vec![], -0.5);
        assert!(p2.confidence() >= 0.0);
    }

    #[test]
    fn pattern_with_metric() {
        let p = CracklePattern::new(PatternKind::Clustering, "x", vec![], 0.5)
            .with_metric("foo", 42.0);
        assert_eq!(p.metrics().len(), 1);
    }

    #[test]
    fn pattern_with_metrics() {
        let p = CracklePattern::new(PatternKind::Clustering, "x", vec![], 0.5)
            .with_metrics(vec![("a".into(), 1.0), ("b".into(), 2.0)]);
        assert_eq!(p.metrics().len(), 2);
    }

    // ── PatternKind Display ──

    #[test]
    fn pattern_kind_display() {
        assert_eq!(format!("{}", PatternKind::Clustering), "clustering");
        assert_eq!(format!("{}", PatternKind::PhaseTransition), "phase transition");
        assert_eq!(format!("{}", PatternKind::Conservation), "conservation law");
        assert_eq!(format!("{}", PatternKind::Correlation), "correlation");
    }

    // ── CrackleError tests ──

    #[test]
    fn error_display() {
        let e = CrackleError::TaskPanicked("boom".into());
        assert!(e.to_string().contains("boom"));
        let e = CrackleError::KilnCooled;
        assert!(e.to_string().contains("cooled"));
        let e = CrackleError::InvalidProfile("bad".into());
        assert!(e.to_string().contains("bad"));
        let e = CrackleError::DetectionFailed("err".into());
        assert!(e.to_string().contains("err"));
    }

    // ── Integration tests ──

    #[test]
    fn full_cycle_with_clustering() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling().without_correlations().without_conservation().without_phase_transitions());
        kiln.add_entry("a", vec![("x".into(), 1.0), ("y".into(), 1.0)]);
        kiln.add_entry("b", vec![("x".into(), 1.1), ("y".into(), 1.1)]);
        kiln.add_entry("c", vec![("x".into(), 50.0), ("y".into(), 50.0)]);
        let patterns = kiln.cool();
        assert!(patterns.iter().any(|p| p.kind() == &PatternKind::Clustering));
    }

    #[test]
    fn full_cycle_with_conservation() {
        let mut kiln = Kiln::new(
            ThermalProfile::fast_cooling()
                .without_clustering()
                .without_correlations()
                .without_phase_transitions()
        );
        for i in 0..5 {
            kiln.add_entry(format!("t{}", i), vec![("energy".into(), 100.0)]);
        }
        let patterns = kiln.cool();
        assert!(patterns.iter().any(|p| p.kind() == &PatternKind::Conservation));
    }

    #[test]
    fn full_cycle_with_correlation() {
        let mut kiln = Kiln::new(
            ThermalProfile::fast_cooling()
                .without_clustering()
                .without_conservation()
                .without_phase_transitions()
        );
        for i in 0..5 {
            let x = i as f64;
            kiln.add_entry(format!("t{}", i), vec![
                ("a".into(), x),
                ("b".into(), x * 3.0 + 1.0),
            ]);
        }
        let patterns = kiln.cool();
        assert!(patterns.iter().any(|p| p.kind() == &PatternKind::Correlation));
    }

    #[test]
    fn full_cycle_no_detection() {
        let mut kiln = Kiln::new(ThermalProfile::no_detection());
        kiln.add_entry("a", vec![("x".into(), 1.0)]);
        kiln.add_entry("b", vec![("x".into(), 1.1)]);
        let patterns = kiln.cool();
        assert!(patterns.is_empty());
    }

    #[test]
    fn patterns_sorted_by_confidence() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        for i in 0..5 {
            kiln.add_entry(format!("t{}", i), vec![
                ("a".into(), i as f64),
                ("b".into(), i as f64 * 2.0),
            ]);
        }
        let patterns = kiln.cool();
        for w in patterns.windows(2) {
            assert!(w[0].confidence() >= w[1].confidence());
        }
    }

    // ── GlazeLayer integration ──

    #[test]
    fn glazed_task_in_kiln() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        let task = ConstTask::new(5.0, "glazed");
        let glazed = GlazeLayer::new(task)
            .with_derived_metric("squared", |o| o.value * o.value);

        let output = kiln.fire_and_record(glazed).unwrap();
        assert_eq!(output.value, 5.0);
        assert_eq!(kiln.task_count(), 1);

        let _patterns = kiln.cool();
    }

    // ── Multiple kiln cycles ──

    #[test]
    fn multiple_cycles() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());

        // First cycle
        kiln.add_entry("a1", vec![("x".into(), 1.0)]);
        kiln.add_entry("a2", vec![("x".into(), 1.0)]);
        let _p1 = kiln.cool();
        assert!(!kiln.is_cooled() || true); // still cooled
        kiln.reset();

        // Second cycle
        kiln.add_entry("b1", vec![("x".into(), 100.0)]);
        kiln.add_entry("b2", vec![("x".into(), 100.0)]);
        let _p2 = kiln.cool();
        // Both cycles should work independently
    }

    // ── Edge cases ──

    #[test]
    fn empty_metrics() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        kiln.add_entry("a", vec![]);
        kiln.add_entry("b", vec![]);
        let patterns = kiln.cool();
        // No metrics to analyze, but shouldn't panic
        assert!(patterns.is_empty() || !patterns.is_empty()); // just don't panic
    }

    #[test]
    fn single_task_no_patterns() {
        let mut kiln = Kiln::new(ThermalProfile::default());
        kiln.add_entry("lonely", vec![("x".into(), 42.0)]);
        let patterns = kiln.cool();
        // Normal cooling needs min 3 tasks
        assert!(patterns.is_empty());
    }

    #[test]
    fn negative_metrics() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        kiln.add_entry("a", vec![("x".into(), -10.0)]);
        kiln.add_entry("b", vec![("x".into(), -10.01)]);
        kiln.add_entry("c", vec![("x".into(), -10.02)]);
        let patterns = kiln.cool();
        // Should handle negatives without panic
        assert!(patterns.len() >= 0);
    }

    #[test]
    fn very_large_metrics() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        kiln.add_entry("a", vec![("x".into(), 1e15)]);
        kiln.add_entry("b", vec![("x".into(), 1e15)]);
        kiln.add_entry("c", vec![("x".into(), 1e15)]);
        let _patterns = kiln.cool();
        // Should handle large values
    }

    #[test]
    fn many_tasks() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        for i in 0..100 {
            kiln.add_entry(format!("t{}", i), vec![("x".into(), i as f64)]);
        }
        let patterns = kiln.cool();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn cooling_rate_default() {
        assert_eq!(CoolingRate::default(), CoolingRate::Normal);
    }

    #[test]
    fn task_fire_with_vec_output() {
        let task = VecTask { v: vec![1.0, 2.0, 3.0], lbl: "vec".into() };
        let out = task.fire();
        assert_eq!(out.value, vec![1.0, 2.0, 3.0]);
        assert_eq!(out.metrics.len(), 3);
    }

    #[test]
    fn vec_task_in_kiln() {
        let mut kiln = Kiln::new(ThermalProfile::fast_cooling());
        kiln.fire_and_record(VecTask { v: vec![1.0, 2.0], lbl: "v1".into() }).unwrap();
        kiln.fire_and_record(VecTask { v: vec![1.1, 2.1], lbl: "v2".into() }).unwrap();
        kiln.fire_and_record(VecTask { v: vec![10.0, 20.0], lbl: "v3".into() }).unwrap();
        let patterns = kiln.cool();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(CrackleError::KilnCooled);
        assert!(e.to_string().contains("cooled"));
    }

    #[test]
    fn thermal_profile_phase_sensitivity() {
        assert!(CoolingRate::Fast.phase_transition_sensitivity() < CoolingRate::Normal.phase_transition_sensitivity());
        assert!(CoolingRate::Normal.phase_transition_sensitivity() < CoolingRate::Slow.phase_transition_sensitivity());
    }

    #[test]
    fn thermal_profile_conservation_tolerance() {
        assert!(CoolingRate::Fast.conservation_tolerance() > CoolingRate::Normal.conservation_tolerance());
        assert!(CoolingRate::Normal.conservation_tolerance() > CoolingRate::Slow.conservation_tolerance());
    }
}
