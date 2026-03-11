# RampOS CLI for Agents

The CLI is designed to be machine-first:

- JSON output is available on every command
- auth can be supplied via flags, env vars, or named profiles
- requests can be passed inline, from files, or from stdin
- path parameters become explicit flags such as `--rfq-id`

## Recommended Patterns

Use stdin for generated payloads:

```bash
echo '{"tenantId":"tenant_123","userId":"user_123","amountVnd":1000000,"railsProvider":"VIETQR"}' | \
python scripts/rampos-cli.py intents create-payin \
  --auth-mode api \
  --api-key test-key \
  --body-stdin
```

Use named profiles for repeated workflows:

```bash
python scripts/rampos-cli.py login \
  --profile ops \
  --base-url https://api.rampos.io \
  --auth-mode admin \
  --admin-key admin_test_key
```

```bash
python scripts/rampos-cli.py rfq list-open --profile ops
```

Use help as the source of truth for argument shapes:

```bash
python scripts/rampos-cli.py bridge routes --help
python scripts/rampos-cli.py licensing upload --help
python scripts/rampos-cli.py lp rfq bid --help
```

## Good Fits

- operator queue inspection
- RFQ lifecycle management
- LP bid submission
- licensing compatibility workflows
- chain quote / bridge route queries
- smoke checks in CI

## Current Caveats

- Some command coverage comes from a curated manifest because not every live route is represented in OpenAPI yet.
- `licensing upload` currently reflects a compatibility storage path, not a hardened document pipeline.
- RFQ finalize behavior mirrors current backend semantics and does not add transactional guarantees on top of the service.
