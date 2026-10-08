use serde::{Deserialize, Serialize};

/// A spectral measurement from a governed session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralMeasurement {
    /// Spectral radius upper bound (lambda_p). Must be < 1.0 for contractivity.
    pub lambda_p: f64,
    /// Lipschitz constant upper bound (L_p). Must be < 1.0 for contractivity.
    pub l_p: f64,
    /// Number of zero-spacing witnesses (must be >= 1).
    pub zero_spacings_count: usize,
    /// Phase coherence metric (0.0 - 1.0).
    pub phase_coherence: f64,
    /// Spectral entropy (nats). Lower indicates more ordered dynamics.
    pub spectral_entropy: f64,
}

/// Result of the stability witness check.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StabilityMetricResult {
    /// Harmonic stability metric `q` (0.0 - 1.0).
    pub q: f64,
    /// Whether the metric is above the required threshold (> 0.95).
    pub passes_threshold: bool,
    /// Whether the system satisfies the contractivity condition (lambda_p * L_p < 1.0).
    pub contractive: bool,
    /// Whether zero-spacing witnesses are present.
    pub has_witnesses: bool,
    /// Human-readable stability classification.
    pub classification: String,
    /// Breakdown of the harmonic stability computation.
    pub breakdown: StabilityBreakdown,
}

/// Detailed breakdown of the harmonic stability computation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StabilityBreakdown {
    /// Product lambda_p * L_p (must be < 1.0).
    pub contraction_product: f64,
    /// Contribution from spectral radius to stability.
    pub spectral_component: f64,
    /// Contribution from Lipschitz bound to stability.
    pub lipschitz_component: f64,
    /// Contribution from phase coherence.
    pub coherence_component: f64,
    /// Contribution from spectral entropy (lower entropy = higher stability).
    pub entropy_component: f64,
    /// Contribution from zero-spacing witnesses.
    pub witness_component: f64,
}

/// Classify the stability metric into a human-readable label.
fn classify_stability(q: f64, contractive: bool) -> String {
    if !contractive {
        return "UNSTABLE: non-contractive dynamics".to_string();
    }
    if q >= 0.99 {
        "STABLE: near-perfect harmonic alignment".to_string()
    } else if q >= 0.95 {
        "STABLE: within governance threshold".to_string()
    } else if q >= 0.90 {
        "MARGINAL: approaching governance threshold".to_string()
    } else if q >= 0.80 {
        "DEGRADED: below governance threshold".to_string()
    } else {
        "CRITICAL: severely degraded stability".to_string()
    }
}

/// Compute the harmonic stability metric from spectral measurements.
///
/// The harmonic stability metric `q` is a weighted combination of:
/// - Contractivity: lambda_p * L_p must be < 1.0 (weighted 40%)
/// - Phase coherence: higher coherence = higher stability (weighted 20%)
/// - Spectral entropy: lower entropy = more ordered = higher stability (weighted 20%)
/// - Zero-spacing witnesses: presence indicates genuine dynamics (weighted 10%)
/// - Lipschitz headroom: how far below 1.0 the product is (weighted 10%)
///
/// The final metric q is in [0.0, 1.0] where 1.0 is perfectly stable.
pub fn get_stability_metric(measurement: &SpectralMeasurement) -> StabilityMetricResult {
    // Contractivity check
    let contraction_product = measurement.lambda_p * measurement.l_p;
    let contractive = contraction_product < 1.0;

    // Spectral component: 1.0 - contraction_product (clamped to [0, 1])
    // Closer to 1.0 product means less headroom, lower score.
    let spectral_component = if contractive {
        (1.0 - contraction_product).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Lipschitz component: direct measure of L_p headroom
    let lipschitz_component = (1.0 - measurement.l_p).clamp(0.0, 1.0);

    // Coherence component: phase coherence is already [0, 1]
    let coherence_component = measurement.phase_coherence.clamp(0.0, 1.0);

    // Entropy component: lower entropy = more ordered = higher score.
    // Normalize against a reference entropy of 3.0 nats (moderate disorder).
    let entropy_component = (1.0 - (measurement.spectral_entropy / 3.0)).clamp(0.0, 1.0);

    // Witness component: binary presence, scaled by zero-spacing count
    let has_witnesses = measurement.zero_spacings_count >= 1;
    let witness_component = if has_witnesses {
        // More witnesses = slightly higher confidence, capped at 1.0
        (measurement.zero_spacings_count as f64 / 5.0).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Weighted combination
    let q = 0.40 * spectral_component
        + 0.20 * coherence_component
        + 0.20 * entropy_component
        + 0.10 * witness_component
        + 0.10 * lipschitz_component;

    // Clamp final metric
    let q = q.clamp(0.0, 1.0);

    let passes_threshold = q > 0.95;
    let classification = classify_stability(q, contractive);

    StabilityMetricResult {
        q,
        passes_threshold,
        contractive,
        has_witnesses,
        classification,
        breakdown: StabilityBreakdown {
            contraction_product,
            spectral_component,
            lipschitz_component,
            coherence_component,
            entropy_component,
            witness_component,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_system() {
        let m = SpectralMeasurement {
            lambda_p: 0.10,
            l_p: 0.10,
            zero_spacings_count: 5,
            phase_coherence: 0.99,
            spectral_entropy: 0.3,
        };
        let result = get_stability_metric(&m);
        assert!(result.contractive);
        assert!(result.has_witnesses);
        assert!(result.passes_threshold);
        assert!(result.q > 0.95);
    }

    #[test]
    fn test_unstable_system() {
        let m = SpectralMeasurement {
            lambda_p: 0.99,
            l_p: 0.99,
            zero_spacings_count: 0,
            phase_coherence: 0.3,
            spectral_entropy: 2.5,
        };
        let result = get_stability_metric(&m);
        assert!(result.contractive); // 0.99 * 0.99 = 0.9801 < 1.0, still contractive
        assert!(!result.passes_threshold); // but below governance threshold
    }

    #[test]
    fn test_boundary_case() {
        let m = SpectralMeasurement {
            lambda_p: 0.999,
            l_p: 0.999,
            zero_spacings_count: 1,
            phase_coherence: 0.99,
            spectral_entropy: 0.1,
        };
        let result = get_stability_metric(&m);
        // 0.999 * 0.999 = 0.998001 < 1.0, so contractive
        assert!(result.contractive);
    }
}
