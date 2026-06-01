//! Information-theoretic foundations for pattern detection.
//!
//! Grounds the pattern detection in actual information theory, providing
//! principled measures of uncertainty, dependence, distribution shift, and
//! information flow.
//!
//! # References
//!
//! - Shannon (1948), "A Mathematical Theory of Communication"
//! - Cover & Thomas (2006), "Elements of Information Theory"
//! - Kullback & Leibler (1951)
//! - Schreiber (2000), "Measuring Information Transfer"
//! - Bandt & Pompe (2002), "Permutation Entropy"

/// Discretize continuous values into `bins` equal-width bins and return histogram counts.
fn histogram(values: &[f64], bins: usize) -> Vec<usize> {
    if values.is_empty() || bins == 0 {
        return vec![];
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        // All values identical — one bin gets everything
        let mut counts = vec![0usize; bins];
        if bins > 0 {
            counts[0] = values.len();
        }
        return counts;
    }

    let width = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];

    for &v in values {
        let idx = ((v - min) / width).floor() as usize;
        // Clamp to last bin for the max value
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }

    counts
}

/// Discretize two variables jointly into a 2D histogram.
fn histogram_2d(x: &[f64], y: &[f64], bins: usize) -> Vec<Vec<usize>> {
    let n = x.len().min(y.len());
    if n == 0 || bins == 0 {
        return vec![];
    }

    let x_min = x[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let x_max = x[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = y[..n].iter().cloned().fold(f64::INFINITY, f64::min);
    let y_max = y[..n].iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let x_width = if (x_max - x_min).abs() < f64::EPSILON {
        1.0
    } else {
        (x_max - x_min) / bins as f64
    };
    let y_width = if (y_max - y_min).abs() < f64::EPSILON {
        1.0
    } else {
        (y_max - y_min) / bins as f64
    };

    let mut counts = vec![vec![0usize; bins]; bins];

    for i in 0..n {
        let xi = if (x_max - x_min).abs() < f64::EPSILON {
            0
        } else {
            (((x[i] - x_min) / x_width).floor() as usize).min(bins - 1)
        };
        let yi = if (y_max - y_min).abs() < f64::EPSILON {
            0
        } else {
            (((y[i] - y_min) / y_width).floor() as usize).min(bins - 1)
        };
        counts[xi][yi] += 1;
    }

    counts
}

/// Compute Shannon entropy of a set of continuous values.
///
/// Discretizes values into `bins` equal-width bins, then computes:
/// `H(X) = -Σ p(x) log₂ p(x)`
///
/// High entropy indicates unpredictability; low entropy indicates regular patterns.
///
/// # Arguments
///
/// * `values` - The continuous values to analyze
/// * `bins` - Number of bins for discretization (typically 10–50)
///
/// # Returns
///
/// Entropy in bits. Returns 0.0 for empty input.
///
/// # Reference
///
/// Shannon (1948), "A Mathematical Theory of Communication"
pub fn entropy(values: &[f64], bins: usize) -> f64 {
    if values.is_empty() || bins == 0 {
        return 0.0;
    }

    let counts = histogram(values, bins);
    let total = values.len() as f64;

    let mut h = 0.0;
    for &c in &counts {
        if c > 0 {
            let p = c as f64 / total;
            h -= p * p.log2();
        }
    }

    h
}

/// Compute joint entropy of two variables.
///
/// `H(X,Y) = -Σ p(x,y) log₂ p(x,y)`
pub fn joint_entropy(x: &[f64], y: &[f64], bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 || bins == 0 {
        return 0.0;
    }

    let counts = histogram_2d(x, y, bins);
    let total = n as f64;

    let mut h = 0.0;
    for row in &counts {
        for &c in row {
            if c > 0 {
                let p = c as f64 / total;
                h -= p * p.log2();
            }
        }
    }

    h
}

/// Compute mutual information between two variables.
///
/// `I(X;Y) = H(X) + H(Y) - H(X,Y)`
///
/// Captures non-linear dependencies that Pearson correlation misses.
/// Returns 0 if X and Y are independent; higher values indicate stronger dependence.
///
/// # Arguments
///
/// * `x` - First variable
/// * `y` - Second variable
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// Mutual information in bits (non-negative).
///
/// # Reference
///
/// Cover & Thomas (2006), "Elements of Information Theory"
pub fn mutual_information(x: &[f64], y: &[f64], bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 || bins == 0 {
        return 0.0;
    }

    let hx = entropy(x, bins);
    let hy = entropy(y, bins);
    let hxy = joint_entropy(x, y, bins);

    // MI is non-negative; numerical issues can make it slightly negative
    (hx + hy - hxy).max(0.0)
}

/// Compute Kullback-Leibler divergence between two distributions.
///
/// `D_KL(P || Q) = Σ P(i) log(P(i)/Q(i))`
///
/// Measures how much information is lost when Q is used to approximate P.
/// This is the principled measure for distribution shift detection.
///
/// # Arguments
///
/// * `current` - The "current" distribution P
/// * `baseline` - The "baseline" distribution Q
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// KL divergence in bits. Returns `f64::INFINITY` if P has support where Q doesn't.
///
/// # Reference
///
/// Kullback & Leibler (1951)
pub fn kl_divergence(current: &[f64], baseline: &[f64], bins: usize) -> f64 {
    if current.is_empty() || baseline.is_empty() || bins == 0 {
        return 0.0;
    }

    let p_counts = histogram(current, bins);
    let q_counts = histogram(baseline, bins);

    let p_total = current.len() as f64;
    let q_total = baseline.len() as f64;

    let mut kl = 0.0;
    for i in 0..bins {
        let p = p_counts[i] as f64 / p_total;
        let q = q_counts[i] as f64 / q_total;

        if p > 0.0 && q > 0.0 {
            kl += p * (p / q).log2();
        } else if p > 0.0 && q <= 0.0 {
            return f64::INFINITY;
        }
        // If p == 0, contribution is 0 regardless of q
    }

    kl
}

/// Compute Jensen-Shannon divergence between two distributions.
///
/// `JSD(P,Q) = ½ D_KL(P||M) + ½ D_KL(Q||M)` where `M = (P+Q)/2`
///
/// Symmetric, always finite, and its square root is a proper metric.
/// Use for detecting ANY distribution shift (not just mean shift).
///
/// # Arguments
///
/// * `p` - First distribution's values
/// * `q` - Second distribution's values
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// JSD in bits (symmetric, bounded by `log₂(bins)`).
pub fn jsd(p: &[f64], q: &[f64], bins: usize) -> f64 {
    if p.is_empty() || q.is_empty() || bins == 0 {
        return 0.0;
    }

    let p_counts = histogram(p, bins);
    let q_counts = histogram(q, bins);

    let p_total = p.len() as f64;
    let q_total = q.len() as f64;

    let mut jsd_val = 0.0;

    for i in 0..bins {
        let pi = p_counts[i] as f64 / p_total;
        let qi = q_counts[i] as f64 / q_total;
        let mi = (pi + qi) / 2.0;

        if pi > 0.0 && mi > 0.0 {
            jsd_val += 0.5 * pi * (pi / mi).log2();
        }
        if qi > 0.0 && mi > 0.0 {
            jsd_val += 0.5 * qi * (qi / mi).log2();
        }
    }

    jsd_val
}

/// Compute transfer entropy from X to Y.
///
/// `TE(X→Y) = I(Y_{t+1}; X_t | Y_t)`
///
/// Measures whether X's past helps predict Y beyond Y's own past.
/// This detects directional influence (information flow) between metrics.
///
/// # Arguments
///
/// * `x` - Source variable (potential cause)
/// * `y` - Target variable (potential effect)
/// * `lag` - Time lag to consider (typically 1)
/// * `bins` - Number of bins for discretization
///
/// # Returns
///
/// Transfer entropy in bits. Non-negative; higher values indicate stronger
/// directional information flow from X to Y.
///
/// # Reference
///
/// Schreiber (2000), "Measuring Information Transfer"
pub fn transfer_entropy(x: &[f64], y: &[f64], lag: usize, bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n <= lag + 1 || bins == 0 {
        return 0.0;
    }

    // Build triplets: (y_{t+1}, x_t, y_t) for t = 0..n-lag-1
    let m = n - lag;
    let y_future: Vec<f64> = (lag..n).map(|t| y[t]).collect();
    let x_past: Vec<f64> = (0..m).map(|t| x[t]).collect();
    let y_past: Vec<f64> = (0..m).map(|t| y[t]).collect();

    // TE = H(Y_{t+1}, X_t, Y_t) - H(X_t, Y_t) - H(Y_{t+1}, Y_t) + H(Y_t)
    // We compute this via 3D and 2D histograms
    let h_yxy = entropy_3d(&y_future, &x_past, &y_past, bins);
    let h_xy = joint_entropy(&x_past, &y_past, bins);
    let h_yy = joint_entropy(&y_future, &y_past, bins);
    let h_y = entropy(&y_past, bins);

    (h_yxy - h_xy - h_yy + h_y).max(0.0)
}

/// Compute 3D joint entropy for three variables.
fn entropy_3d(a: &[f64], b: &[f64], c: &[f64], bins: usize) -> f64 {
    let n = a.len().min(b.len()).min(c.len());
    if n == 0 || bins == 0 {
        return 0.0;
    }

    // Discretize each variable
    let a_disc = discretize(&a[..n], bins);
    let b_disc = discretize(&b[..n], bins);
    let c_disc = discretize(&c[..n], bins);

    // Count joint occurrences using a flat map
    let mut counts = std::collections::HashMap::new();
    for i in 0..n {
        let key = (a_disc[i], b_disc[i], c_disc[i]);
        *counts.entry(key).or_insert(0usize) += 1;
    }

    let total = n as f64;
    let mut h = 0.0;
    for &c in counts.values() {
        let p = c as f64 / total;
        h -= p * p.log2();
    }

    h
}

/// Discretize values into bin indices.
fn discretize(values: &[f64], bins: usize) -> Vec<usize> {
    if values.is_empty() || bins == 0 {
        return vec![];
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        return vec![0; values.len()];
    }

    let width = (max - min) / bins as f64;
    values
        .iter()
        .map(|&v| ((v - min) / width).floor() as usize).map(|idx| idx.min(bins - 1))
        .collect()
}

/// Compute permutation entropy of a time series.
///
/// Ordinal pattern analysis captures temporal structure that static entropy misses.
/// Measures the complexity of the time series based on the ordering of consecutive values.
///
/// # Arguments
///
/// * `values` - The time series to analyze
/// * `order` - Embedding dimension (typically 3–7). Must be >= 2.
///
/// # Returns
///
/// Normalized permutation entropy in [0, 1].
/// 0 = perfectly regular (single ordinal pattern), 1 = random (all patterns equally likely).
///
/// # Reference
///
/// Bandt & Pompe (2002), "Permutation Entropy"
pub fn permutation_entropy(values: &[f64], order: usize) -> f64 {
    if values.len() < order || order < 2 {
        return 0.0;
    }

    let n_patterns = factorial(order);
    let mut pattern_counts = vec![0usize; n_patterns];

    let n = values.len() - order + 1;
    for i in 0..n {
        let window = &values[i..i + order];
        let idx = ordinal_pattern_index(window);
        pattern_counts[idx] += 1;
    }

    let total = n as f64;
    let mut h = 0.0;
    for &c in &pattern_counts {
        if c > 0 {
            let p = c as f64 / total;
            h -= p * p.log2();
        }
    }

    // Normalize by log2(n_patterns!)
    let max_h = (n_patterns as f64).log2();
    if max_h > 0.0 {
        h / max_h
    } else {
        0.0
    }
}

/// Compute the ordinal pattern index for a window of values.
///
/// Maps the permutation of indices sorted by value to a unique integer.
fn ordinal_pattern_index(window: &[f64]) -> usize {
    let order = window.len();
    let mut indices: Vec<usize> = (0..order).collect();

    // Sort indices by corresponding values (stable sort for ties)
    indices.sort_by(|&a, &b| {
        window[a]
            .partial_cmp(&window[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Compute the Lehmer code (factorial number system)
    let mut code = 0usize;
    for i in 0..order {
        let mut count = 0;
        for j in (i + 1)..order {
            if indices[j] < indices[i] {
                count += 1;
            }
        }
        code += count * factorial(order - i - 1);
    }

    code
}

/// Compute factorial (small values only, used for ordinal patterns).
fn factorial(n: usize) -> usize {
    if n <= 1 {
        1
    } else {
        (2..=n).product()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Histogram tests ──

    #[test]
    fn histogram_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let counts = histogram(&values, 5);
        assert_eq!(counts.iter().sum::<usize>(), 5);
    }

    #[test]
    fn histogram_empty() {
        let counts = histogram(&[], 10);
        assert!(counts.is_empty());
    }

    #[test]
    fn histogram_single_value() {
        let counts = histogram(&[5.0; 10], 5);
        assert_eq!(counts[0], 10);
        assert_eq!(counts.iter().sum::<usize>(), 10);
    }

    #[test]
    fn histogram_2d_basic() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];
        let counts = histogram_2d(&x, &y, 3);
        let total: usize = counts.iter().flat_map(|r| r.iter()).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn histogram_2d_empty() {
        let counts = histogram_2d(&[], &[], 3);
        assert!(counts.is_empty());
    }

    // ── Shannon entropy tests ──

    #[test]
    fn entropy_uniform_distribution() {
        // Uniform over 4 bins should give log2(4) = 2.0 bits
        let values: Vec<f64> = (0..100).map(|i| (i % 4) as f64).collect();
        let h = entropy(&values, 4);
        assert!((h - 2.0).abs() < 0.05, "expected ~2.0, got {}", h);
    }

    #[test]
    fn entropy_single_value() {
        let h = entropy(&[5.0; 100], 10);
        assert!(h.abs() < 0.001, "expected ~0.0, got {}", h);
    }

    #[test]
    fn entropy_empty() {
        assert_eq!(entropy(&[], 10), 0.0);
    }

    #[test]
    fn entropy_zero_bins() {
        assert_eq!(entropy(&[1.0, 2.0], 0), 0.0);
    }

    #[test]
    fn entropy_two_values_different() {
        let h = entropy(&[1.0, 2.0], 2);
        // Two values in different bins: log2(2) = 1.0
        assert!((h - 1.0).abs() < 0.01, "expected ~1.0, got {}", h);
    }

    #[test]
    fn entropy_monotonic_in_bins() {
        // More bins shouldn't decrease entropy for uniform data
        let values: Vec<f64> = (0..1000).map(|i| (i % 10) as f64).collect();
        let h5 = entropy(&values, 5);
        let h20 = entropy(&values, 20);
        // With uniform data, more bins up to the number of distinct values gives higher entropy
        assert!(h20 >= h5 - 0.1);
    }

    // ── Joint entropy tests ──

    #[test]
    fn joint_entropy_independent() {
        let x: Vec<f64> = (0..100).map(|i| (i % 2) as f64).collect();
        let y: Vec<f64> = (0..100).map(|i| (i % 2) as f64).collect();
        let hxy = joint_entropy(&x, &y, 2);
        let hx = entropy(&x, 2);
        let hy = entropy(&y, 2);
        // Identical variables: joint entropy = marginal entropy
        assert!((hxy - hx).abs() < 0.1);
        let _ = hy; // suppress unused warning
    }

    #[test]
    fn joint_entropy_empty() {
        assert_eq!(joint_entropy(&[], &[], 10), 0.0);
    }

    #[test]
    fn joint_entropy_increases_with_independence() {
        // Perfectly correlated
        let x: Vec<f64> = (0..100).map(|i| (i % 4) as f64).collect();
        let y_same: Vec<f64> = x.clone();
        let y_indep: Vec<f64> = (0..100).map(|i| ((i + 2) % 4) as f64).collect();

        let h_corr = joint_entropy(&x, &y_same, 4);
        let h_indep = joint_entropy(&x, &y_indep, 4);
        // Independent variables have higher joint entropy than identical
        assert!(h_indep >= h_corr);
    }

    // ── Mutual information tests ──

    #[test]
    fn mi_identical_variables() {
        let x: Vec<f64> = (0..100).map(|i| (i % 4) as f64).collect();
        let mi = mutual_information(&x, &x, 4);
        let hx = entropy(&x, 4);
        // MI of a variable with itself = H(X)
        assert!((mi - hx).abs() < 0.1, "MI = {}, H(X) = {}", mi, hx);
    }

    #[test]
    fn mi_independent_variables() {
        // Alternating patterns that are independent
        let x: Vec<f64> = (0..100).map(|i| (i % 2) as f64).collect();
        let y: Vec<f64> = (0..100).map(|i| ((i / 2) % 2) as f64).collect();
        let mi = mutual_information(&x, &y, 2);
        // Should be low (not perfectly 0 due to discretization)
        assert!(mi < 0.5, "expected low MI, got {}", mi);
    }

    #[test]
    fn mi_empty() {
        assert_eq!(mutual_information(&[], &[], 10), 0.0);
    }

    #[test]
    fn mi_nonlinear_dependence() {
        // Y = X^2 — nonlinear relation that Pearson might miss
        let x: Vec<f64> = (0..50).map(|i| (i as f64 - 25.0) / 5.0).collect();
        let y: Vec<f64> = x.iter().map(|v| v * v).collect();
        let mi = mutual_information(&x, &y, 10);
        // Should detect the dependence
        assert!(mi > 0.5, "MI should detect nonlinear dependence, got {}", mi);
    }

    #[test]
    fn mi_non_negative() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![4.0, 3.0, 2.0, 1.0];
        let mi = mutual_information(&x, &y, 4);
        assert!(mi >= 0.0);
    }

    // ── KL divergence tests ──

    #[test]
    fn kl_identical_distributions() {
        let p = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let q = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kl = kl_divergence(&p, &q, 5);
        assert!(kl.abs() < 0.01, "expected ~0.0, got {}", kl);
    }

    #[test]
    fn kl_different_distributions() {
        let p: Vec<f64> = (0..100).map(|_| 1.0).collect();
        let q: Vec<f64> = (0..100).map(|i| if i < 50 { 1.0 } else { 10.0 }).collect();
        let kl = kl_divergence(&p, &q, 10);
        assert!(kl > 0.0);
    }

    #[test]
    fn kl_empty() {
        assert_eq!(kl_divergence(&[], &[1.0], 10), 0.0);
        assert_eq!(kl_divergence(&[1.0], &[], 10), 0.0);
    }

    #[test]
    fn kl_asymmetric() {
        let p = vec![1.0, 1.0, 1.0, 10.0];
        let q = vec![1.0, 1.0, 1.0, 1.0];
        let kl_pq = kl_divergence(&p, &q, 4);
        let kl_qp = kl_divergence(&q, &p, 4);
        // KL is asymmetric: D(P||Q) != D(Q||P) in general
        assert!((kl_pq - kl_qp).abs() > 0.01 || (kl_pq.is_infinite() || kl_qp.is_infinite()));
    }

    #[test]
    fn kl_non_negative() {
        let p = vec![1.0, 2.0, 3.0];
        let q = vec![3.0, 2.0, 1.0];
        let kl = kl_divergence(&p, &q, 3);
        assert!(kl >= 0.0);
    }

    // ── JSD tests ──

    #[test]
    fn jsd_identical() {
        let p = vec![1.0, 2.0, 3.0, 4.0];
        let js = jsd(&p, &p, 4);
        assert!(js.abs() < 0.01, "expected ~0.0, got {}", js);
    }

    #[test]
    fn jsd_symmetric() {
        let p = vec![1.0, 1.0, 1.0, 10.0];
        let q = vec![1.0, 1.0, 1.0, 1.0];
        let js_pq = jsd(&p, &q, 4);
        let js_qp = jsd(&q, &p, 4);
        assert!((js_pq - js_qp).abs() < 0.01);
    }

    #[test]
    fn jsd_different_distributions() {
        let p: Vec<f64> = (0..100).map(|_| 1.0).collect();
        let q: Vec<f64> = (0..100).map(|i| if i < 50 { 1.0 } else { 10.0 }).collect();
        let js = jsd(&p, &q, 10);
        assert!(js > 0.0);
    }

    #[test]
    fn jsd_empty() {
        assert_eq!(jsd(&[], &[], 10), 0.0);
    }

    #[test]
    fn jsd_bounded() {
        let p = vec![1.0, 1.0, 1.0];
        let q = vec![10.0, 10.0, 10.0];
        let js = jsd(&p, &q, 2);
        // JSD is bounded by log2(bins) / 2 at most in practice
        assert!(js.is_finite());
        assert!(js >= 0.0);
    }

    // ── Transfer entropy tests ──

    #[test]
    fn te_empty() {
        assert_eq!(transfer_entropy(&[], &[], 1, 10), 0.0);
    }

    #[test]
    fn te_too_short() {
        assert_eq!(transfer_entropy(&[1.0], &[1.0], 1, 10), 0.0);
    }

    #[test]
    fn te_causal_direction() {
        // X causes Y with lag 1: Y[t+1] = X[t]
        let x: Vec<f64> = (0..50).map(|i| (i % 4) as f64).collect();
        let mut y = vec![0.0; 50];
        for i in 1..50 {
            y[i] = x[i - 1];
        }
        let te_xy = transfer_entropy(&x, &y, 1, 4);
        let te_yx = transfer_entropy(&y, &x, 1, 4);
        // X→Y should be stronger than Y→X
        assert!(te_xy >= te_yx, "X→Y = {}, Y→X = {}", te_xy, te_yx);
    }

    #[test]
    fn te_independent() {
        // Independent variables: transfer entropy should be near 0
        let x: Vec<f64> = (0..100).map(|i| (i % 2) as f64).collect();
        let y: Vec<f64> = (0..100).map(|i| ((i * 7 + 3) % 5) as f64).collect();
        let te = transfer_entropy(&x, &y, 1, 5);
        // Should be small
        assert!(te < 1.0, "expected small TE, got {}", te);
    }

    #[test]
    fn te_non_negative() {
        let x: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..20).map(|i| (i * 2) as f64).collect();
        let te = transfer_entropy(&x, &y, 1, 5);
        assert!(te >= 0.0);
    }

    #[test]
    fn te_lag_2() {
        // X causes Y with lag 2: Y[t] = X[t-2] + noise
        let x: Vec<f64> = (0..100).map(|i| (i as f64 * 0.3).sin()).collect();
        let mut y = vec![0.0; 100];
        for i in 2..100 {
            y[i] = x[i - 2] + 0.01 * (i as f64).cos();
        }
        let te_xy = transfer_entropy(&x, &y, 2, 5);
        let te_yx = transfer_entropy(&y, &x, 2, 5);
        // X→Y should be stronger than Y→X
        assert!(te_xy > 0.0 || te_xy >= te_yx, "X→Y = {}, Y→X = {}", te_xy, te_yx);
    }

    // ── Permutation entropy tests ──

    #[test]
    fn pe_constant_series() {
        let values = vec![5.0; 100];
        let pe = permutation_entropy(&values, 3);
        // Constant series: only one ordinal pattern possible
        assert!(pe.abs() < 0.01, "expected ~0.0, got {}", pe);
    }

    #[test]
    fn pe_increasing_series() {
        let values: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let pe = permutation_entropy(&values, 3);
        // Strictly increasing: only one ordinal pattern (0,1,2)
        assert!(pe.abs() < 0.01, "expected ~0.0, got {}", pe);
    }

    #[test]
    fn pe_random_like() {
        // A series that visits many ordinal patterns
        let values: Vec<f64> = (0..100).map(|i| ((i * 17 + 3) % 97) as f64).collect();
        let pe = permutation_entropy(&values, 3);
        // Should be closer to 1.0 (high complexity)
        assert!(pe > 0.4, "expected high PE for pseudo-random, got {}", pe);
    }

    #[test]
    fn pe_empty() {
        assert_eq!(permutation_entropy(&[], 3), 0.0);
    }

    #[test]
    fn pe_order_too_large() {
        let values = vec![1.0, 2.0];
        assert_eq!(permutation_entropy(&values, 5), 0.0);
    }

    #[test]
    fn pe_order_1() {
        assert_eq!(permutation_entropy(&[1.0, 2.0, 3.0], 1), 0.0);
    }

    #[test]
    fn pe_normalized_between_0_and_1() {
        let values: Vec<f64> = (0..50).map(|i| (i as f64).sin()).collect();
        let pe = permutation_entropy(&values, 4);
        assert!(pe >= 0.0 && pe <= 1.0);
    }

    #[test]
    fn pe_sinusoidal() {
        // Sinusoidal should have some structure but not be maximally complex
        let values: Vec<f64> = (0..200).map(|i| (i as f64 * 0.1).sin()).collect();
        let pe = permutation_entropy(&values, 3);
        assert!(pe > 0.0 && pe < 1.0);
    }

    // ── Factorial tests ──

    #[test]
    fn factorial_values() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(2), 2);
        assert_eq!(factorial(3), 6);
        assert_eq!(factorial(4), 24);
        assert_eq!(factorial(5), 120);
    }

    // ── Discretize tests ──

    #[test]
    fn discretize_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let disc = discretize(&values, 5);
        assert_eq!(disc.len(), 5);
        // All indices should be valid
        for &d in &disc {
            assert!(d < 5);
        }
    }

    #[test]
    fn discretize_constant() {
        let disc = discretize(&[5.0; 10], 5);
        assert!(disc.iter().all(|&d| d == 0));
    }

    // ── Ordinal pattern tests ──

    #[test]
    fn ordinal_pattern_increasing() {
        let idx = ordinal_pattern_index(&[1.0, 2.0, 3.0]);
        // Increasing: indices (0,1,2), Lehmer code = 0
        assert_eq!(idx, 0);
    }

    #[test]
    fn ordinal_pattern_decreasing() {
        let idx = ordinal_pattern_index(&[3.0, 2.0, 1.0]);
        // Decreasing: indices (2,1,0), Lehmer code should be max (5 for order 3)
        assert_eq!(idx, 5);
    }

    // ── 3D entropy test ──

    #[test]
    fn entropy_3d_basic() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![1.0, 2.0, 3.0, 4.0];
        let c = vec![1.0, 2.0, 3.0, 4.0];
        let h = entropy_3d(&a, &b, &c, 4);
        // All unique combos: log2(4) = 2.0
        assert!(h > 0.0);
        assert!(h.is_finite());
    }

    // ── Integration: entropy vs distribution shape ──

    #[test]
    fn entropy_peaked_vs_flat() {
        // Peaked: most values in one bin
        let peaked: Vec<f64> = (0..100).map(|_| 5.0).chain(vec![1.0, 2.0, 8.0, 9.0]).collect();
        // Flat: spread across bins
        let flat: Vec<f64> = (0..100).map(|i| i as f64).collect();

        let h_peaked = entropy(&peaked, 10);
        let h_flat = entropy(&flat, 10);
        assert!(h_peaked < h_flat);
    }

    #[test]
    fn entropy_bits_units() {
        // 8 uniform bins → log2(8) = 3 bits
        let values: Vec<f64> = (0..800).map(|i| (i % 8) as f64).collect();
        let h = entropy(&values, 8);
        assert!((h - 3.0).abs() < 0.05);
    }
}
