"""
Tests for the fraud model training pipeline.
"""

import sys
import os
import tempfile
from pathlib import Path

import numpy as np
import pytest

# Ensure the scripts directory is on the path so we can import fraud_model
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from fraud_model.features import (
    FEATURE_NAMES,
    NUM_FEATURES,
    FEATURE_RANGES,
    normalize_features,
    extract_feature_vector,
)
from fraud_model.data_loader import (
    generate_synthetic_data,
    split_data,
)
from fraud_model.train import train_model
from fraud_model.evaluate import evaluate_model


# ──────────────────────────────────────────────────────────────
# Feature Tests
# ──────────────────────────────────────────────────────────────

class TestFeatures:
    def test_feature_count(self):
        """NUM_FEATURES must be 17, matching Rust FraudFeatureVector."""
        assert NUM_FEATURES == 17

    def test_feature_names_match_rust(self):
        """Feature names must match the Rust FraudFeatureVector field order."""
        expected = [
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
        assert FEATURE_NAMES == expected

    def test_feature_ranges_complete(self):
        """Every feature must have a normalization range defined."""
        for name in FEATURE_NAMES:
            assert name in FEATURE_RANGES, f"Missing range for {name}"
            lo, hi = FEATURE_RANGES[name]
            assert hi > lo, f"Invalid range for {name}: ({lo}, {hi})"

    def test_normalize_features_shape(self):
        """Normalization must preserve shape."""
        raw = np.random.rand(10, NUM_FEATURES) * 1000
        norm = normalize_features(raw)
        assert norm.shape == (10, NUM_FEATURES)

    def test_normalize_features_range(self):
        """Normalized values must be in [0, 1]."""
        raw = np.random.rand(50, NUM_FEATURES) * 10_000
        norm = normalize_features(raw)
        assert np.all(norm >= 0.0)
        assert np.all(norm <= 1.0)

    def test_normalize_zeros(self):
        """Normalizing all-zero input should produce all zeros (features have min=0)."""
        raw = np.zeros((5, NUM_FEATURES))
        norm = normalize_features(raw)
        assert np.allclose(norm, 0.0)

    def test_extract_feature_vector_basic(self):
        """Extract a vector from a dict with all features."""
        row = {name: float(i) for i, name in enumerate(FEATURE_NAMES)}
        vec = extract_feature_vector(row)
        assert vec.shape == (NUM_FEATURES,)
        for i, name in enumerate(FEATURE_NAMES):
            assert vec[i] == float(i)

    def test_extract_feature_vector_missing_key(self):
        """Missing keys should default to 0.0."""
        row = {"amount_percentile": 0.5}
        vec = extract_feature_vector(row)
        assert vec[0] == 0.5
        assert vec[1] == 0.0  # velocity_1h defaults to 0


# ──────────────────────────────────────────────────────────────
# Data Loader Tests
# ──────────────────────────────────────────────────────────────

class TestDataLoader:
    def test_synthetic_data_shape(self):
        """Synthetic data must have correct shape."""
        X, y = generate_synthetic_data(n_samples=500, fraud_ratio=0.1)
        assert X.shape == (500, NUM_FEATURES)
        assert y.shape == (500,)

    def test_synthetic_data_labels(self):
        """Labels must be 0 or 1."""
        X, y = generate_synthetic_data(n_samples=1000)
        assert set(np.unique(y)) == {0, 1}

    def test_synthetic_data_fraud_ratio(self):
        """Fraud ratio should be approximately correct."""
        X, y = generate_synthetic_data(n_samples=10_000, fraud_ratio=0.05)
        actual_ratio = y.sum() / len(y)
        assert abs(actual_ratio - 0.05) < 0.01

    def test_synthetic_data_reproducibility(self):
        """Same seed should produce same data."""
        X1, y1 = generate_synthetic_data(n_samples=100, seed=123)
        X2, y2 = generate_synthetic_data(n_samples=100, seed=123)
        np.testing.assert_array_equal(X1, X2)
        np.testing.assert_array_equal(y1, y2)

    def test_synthetic_data_different_seeds(self):
        """Different seeds should produce different data."""
        X1, y1 = generate_synthetic_data(n_samples=100, seed=1)
        X2, y2 = generate_synthetic_data(n_samples=100, seed=2)
        assert not np.array_equal(X1, X2)

    def test_synthetic_data_schema(self):
        """Synthetic data features should be non-negative."""
        X, y = generate_synthetic_data(n_samples=1000)
        assert np.all(X >= 0), "All features should be non-negative"

    def test_split_data_sizes(self):
        """Train/test split should have correct proportions."""
        X, y = generate_synthetic_data(n_samples=1000)
        X_train, X_test, y_train, y_test = split_data(X, y, test_ratio=0.2)
        assert len(X_train) == 800
        assert len(X_test) == 200
        assert len(y_train) == 800
        assert len(y_test) == 200

    def test_split_data_stratification(self):
        """Split should maintain approximately the same fraud ratio."""
        X, y = generate_synthetic_data(n_samples=2000, fraud_ratio=0.1)
        X_train, X_test, y_train, y_test = split_data(X, y, test_ratio=0.2)
        train_ratio = y_train.sum() / len(y_train)
        test_ratio = y_test.sum() / len(y_test)
        assert abs(train_ratio - test_ratio) < 0.02


# ──────────────────────────────────────────────────────────────
# Model Training Tests
# ──────────────────────────────────────────────────────────────

class TestModelTraining:
    @pytest.fixture
    def training_data(self):
        X, y = generate_synthetic_data(n_samples=2000, fraud_ratio=0.1, seed=42)
        X_norm = normalize_features(X)
        return split_data(X_norm, y, seed=42)

    def test_model_trains(self, training_data):
        """Model should train without errors."""
        X_train, X_test, y_train, y_test = training_data
        model = train_model(X_train, y_train, n_estimators=50)
        assert model is not None
        assert hasattr(model, "predict")
        assert hasattr(model, "predict_proba")

    def test_model_predicts(self, training_data):
        """Model should produce predictions with correct shape."""
        X_train, X_test, y_train, y_test = training_data
        model = train_model(X_train, y_train, n_estimators=50)
        preds = model.predict(X_test)
        assert preds.shape == y_test.shape
        assert set(np.unique(preds)).issubset({0, 1})

    def test_model_probabilities(self, training_data):
        """Probabilities should sum to 1 and be in [0,1]."""
        X_train, X_test, y_train, y_test = training_data
        model = train_model(X_train, y_train, n_estimators=50)
        proba = model.predict_proba(X_test)
        assert proba.shape == (len(X_test), 2)
        np.testing.assert_allclose(proba.sum(axis=1), 1.0, atol=1e-6)
        assert np.all(proba >= 0.0)
        assert np.all(proba <= 1.0)

    def test_model_better_than_random(self, training_data):
        """Model AUC-ROC should be significantly above 0.5."""
        from sklearn.metrics import roc_auc_score

        X_train, X_test, y_train, y_test = training_data
        model = train_model(X_train, y_train, n_estimators=100)
        y_proba = model.predict_proba(X_test)[:, 1]
        auc = roc_auc_score(y_test, y_proba)
        assert auc > 0.7, f"AUC-ROC {auc:.3f} is too low"


# ──────────────────────────────────────────────────────────────
# ONNX Export Tests
# ──────────────────────────────────────────────────────────────

class TestOnnxExport:
    @pytest.fixture
    def trained_model(self):
        X, y = generate_synthetic_data(n_samples=1000, seed=42)
        X_norm = normalize_features(X)
        X_train, _, y_train, _ = split_data(X_norm, y, seed=42)
        return train_model(X_train, y_train, n_estimators=50)

    def test_export_creates_file(self, trained_model):
        """ONNX export should create a file."""
        from fraud_model.export_onnx import export_to_onnx

        tmp = tempfile.mktemp(suffix=".onnx")
        try:
            path = export_to_onnx(trained_model, tmp)
            assert Path(path).exists()
            assert Path(path).stat().st_size > 0
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)

    def test_onnx_validates(self, trained_model):
        """Exported ONNX model should pass validation."""
        from fraud_model.export_onnx import export_to_onnx, validate_onnx

        tmp = tempfile.mktemp(suffix=".onnx")
        try:
            path = export_to_onnx(trained_model, tmp)
            assert validate_onnx(path)
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)

    def test_onnx_inference_matches_sklearn(self, trained_model):
        """ONNX predictions should match sklearn predictions."""
        import onnxruntime as ort
        from fraud_model.export_onnx import export_to_onnx

        sample = np.random.rand(5, NUM_FEATURES).astype(np.float32)
        sklearn_preds = trained_model.predict(sample)

        tmp = tempfile.mktemp(suffix=".onnx")
        try:
            export_to_onnx(trained_model, tmp)
            sess = ort.InferenceSession(tmp)
            input_name = sess.get_inputs()[0].name
            onnx_result = sess.run(None, {input_name: sample})
            onnx_preds = onnx_result[0]
            np.testing.assert_array_equal(sklearn_preds, onnx_preds)
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)


