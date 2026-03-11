# RampOS CLI

RampOS now exposes an installable Python-backed CLI surface for operator and agent workflows.

## Entry Points

- Packaged entrypoint: `rampos`
- Repo compatibility shim: `python scripts/rampos-cli.py`

## Auth Modes

- `api`: `--api-key`, optional `--api-secret`
- `admin`: `--admin-key`, optional `--admin-role`, `--admin-user-id`
- `portal`: `--portal-token`
- `lp`: `--lp-key`

All command families also support `--profile`, `--base-url`, `--tenant-id`, `--output`, `--body`, `--body-file`, `--body-stdin`, `--request-id`, and `--idempotency-key`.

## Current Command Families

- `intents`
- `users`
- `ledger`
- `chain`
- `rfq`
- `lp rfq`
- `swap`
- `bridge`
- `licensing`
- `sandbox`
- `reconciliation`
- `treasury`

## Examples

```bash
python scripts/rampos-cli.py intents create-payin \
  --auth-mode api \
  --api-key test-key \
  --body '{"tenantId":"tenant_123","userId":"user_123","amountVnd":1000000,"railsProvider":"VIETQR"}'
```

```bash
python scripts/rampos-cli.py rfq list-open \
  --auth-mode admin \
  --admin-key admin_test_key
```

```bash
python scripts/rampos-cli.py lp rfq bid \
  --auth-mode lp \
  --lp-key lp_123:tenant_123:secret \
  --rfq-id rfq_123 \
  --body '{"exchangeRate":"25300","vndAmount":"25300000"}'
```

```bash
python scripts/rampos-cli.py licensing upload \
  --auth-mode admin \
  --admin-key admin_test_key \
  --body-file payload.json
```
