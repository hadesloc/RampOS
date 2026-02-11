"""
Main training script for the fraud detection model.

Usage:
    python -m fraud_model.train [--data PATH] [--output PATH] [--n-estimators N]
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from sklearn.ensemble import GradientBoostingClassifier
from sklearn.model_selection import cross_val_score
from sklearn.metrics import classification_report
import joblib

from .data_loader import generate_synthetic_data, load_csv, split_data
from .features import normalize_features, FEATURE_NAMES


def train_model(
    X_train: np.ndarray,
    y_train: np.ndarray,
    n_estimators: int = 200,
    max_depth: int = 5,
    learning_rate: float = 0.1,
    seed: int = 42,
) -> GradientBoostingClassifier:
    """Train a GradientBoosting classifier.

    Args:
        X_train: Normalized training features.
        y_train: Training labels (0=legit, 1=fraud).
        n_estimators: Number of boosting stages.
        max_depth: Maximum tree depth.
        learning_rate: Shrinkage factor.
        seed: Random seed.

    Returns:
        Trained classifier.
    """
    model = GradientBoostingClassifier(
        n_estimators=n_estimators,
        max_depth=max_depth,
        learning_rate=learning_rate,
        random_state=seed,
        subsample=0.8,
    )
    model.fit(X_train, y_train)
    return model


def run_cross_validation(
    model: GradientBoostingClassifier,
    X: np.ndarray,
    y: np.ndarray,
    cv: int = 5,
) -> np.ndarray:
    """Run stratified k-fold cross-validation.

    Returns:
        Array of AUC-ROC scores for each fold.
    """
    scores = cross_val_score(model, X, y, cv=cv, scoring="roc_auc")
    return scores


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Train fraud detection model")
    parser.add_argument("--data", type=str, default=None, help="Path to CSV training data")
    parser.add_argument("--output", type=str, default="model.joblib", help="Output model path")
    parser.add_argument("--n-estimators", type=int, default=200, help="Number of estimators")
    parser.add_argument("--synthetic-samples", type=int, default=10_000, help="Synthetic sample count")
    args = parser.parse_args(argv)

    # Load or generate data
    if args.data:
        print(f"Loading data from {args.data}")
        X, y = load_csv(args.data)
    else:
        print(f"Generating {args.synthetic_samples} synthetic samples")
        X, y = generate_synthetic_data(n_samples=args.synthetic_samples)

    # Normalize
    X_norm = normalize_features(X)

    # Split
    X_train, X_test, y_train, y_test = split_data(X_norm, y)

    print(f"Training set: {len(X_train)} samples ({y_train.sum()} fraud)")
    print(f"Test set: {len(X_test)} samples ({y_test.sum()} fraud)")

    # Cross-validate
    model = GradientBoostingClassifier(
        n_estimators=args.n_estimators,
        max_depth=5,
        learning_rate=0.1,
        random_state=42,
        subsample=0.8,
    )
    cv_scores = run_cross_validation(model, X_train, y_train)
    print(f"\nCross-validation AUC-ROC: {cv_scores.mean():.4f} (+/- {cv_scores.std():.4f})")

    # Train final model
    model = train_model(X_train, y_train, n_estimators=args.n_estimators)

    # Evaluate
    y_pred = model.predict(X_test)
    print("\nClassification Report:")
    print(classification_report(y_test, y_pred, target_names=["legit", "fraud"]))

    # Feature importance
    importances = model.feature_importances_
    sorted_idx = np.argsort(importances)[::-1]
    print("Top 10 Feature Importances:")
    for i in sorted_idx[:10]:
        print(f"  {FEATURE_NAMES[i]:30s} {importances[i]:.4f}")

    # Save
    output_path = Path(args.output)
    joblib.dump(model, output_path)
    print(f"\nModel saved to {output_path}")


if __name__ == "__main__":
    main()