# ──────────────────────────────────────────────────────────────
# Evaluation Tests
# ──────────────────────────────────────────────────────────────

class TestEvaluation:
    def test_evaluation_metrics_valid(self):
        """All evaluation metrics should be in valid range [0, 1]."""
        X, y = generate_synthetic_data(n_samples=2000, seed=42)
        X_norm = normalize_features(X)
        X_train, X_test, y_train, y_test = split_data(X_norm, y, seed=42)
        model = train_model(X_train, y_train, n_estimators=50)

        metrics = evaluate_model(model, X_test, y_test)

        assert 0.0 <= metrics["precision"] <= 1.0
        assert 0.0 <= metrics["recall"] <= 1.0
        assert 0.0 <= metrics["f1"] <= 1.0
        assert 0.0 <= metrics["auc_roc"] <= 1.0

    def test_evaluation_confusion_matrix(self):
        """Confusion matrix should have correct shape."""
        X, y = generate_synthetic_data(n_samples=1000, seed=42)
        X_norm = normalize_features(X)
        X_train, X_test, y_train, y_test = split_data(X_norm, y, seed=42)
        model = train_model(X_train, y_train, n_estimators=50)

        metrics = evaluate_model(model, X_test, y_test)
        cm = metrics["confusion_matrix"]
        assert len(cm) == 2
        assert len(cm[0]) == 2
        # Sum of confusion matrix should equal test set size
        assert sum(sum(row) for row in cm) == len(y_test)

    def test_evaluation_report_string(self):
        """Classification report should be a non-empty string."""
        X, y = generate_synthetic_data(n_samples=500, seed=42)
        X_norm = normalize_features(X)
        X_train, X_test, y_train, y_test = split_data(X_norm, y, seed=42)
        model = train_model(X_train, y_train, n_estimators=50)

        metrics = evaluate_model(model, X_test, y_test)
        assert isinstance(metrics["report"], str)
        assert len(metrics["report"]) > 50
        assert "legit" in metrics["report"]
        assert "fraud" in metrics["report"]
