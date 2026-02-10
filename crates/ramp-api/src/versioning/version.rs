//! API Version type - Stripe-style date-based versioning
//!
//! Each API version is identified by a release date in `YYYY-MM-DD` format.
//! Versions are ordered chronologically: newer versions introduce changes
//! that may be backward-incompatible.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Known API versions in chronological order.
/// The first entry is the initial release; the last is the latest.
pub const KNOWN_VERSIONS: &[&str] = &["2026-02-01", "2026-03-01"];

/// The default version assigned to new tenants or requests without an explicit version.
pub const DEFAULT_VERSION: &str = "2026-02-01";

/// The latest available API version.
pub const LATEST_VERSION: &str = "2026-03-01";

/// The minimum version the server still supports.
pub const MINIMUM_VERSION: &str = "2026-02-01";

/// A date-based API version following Stripe's convention.
///
/// Versions are compared by their date: earlier dates are "older" versions.
/// Each version may have an associated set of request/response transformations
/// that adapt payloads from the latest internal format to the version the
/// client expects.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ApiVersion {
    date: NaiveDate,
}

impl ApiVersion {
    /// Create a new `ApiVersion` from a `NaiveDate`.
    pub fn new(date: NaiveDate) -> Self {
        Self { date }
    }

    /// Parse a version string in `YYYY-MM-DD` format.
    pub fn parse(s: &str) -> Result<Self, ApiVersionError> {
        let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| ApiVersionError::InvalidFormat(s.to_string()))?;
        Ok(Self { date })
    }

    /// The default version for requests/tenants that don't specify one.
    pub fn default_version() -> Self {
        Self::parse(DEFAULT_VERSION).expect("DEFAULT_VERSION is valid")
    }

    /// The latest (newest) available version.
    pub fn latest() -> Self {
        Self::parse(LATEST_VERSION).expect("LATEST_VERSION is valid")
    }

    /// The minimum supported version.
    pub fn minimum() -> Self {
        Self::parse(MINIMUM_VERSION).expect("MINIMUM_VERSION is valid")
    }

    /// Returns `true` if this version is compatible (i.e., within the supported range).
    pub fn is_compatible(&self) -> bool {
        *self >= Self::minimum() && *self <= Self::latest()
    }

    /// Returns `true` if this version is a known, officially published version.
    pub fn is_known(&self) -> bool {
        let s = self.to_string();
        KNOWN_VERSIONS.contains(&s.as_str())
    }

    /// Returns the underlying `NaiveDate`.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// Returns `true` if `self` is at least as new as `other`.
    pub fn is_at_least(&self, other: &Self) -> bool {
        self.date >= other.date
    }

    /// Returns `true` if `self` is strictly older than `other`.
    pub fn is_older_than(&self, other: &Self) -> bool {
        self.date < other.date
    }

    /// Return all known versions in chronological order.
    pub fn all_known() -> Vec<Self> {
        KNOWN_VERSIONS
            .iter()
            .filter_map(|s| Self::parse(s).ok())
            .collect()
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.date.format("%Y-%m-%d"))
    }
}

impl FromStr for ApiVersion {
    type Err = ApiVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Errors that can occur when working with API versions.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ApiVersionError {
    #[error("Invalid API version format: '{0}'. Expected YYYY-MM-DD.")]
    InvalidFormat(String),

    #[error("API version '{0}' is no longer supported. Minimum: {1}")]
    TooOld(String, String),

    #[error("Unknown API version '{0}'. Latest: {1}")]
    Unknown(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_version() {
        let v = ApiVersion::parse("2026-02-01").unwrap();
        assert_eq!(v.to_string(), "2026-02-01");
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(ApiVersion::parse("not-a-date").is_err());
        assert!(ApiVersion::parse("2026/02/01").is_err());
        assert!(ApiVersion::parse("").is_err());
        assert!(ApiVersion::parse("2026-13-01").is_err());
    }

    #[test]
    fn test_default_version() {
        let v = ApiVersion::default_version();
        assert_eq!(v.to_string(), "2026-02-01");
    }

    #[test]
    fn test_latest_version() {
        let v = ApiVersion::latest();
        assert_eq!(v.to_string(), "2026-03-01");
    }

    #[test]
    fn test_version_ordering() {
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();
        assert!(v1 < v2);
        assert!(v2 > v1);
        assert!(v1.is_older_than(&v2));
        assert!(!v2.is_older_than(&v1));
    }

    #[test]
    fn test_is_compatible() {
        let old = ApiVersion::parse("2025-01-01").unwrap();
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();
        let future = ApiVersion::parse("2030-01-01").unwrap();

        assert!(!old.is_compatible());
        assert!(v1.is_compatible());
        assert!(v2.is_compatible());
        assert!(!future.is_compatible());
    }

    #[test]
    fn test_is_known() {
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();
        let unknown = ApiVersion::parse("2026-02-15").unwrap();

        assert!(v1.is_known());
        assert!(v2.is_known());
        assert!(!unknown.is_known());
    }

    #[test]
    fn test_is_at_least() {
        let v1 = ApiVersion::parse("2026-02-01").unwrap();
        let v2 = ApiVersion::parse("2026-03-01").unwrap();
        assert!(v2.is_at_least(&v1));
        assert!(v1.is_at_least(&v1));
        assert!(!v1.is_at_least(&v2));
    }

    #[test]
    fn test_from_str() {
        let v: ApiVersion = "2026-02-01".parse().unwrap();
        assert_eq!(v.to_string(), "2026-02-01");
    }

    #[test]
    fn test_all_known() {
        let all = ApiVersion::all_known();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].to_string(), "2026-02-01");
        assert_eq!(all[1].to_string(), "2026-03-01");
    }

    #[test]
    fn test_equality() {
        let a = ApiVersion::parse("2026-02-01").unwrap();
        let b = ApiVersion::parse("2026-02-01").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_serialize_deserialize() {
        let v = ApiVersion::parse("2026-02-01").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        let v2: ApiVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(v, v2);
    }
}
