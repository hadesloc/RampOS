# Fix API Security

## Changes
1. **CORS Configuration:**
   - Modified `router.rs` to read allowed origins from `CORS_ALLOWED_ORIGINS` environment variable.
   - Falls back to `http://localhost:3000` if environment variable is not set.

2. **Security Headers:**
   - Added `tower_http::set_header::SetResponseHeaderLayer` middleware.
   - Headers added:
     - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
     - `X-Content-Type-Options: nosniff`
     - `X-Frame-Options: DENY`
     - `Content-Security-Policy: default-src 'self'`

## Verification
- Run `cargo check -p ramp-api` to ensure no compilation errors.
- Manual verification can be done by running the server and checking response headers.

## Files Modified
- `crates/ramp-api/src/router.rs`: Added middleware layers.
- `crates/ramp-api/Cargo.toml`: Enabled `set-header` feature for `tower-http`.
