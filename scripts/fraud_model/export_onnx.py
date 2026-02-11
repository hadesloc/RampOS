"""
Export a trained sklearn model to ONNX format.

Usage:
    python -m fraud_model.export_onnx [--model PATH] [--output PATH]
"""

import argparse
from pathlib import Path

import numpy as np
import joblib

from .features import FEATURE_NAMES, NUM_FEATURES


def export_to_onnx(
    model,
    output_path: str | Path = "model.onnx",
    opset_version: int = 13,
) -> Path:
    """Export a sklearn model to ONNX format.

    Args:
        model: Trained sklearn classifier with predict/predict_proba.
        output_path: Where to save the .onnx file.
        opset_version: ONNX opset version.

    Returns:
        Path to the exported model.
    """
    from skl2onnx import convert_sklearn
    from skl2onnx.common.data_types import FloatTensorType

    initial_type = [("features", FloatTensorType([None, NUM_FEATURES]))]
    onnx_model = convert_sklearn(model, initial_types=initial_type, target_opset=opset_version)

    output_path = Path(output_path)
    with open(output_path, "wb") as f:
        f.write(onnx_model.SerializeToString())

    return output_path


def validate_onnx(
    onnx_path: str | Path,
    sample_input: np.ndarray | None = None,
) -> bool:
    """Validate the exported ONNX model by running inference.

    Args:
        onnx_path: Path to the .onnx file.
        sample_input: Optional test input of shape (1, NUM_FEATURES).

    Returns:
        True if validation passes.
    """
    import onnx
    import onnxruntime as ort

    # Structural validation
    model = onnx.load(str(onnx_path))
    onnx.checker.check_model(model)

    # Runtime validation
    sess = ort.InferenceSession(str(onnx_path))
    if sample_input is None:
        sample_input = np.random.rand(1, NUM_FEATURES).astype(np.float32)

    input_name = sess.get_inputs()[0].name
    result = sess.run(None, {input_name: sample_input})

    # Should return prediction label and probabilities
    assert len(result) >= 1, "ONNX model should return at least one output"
    return True


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Export model to ONNX")
    parser.add_argument("--model", type=str, default="model.joblib", help="Input sklearn model")
    parser.add_argument("--output", type=str, default="model.onnx", help="Output ONNX path")
    args = parser.parse_args(argv)

    print(f"Loading model from {args.model}")
    model = joblib.load(args.model)

    print(f"Exporting to ONNX: {args.output}")
    path = export_to_onnx(model, args.output)

    print("Validating ONNX model...")
    validate_onnx(path)
    print(f"ONNX model exported and validated: {path}")


if __name__ == "__main__":
    main()
