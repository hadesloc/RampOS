//! Cursor-based pagination following the Relay Connection specification

use async_graphql::{Object, SimpleObject};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use super::types::{IntentType, LedgerEntryType, UserType};

/// Page info for cursor-based pagination
#[derive(SimpleObject, Debug, Clone)]
pub struct PageInfo {
    /// Whether there are more items after the last edge
    pub has_next_page: bool,
    /// Whether there are more items before the first edge
    pub has_previous_page: bool,
    /// Cursor of the last edge in the current page
    pub end_cursor: Option<String>,
    /// Cursor of the first edge in the current page
    pub start_cursor: Option<String>,
}

// ============================================================================
// Intent Connection
// ============================================================================

/// A single edge wrapping an Intent node with its cursor
pub struct IntentEdge {
    pub cursor: String,
    pub node: IntentType,
}

#[Object]
impl IntentEdge {
    async fn cursor(&self) -> &str {
        &self.cursor
    }

    async fn node(&self) -> &IntentType {
        &self.node
    }
}

/// Intent connection with edges and page info
#[derive(SimpleObject)]
pub struct IntentConnection {
    pub edges: Vec<IntentEdge>,
    pub page_info: PageInfo,
    pub total_count: Option<i64>,
}

// ============================================================================
// User Connection
// ============================================================================

/// A single edge wrapping a User node with its cursor
pub struct UserEdge {
    pub cursor: String,
    pub node: UserType,
}

#[Object]
impl UserEdge {
    async fn cursor(&self) -> &str {
        &self.cursor
    }

    async fn node(&self) -> &UserType {
        &self.node
    }
}

/// User connection with edges and page info
#[derive(SimpleObject)]
pub struct UserConnection {
    pub edges: Vec<UserEdge>,
    pub page_info: PageInfo,
    pub total_count: Option<i64>,
}

// ============================================================================
// LedgerEntry Connection
// ============================================================================

/// A single edge wrapping a LedgerEntry node with its cursor
pub struct LedgerEntryEdge {
    pub cursor: String,
    pub node: LedgerEntryType,
}

#[Object]
impl LedgerEntryEdge {
    async fn cursor(&self) -> &str {
        &self.cursor
    }

    async fn node(&self) -> &LedgerEntryType {
        &self.node
    }
}

/// LedgerEntry connection with edges and page info
#[derive(SimpleObject)]
pub struct LedgerEntryConnection {
    pub edges: Vec<LedgerEntryEdge>,
    pub page_info: PageInfo,
    pub total_count: Option<i64>,
}

// ============================================================================
// Cursor Encoding/Decoding
// ============================================================================

/// Encode an offset into an opaque cursor string
pub fn encode_cursor(offset: usize) -> String {
    BASE64.encode(format!("cursor:{}", offset))
}

/// Decode an opaque cursor string back into an offset
pub fn decode_cursor(cursor: &str) -> Option<usize> {
    let decoded = BASE64.decode(cursor).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let offset_str = s.strip_prefix("cursor:")?;
    offset_str.parse::<usize>().ok()
}

/// Build an intent connection from items with pagination parameters
pub fn build_intent_connection(
    items: Vec<IntentType>,
    first: Option<i32>,
    after: Option<String>,
    total_count: Option<i64>,
) -> IntentConnection {
    let start_offset = after
        .as_ref()
        .and_then(|c| decode_cursor(c))
        .map(|o| o + 1)
        .unwrap_or(0);

    let limit = first.unwrap_or(20).max(1).min(100) as usize;
    let has_next_page = items.len() > limit;
    let items: Vec<IntentType> = items.into_iter().take(limit).collect();

    let edges: Vec<IntentEdge> = items
        .into_iter()
        .enumerate()
        .map(|(i, node)| IntentEdge {
            cursor: encode_cursor(start_offset + i),
            node,
        })
        .collect();

    let start_cursor = edges.first().map(|e| e.cursor.clone());
    let end_cursor = edges.last().map(|e| e.cursor.clone());
    let has_previous_page = start_offset > 0;

    IntentConnection {
        edges,
        page_info: PageInfo {
            has_next_page,
            has_previous_page,
            start_cursor,
            end_cursor,
        },
        total_count,
    }
}

/// Build a user connection from items with pagination parameters
pub fn build_user_connection(
    items: Vec<UserType>,
    first: Option<i32>,
    after: Option<String>,
    total_count: Option<i64>,
) -> UserConnection {
    let start_offset = after
        .as_ref()
        .and_then(|c| decode_cursor(c))
        .map(|o| o + 1)
        .unwrap_or(0);

    let limit = first.unwrap_or(20).max(1).min(100) as usize;
    let has_next_page = items.len() > limit;
    let items: Vec<UserType> = items.into_iter().take(limit).collect();

    let edges: Vec<UserEdge> = items
        .into_iter()
        .enumerate()
        .map(|(i, node)| UserEdge {
            cursor: encode_cursor(start_offset + i),
            node,
        })
        .collect();

    let start_cursor = edges.first().map(|e| e.cursor.clone());
    let end_cursor = edges.last().map(|e| e.cursor.clone());
    let has_previous_page = start_offset > 0;

    UserConnection {
        edges,
        page_info: PageInfo {
            has_next_page,
            has_previous_page,
            start_cursor,
            end_cursor,
        },
        total_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_encode_decode_roundtrip() {
        let offset = 42;
        let cursor = encode_cursor(offset);
        let decoded = decode_cursor(&cursor);
        assert_eq!(decoded, Some(42));
    }

    #[test]
    fn test_cursor_decode_invalid_returns_none() {
        assert_eq!(decode_cursor("not-a-valid-cursor"), None);
        assert_eq!(decode_cursor(""), None);
    }

    #[test]
    fn test_cursor_encode_zero() {
        let cursor = encode_cursor(0);
        assert_eq!(decode_cursor(&cursor), Some(0));
    }
}
