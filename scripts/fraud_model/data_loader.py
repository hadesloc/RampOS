"""
Data loading and synthetic data generation for fraud model training.
"""

import numpy as np
import pandas as pd
from pathlib import Path

from .features import FEATURE_NAMES, NUM_FEATURES


def load_csv(path: str | Path) -> tuple[np.ndarray, np.ndarray]:
    """Load training data from a CSV file.

    Expects columns matching FEATURE_NAMES plus a 'label' column (0 or 1).

    Returns:
        Tuple of (X, y) where X has shape (n, NUM_FEATURES) and y has shape (n,).
    """
    df = pd.read_csv(path)
    X = df[FEATURE_NAMES].values.astype(np.float64)
    y = df["label"].values.astype(np.int32)
    return X, y


def generate_synthetic_data(
    n_samples: int = 10_000,
    fraud_ratio: float = 0.05,
    seed: int = 42,
) -> tuple[np.ndarray, np.ndarray]:
    """Generate synthetic transaction data for bootstrapping.

    Legitimate transactions have low-risk feature distributions.
    Fraudulent transactions have elevated risk indicators.

    Args:
        n_samples: Total number of samples.
        fraud_ratio: Fraction of samples that are fraudulent.
        seed: Random seed for reproducibility.

    Returns:
        Tuple of (X, y).
    """
    rng = np.random.RandomState(seed)
    n_fraud = int(n_samples * fraud_ratio)
    n_legit = n_samples - n_fraud

    legit = _generate_legit(rng, n_legit)
    fraud = _generate_fraud(rng, n_fraud)

    X = np.vstack([legit, fraud])
    y = np.concatenate([np.zeros(n_legit, dtype=np.int32), np.ones(n_fraud, dtype=np.int32)])

    # Shuffle
    idx = rng.permutation(n_samples)
    return X[idx], y[idx]


def _generate_legit(rng: np.random.RandomState, n: int) -> np.ndarray:
    """Generate legitimate transaction features."""
    data = np.zeros((n, NUM_FEATURES), dtype=np.float64)

    data[:, 0] = rng.beta(2, 5, n)                          # amount_percentile: skewed low
    data[:, 1] = rng.poisson(1.0, n).clip(0, 10)            # velocity_1h
    data[:, 2] = rng.poisson(3.0, n).clip(0, 30)            # velocity_24h
    data[:, 3] = rng.poisson(10.0, n).clip(0, 80)           # velocity_7d
    data[:, 4] = rng.beta(1, 10, n)                          # time_of_day_anomaly: usually normal
    data[:, 5] = rng.choice([0.0, 0.2, 0.4], n, p=[0.7, 0.2, 0.1])  # amount_rounding_pattern
    data[:, 6] = rng.beta(1, 5, n)                           # recipient_recency: mostly known
    data[:, 7] = rng.beta(1, 100, n)                         # historical_dispute_rate: very low
    data[:, 8] = rng.exponential(180, n).clip(7, 3650)       # account_age_days: mostly old
    data[:, 9] = rng.lognormal(0, 0.3, n).clip(0.1, 5)      # amount_to_avg_ratio: near 1x
    data[:, 10] = rng.poisson(1.5, n).clip(1, 10)            # distinct_recipients_24h
    data[:, 11] = rng.choice([0.0, 1.0], n, p=[0.9, 0.1])   # device_novelty: mostly known
    data[:, 12] = rng.beta(1, 10, n)                         # country_risk: low
    data[:, 13] = rng.choice([0.0, 1.0], n, p=[0.8, 0.2])   # is_cross_border
    data[:, 14] = rng.lognormal(4, 1, n).clip(1, 50_000)    # amount_usd
    data[:, 15] = rng.poisson(0.2, n).clip(0, 5)             # failed_txn_count_24h
    data[:, 16] = rng.lognormal(5, 1, n).clip(1, 100_000)   # cumulative_amount_24h_usd

    return data


def _generate_fraud(rng: np.random.RandomState, n: int) -> np.ndarray:
    """Generate fraudulent transaction features with elevated risk signals."""
    data = np.zeros((n, NUM_FEATURES), dtype=np.float64)

    data[:, 0] = rng.beta(5, 2, n)                           # amount_percentile: high
    data[:, 1] = rng.poisson(6.0, n).clip(2, 50)             # velocity_1h: elevated
    data[:, 2] = rng.poisson(15.0, n).clip(5, 100)           # velocity_24h: elevated
    data[:, 3] = rng.poisson(30.0, n).clip(10, 200)          # velocity_7d: elevated
    data[:, 4] = rng.beta(3, 2, n)                            # time_of_day_anomaly: often unusual
    data[:, 5] = rng.choice([0.0, 0.4, 0.6, 0.8, 1.0], n, p=[0.2, 0.2, 0.2, 0.2, 0.2])
    data[:, 6] = rng.beta(5, 2, n)                            # recipient_recency: often new
    data[:, 7] = rng.beta(3, 20, n)                           # historical_dispute_rate: higher
    data[:, 8] = rng.exponential(10, n).clip(0, 30)           # account_age_days: often new
    data[:, 9] = rng.lognormal(1.5, 0.5, n).clip(2, 50)      # amount_to_avg_ratio: high
    data[:, 10] = rng.poisson(5.0, n).clip(2, 30)             # distinct_recipients_24h: many
    data[:, 11] = rng.choice([0.0, 1.0], n, p=[0.3, 0.7])    # device_novelty: often new
    data[:, 12] = rng.beta(3, 3, n)                           # country_risk: moderate-high
    data[:, 13] = rng.choice([0.0, 1.0], n, p=[0.4, 0.6])    # is_cross_border: often
    data[:, 14] = rng.lognormal(7, 1.5, n).clip(500, 500_000)  # amount_usd: high
    data[:, 15] = rng.poisson(3.0, n).clip(1, 20)             # failed_txn_count_24h: elevated
    data[:, 16] = rng.lognormal(8, 1, n).clip(5_000, 500_000)  # cumulative_amount_24h_usd: high

    return data


def split_data(
    X: np.ndarray,
    y: np.ndarray,
    test_ratio: float = 0.2,
    seed: int = 42,
) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    """Split data into train and test sets with stratification.

    Returns:
        (X_train, X_test, y_train, y_test)
    """
    from sklearn.model_selection import train_test_split

    return train_test_split(X, y, test_size=test_ratio, random_state=seed, stratify=y)
