# OWASP Pre-Release Checklist

Based on OWASP Application Security Verification Standard (ASVS) and Top 10.

## A. Authentication & Session Management (A07:2021-Identification and Authentication Failures)
- [x] **Password Policy:** Enforce strong passwords (length, complexity) and prevent credential stuffing.
- [x] **MFA:** Multi-Factor Authentication is implemented for sensitive accounts.
- [x] **Session Timeout:** Sessions expire after a period of inactivity.
- [x] **Secure Cookies:** Cookies are marked `Secure`, `HttpOnly`, and `SameSite`.
- [x] **Logout:** Logout functionality terminates the session on both client and server.

## B. Access Control (A01:2021-Broken Access Control)
- [x] **Principle of Least Privilege:** Users only have access to what they need.
- [x] **IDOR Protection:** Verify user ownership before accessing objects by ID (Tenant Isolation confirmed).
- [x] **Role Validation:** Server-side checks for user roles on every privileged request.
- [x] **Directory Browsing:** Disabled on the web server.

## C. Input Validation & Injection Prevention (A03:2021-Injection)
- [x] **SQL Injection:** Use Prepared Statements / Parameterized Queries for all database interactions (Checked: sqlx usage).
- [x] **XSS Prevention:** Output encoding/escaping is applied to all user-supplied data displayed in the browser.
- [x] **Input Sanitization:** Validate all inputs against a whitelist of allowed characters/formats (Checked: Validator crate).
- [x] **Command Injection:** Avoid `eval()` and system commands with user input.

## D. Cryptography & Data Protection (A02:2021-Cryptographic Failures)
- [x] **Transport Layer Security:** HTTPS is enforced (HSTS) with strong cipher suites.
- [x] **Data at Rest:** Sensitive data (PII, secrets) is encrypted in the database.
- [x] **Secrets Management:** API keys and passwords are NOT hardcoded; use environment variables or secret managers (Checked: .env usage).
- [x] **Hashing:** Passwords are hashed using strong algorithms (Argon2, bcrypt, scrypt) (Checked: Argon2).

## E. Security Misconfiguration (A05:2021-Security Misconfiguration)
- [x] **Debug Mode:** Disabled in production.
- [x] **Error Handling:** Generic error messages are shown to users (no stack traces).
- [x] **Default Credentials:** All default passwords changed.
- [x] **Headers:** Security headers configured (CSP, X-Frame-Options, X-Content-Type-Options).

## F. Vulnerable and Outdated Components (A06:2021-Vulnerable and Outdated Components)
- [x] **Dependency Scan:** Run `npm audit` / `pip audit` / `cargo audit` to find vulnerabilities (Checked: Cargo audit passed).
- [x] **Updates:** All libraries and frameworks are updated to stable, secure versions.
- [x] **Unused Dependencies:** Remove unused libraries.

## G. Logging & Monitoring (A09:2021-Security Logging and Monitoring Failures)
- [x] **Audit Logs:** Critical actions (login, failed attempts, sensitive data access) are logged.
- [x] **No Secrets in Logs:** Ensure passwords/tokens are not written to logs.
- [x] **Alerting:** Mechanism in place to alert on suspicious activities.

## H. SSRF & Integrity (A10:2021-Server-Side Request Forgery)
- [x] **URL Validation:** Validate and sanitize user-supplied URLs if fetching remote resources.
- [x] **Integrity Checks:** Verify integrity of software updates and critical data.
