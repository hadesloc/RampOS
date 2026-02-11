"""
Evaluate a trained fraud detection model.

Usage:
    python -m fraud_model.evaluate [--model PATH] [--data PATH]
"""

import argparse
from pathlib import Path

import numpy as np
import joblib
from sklearn.metrics import (
    classification_report,
    roc_auc_score,
    precision_score,
    recall_score,
    f1_score,
    confusion_matrix,
)

from .data_loader import generate_synthetic_data, load_csv, split_data
from .features import normalize_features


def evaluate_model(
    model,
    X_test: np.ndarray,
    y_test: np.ndarray,
) -> dict:
    """Evaluate model and return metrics.

    Returns:
        Dict with precision, recall, f1, auc_roc, and confusion_matrix.
    """
    y_pred = model.predict(X_test)
    y_proba = model.predict_proba(X_test)[:, 1]

    return {
        "precision": float(precision_score(y_test, y_pred, zero_division=0)),
        "recall": float(recall_score(y_test, y_pred, zero_division=0)),
        "f1": float(f1_score(y_test, y_pred, zero_division=0)),
        "auc_roc": float(roc_auc_score(y_test, y_proba)),
        "confusion_matrix": confusion_matrix(y_test, y_pred).tolist(),
        "report": classification_report(y_test, y_pred, target_names=["legit", "fraud"]),
    }


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Evaluate fraud model")
    parser.add_argument("--model", type=str, default="model.joblib", help="Model path")
    parser.add_argument("--data", type=str, default=None, help="Test data CSV")
    args = parser.parse_args(argv)

    model = joblib.load(args.model)

    if args.data:
        X, y = load_csv(args.data)
    else:
        X, y = generate_synthetic_data(n_samples=5_000, seed=99)

    X_norm = normalize_features(X)
    _, X_test, _, y_test = split_data(X_norm, y, seed=99)

    metrics = evaluate_model(model, X_test, y_test)

    print("Evaluation Results:")
    print(f"  Precision: {metrics['precision']:.4f}")
    print(f"  Recall:    {metrics['recall']:.4f}")
    print(f"  F1:        {metrics['f1']:.4f}")
    print(f"  AUC-ROC:   {metrics['auc_roc']:.4f}")
    print(f"\nConfusion Matrix:\n  {metrics['confusion_matrix']}")
    print(f"\n{metrics['report']}")


if __name__ == "__main__":
    main()
