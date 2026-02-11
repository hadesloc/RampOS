"""
Feature extraction matching Rust FraudFeatureExtractor.

The 17 features must match the FraudFeatureVector struct in
crates/ramp-compliance/src/fraud/features.rs exactly.
"""

import numpy as np

# Feature names in the exact order matching Rust FraudFeatureVector fields.
FEATURE_NAMES = [
    "amount_percentile",
    "velocity_1h",
    "velocity_24h",
    "velocity_7d",
    "time_of_day_anomaly",
    "amount_rounding_pattern",
    "recipient_recency",
    "historical_dispute_rate",
    "account_age_days",
    "amount_to_avg_ratio",
    "distinct_recipients_24h",
    "device_novelty",
    "country_risk",
    "is_cross_border",
    "amount_usd",
    "failed_txn_count_24h",
    "cumulative_amount_24h_usd",
]

NUM_FEATURES = len(FEATURE_NAMES)

# Normalization ranges for each feature: (min, max).
# Used to scale raw values into [0, 1] for the ML model.
FEATURE_RANGES = {
    "amount_percentile": (0.0, 1.0),
    "velocity_1h": (0.0, 50.0),
    "velocity_24h": (0.0, 200.0),
    "velocity_7d": (0.0, 500.0),
    "time_of_day_anomaly": (0.0, 1.0),
    "amount_rounding_pattern": (0.0, 1.0),
    "recipient_recency": (0.0, 1.0),
    "historical_dispute_rate": (0.0, 1.0),
    "account_age_days": (0.0, 3650.0),  # up to 10 years
    "amount_to_avg_ratio": (0.0, 100.0),
    "distinct_recipients_24h": (0.0, 100.0),
    "device_novelty": (0.0, 1.0),
    "country_risk": (0.0, 1.0),
    "is_cross_border": (0.0, 1.0),
    "amount_usd": (0.0, 1_000_000.0),
    "failed_txn_count_24h": (0.0, 50.0),
    "cumulative_amount_24h_usd": (0.0, 1_000_000.0),
}


def normalize_features(raw: np.ndarray) -> np.ndarray:
    """Normalize raw feature matrix to [0, 1] using predefined ranges.

    Args:
        raw: Array of shape (n_samples, NUM_FEATURES) with raw feature values.

    Returns:
        Normalized array of same shape with values clipped to [0, 1].
    """
    normalized = np.copy(raw).astype(np.float64)
    for i, name in enumerate(FEATURE_NAMES):
        lo, hi = FEATURE_RANGES[name]
        if hi - lo > 0:
            normalized[:, i] = (normalized[:, i] - lo) / (hi - lo)
    return np.clip(normalized, 0.0, 1.0)


def extract_feature_vector(row: dict) -> np.ndarray:
    """Extract a single feature vector from a dict (e.g. a CSV row).

    Args:
        row: Dictionary with keys matching FEATURE_NAMES.

    Returns:
        1-D numpy array of length NUM_FEATURES.
    """
    return np.array([float(row.get(name, 0.0)) for name in FEATURE_NAMES], dtype=np.float64)
