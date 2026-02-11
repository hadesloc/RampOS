# Fraud Detection Model Training Pipeline

ML training pipeline for the RampOS fraud detection system. Produces an ONNX model
compatible with the Rust `OnnxModelScorer` in `crates/ramp-compliance/src/fraud/scorer.rs`.

## Features

The model uses 17 features matching the Rust `FraudFeatureVector`:

| # | Feature | Range |
|---|---------|-------|
| 1 | amount_percentile | 0-1 |
| 2 | velocity_1h | 0-50 |
| 3 | velocity_24h | 0-200 |
| 4 | velocity_7d | 0-500 |
| 5 | time_of_day_anomaly | 0-1 |
| 6 | amount_rounding_pattern | 0-1 |
| 7 | recipient_recency | 0-1 |
| 8 | historical_dispute_rate | 0-1 |
| 9 | account_age_days | 0-3650 |
| 10 | amount_to_avg_ratio | 0-100 |
| 11 | distinct_recipients_24h | 0-100 |
| 12 | device_novelty | 0-1 |
| 13 | country_risk | 0-1 |
| 14 | is_cross_border | 0-1 |
| 15 | amount_usd | 0-1M |
| 16 | failed_txn_count_24h | 0-50 |
| 17 | cumulative_amount_24h_usd | 0-1M |

## Setup

```bash
cd scripts/fraud_model
pip install -r requirements.txt
```

## Usage

### Train with synthetic data
```bash
python -m fraud_model.train --synthetic-samples 10000
```

### Train with real data
```bash
python -m fraud_model.train --data training_data.csv
```

### Export to ONNX
```bash
python -m fraud_model.export_onnx --model model.joblib --output model.onnx
```

### Evaluate
```bash
python -m fraud_model.evaluate --model model.joblib
```

### Run tests
```bash
python -m pytest tests/ -v
```

## CSV Format

Training CSV files must include all 17 feature columns plus a `label` column:
- `label=0`: legitimate transaction
- `label=1`: fraudulent transaction
