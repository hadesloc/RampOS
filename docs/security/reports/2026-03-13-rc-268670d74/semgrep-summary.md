# Semgrep Summary

- Release candidate: `268670d74`
- Tool: `semgrep 1.149.0`
- Command shape: `semgrep scan --metrics=off --config p/rust --config p/secrets --json`
- Result: `10` findings total

## Triage Summary

- `7` findings are on docs or evidence artifacts, primarily secret-pattern matches against committed recovery or evidence material rather than live application code.
- `3` findings are on non-doc files:
  - `crates/ramp-compliance/src/providers/factory.rs`
    - `temp_dir` audit finding twice
    - severity `INFO`
  - `crates/ramp-core/src/sso/saml.rs`
    - insecure hash audit finding for `sha1`
    - severity `WARNING`

## Current Assessment

- No new Semgrep finding in this run is being treated as a bank-grade blocking code execution issue by itself.
- The `sha1` SAML finding still requires explicit engineering or security triage because protocol compatibility code often needs a bounded justification rather than silent acceptance.
- The `temp_dir` findings are audit-level hardening items and should be reviewed, but they are not currently signoff-blocking compared with the open dependency and external-review blockers.
