# Release Evidence Directory

This folder stores attributable artifacts for release hardening, staging validation, and recovery drills.

Recommended layout:

- `<timestamp>/summary.json`
- `<timestamp>/summary.md`
- `<timestamp>/*.stdout.txt`
- `<timestamp>/*.stderr.txt`

Guidelines:

- create one evidence directory per rehearsal or candidate run
- keep the candidate SHA in the checklist or ledger that points to the directory
- do not overwrite evidence from a prior run; create a new timestamped folder instead
- store exports, screenshots, or links next to the summary when required by the staging or DR plans
