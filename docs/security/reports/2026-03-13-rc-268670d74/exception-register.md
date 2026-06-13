# Exception Register

Release candidate: `268670d74`

There are no approved risk acceptances or waivers recorded in this preserved RC evidence set.
If a newer review is performed later, record exceptions against that newer review window instead of extending this file with old process scaffolding.

## 2026-06-12 addendum — `rsa` advisory posture update

This addendum corrects stale follow-on wording that described `rsa 0.9.10` as only transitive through SQLx with no active source imports and the Napas runtime on `ring`. Current source has `rsa` as a direct `ramp-adapter` dependency and the Napas adapter imports it for RSA PKCS#1 v1.5 SHA-256 signing and verification.

`RUSTSEC-2023-0071` covers the Marvin timing side-channel against RSA PKCS#1 v1.5 decryption. The current Napas adapter does not expose an RSA decryption operation or decryption oracle; it signs outbound payloads and verifies inbound signatures only, with signing performed through the crate's randomized signing API. This remains an advisory-tracked dependency exception, not evidence of an exposed Marvin decryption vector in the Napas runtime path.
