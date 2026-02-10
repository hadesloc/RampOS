//! F10 Chain Abstraction - API Tests
//!
//! Tests for the chain abstraction layer: chain registry, address validation,
//! chain info types, and API route handler DTOs.

use ramp_core::chain::{
    ChainId, ChainInfo, ChainRegistry, ChainType, UnifiedAddress,
};

// ============================================================
// Test 1: Supported chains list from registry
// ============================================================

#[test]
fn test_supported_chains_list() {
    let registry = ChainRegistry::new();
    // Empty registry initially
    let chains = registry.list();
    assert!(chains.is_empty());
    assert_eq!(registry.count(), 0);

    // Verify the known chain ID constants exist
    assert_eq!(ChainId::ETHEREUM.0, 1);
    assert_eq!(ChainId::ARBITRUM.0, 42161);
    assert_eq!(ChainId::BASE.0, 8453);
    assert_eq!(ChainId::OPTIMISM.0, 10);
    assert_eq!(ChainId::POLYGON.0, 137);
    assert_eq!(ChainId::POLYGON_ZKEVM.0, 1101);
    assert_eq!(ChainId::BSC.0, 56);
    assert_eq!(ChainId::AVALANCHE.0, 43114);
    assert_eq!(ChainId::SOLANA_MAINNET.0, 900001);
    assert_eq!(ChainId::TON_MAINNET.0, 900101);
}

// ============================================================
// Test 2: Chain status via registry lookup
// ============================================================

#[test]
fn test_chain_status_endpoint() {
    let registry = ChainRegistry::new();

    // Lookup a chain that is NOT registered -> should return None
    assert!(registry.get(ChainId::ETHEREUM).is_none());
    assert!(!registry.has(ChainId::ETHEREUM));

    // get_or_error returns proper error
    let result = registry.get_or_error(ChainId::ETHEREUM);
    assert!(result.is_err());
    let err = result.err().unwrap();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("Chain not found"),
        "Error should say 'Chain not found', got: {}",
        err_msg
    );
}

// ============================================================
// Test 3: Chain address validation (bridge prerequisite)
// ============================================================

#[test]
fn test_chain_bridge_quote_address_validation() {
    // EVM address validation
    let valid_evm = UnifiedAddress::new(
        ChainType::Evm,
        "0x1234567890123456789012345678901234567890",
    );
    assert!(valid_evm.is_ok());
    let addr = valid_evm.unwrap();
    assert_eq!(addr.chain_type, ChainType::Evm);
    assert_eq!(
        addr.normalized,
        "0x1234567890123456789012345678901234567890"
    );

    // Invalid EVM address
    let invalid_evm = UnifiedAddress::new(ChainType::Evm, "0xinvalid");
    assert!(invalid_evm.is_err());

    let no_prefix = UnifiedAddress::new(ChainType::Evm, "1234567890123456789012345678901234567890");
    assert!(no_prefix.is_err());

    // Solana address validation (base58, 32-44 chars)
    let valid_sol = UnifiedAddress::new(
        ChainType::Solana,
        "7EcDhSYGxXyscszYEp35KHN8vvw3svAuLKTzXwCFLtV",
    );
    assert!(valid_sol.is_ok());

    let invalid_sol = UnifiedAddress::new(ChainType::Solana, "short");
    assert!(invalid_sol.is_err());

    // TON address validation
    let valid_ton = UnifiedAddress::new(
        ChainType::Ton,
        "EQDtFpEwcFAEcRe5mLVh2N6C0x-_hJEM7W61_JLnSF78p7As",
    );
    assert!(valid_ton.is_ok());

    let invalid_ton = UnifiedAddress::new(ChainType::Ton, "tiny");
    assert!(invalid_ton.is_err());
}

// ============================================================
// Test 4: Chain type display and serialization
// ============================================================

#[test]
fn test_chain_type_display() {
    assert_eq!(format!("{}", ChainType::Evm), "EVM");
    assert_eq!(format!("{}", ChainType::Solana), "Solana");
    assert_eq!(format!("{}", ChainType::Ton), "TON");
}

// ============================================================
// Test 5: ChainId display
// ============================================================

#[test]
fn test_chain_id_display() {
    assert_eq!(format!("{}", ChainId::ETHEREUM), "1");
    assert_eq!(format!("{}", ChainId::ARBITRUM), "42161");
    assert_eq!(format!("{}", ChainId(12345)), "12345");
}

// ============================================================
// Test 6: Registry list_by_type with no registered chains
// ============================================================

#[test]
fn test_registry_list_by_type() {
    let registry = ChainRegistry::new();
    let evm_chains = registry.list_by_type(ChainType::Evm);
    assert!(evm_chains.is_empty());

    let sol_chains = registry.list_by_type(ChainType::Solana);
    assert!(sol_chains.is_empty());

    let ton_chains = registry.list_by_type(ChainType::Ton);
    assert!(ton_chains.is_empty());
}

// ============================================================
// Test 7: ChainId equality and hashing
// ============================================================

#[test]
fn test_chain_id_equality() {
    assert_eq!(ChainId(1), ChainId::ETHEREUM);
    assert_ne!(ChainId::ETHEREUM, ChainId::ARBITRUM);

    // ChainId can be used as HashMap key (Hash + Eq)
    let mut map = std::collections::HashMap::new();
    map.insert(ChainId::ETHEREUM, "Ethereum");
    map.insert(ChainId::ARBITRUM, "Arbitrum");
    assert_eq!(map.get(&ChainId::ETHEREUM), Some(&"Ethereum"));
    assert_eq!(map.get(&ChainId::ARBITRUM), Some(&"Arbitrum"));
    assert_eq!(map.get(&ChainId::BASE), None);
}

// ============================================================
// Test 8: Testnet chain IDs
// ============================================================

#[test]
fn test_testnet_chain_ids() {
    assert_eq!(ChainId::SEPOLIA.0, 11155111);
    assert_eq!(ChainId::ARBITRUM_SEPOLIA.0, 421614);
    assert_eq!(ChainId::BASE_SEPOLIA.0, 84532);
    assert_eq!(ChainId::OPTIMISM_SEPOLIA.0, 11155420);
    assert_eq!(ChainId::POLYGON_AMOY.0, 80002);
    assert_eq!(ChainId::SOLANA_DEVNET.0, 900002);
    assert_eq!(ChainId::TON_TESTNET.0, 900102);
}

// ============================================================
// Test 9: ChainInfo from_chain and serialization
// ============================================================

#[test]
fn test_chain_info_serialization() {
    let info = ChainInfo {
        chain_id: ChainId::ETHEREUM.0,
        name: "Ethereum".to_string(),
        chain_type: ChainType::Evm,
        native_symbol: "ETH".to_string(),
        is_testnet: false,
        explorer_url: "https://etherscan.io".to_string(),
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"chainId\":1"));
    assert!(json.contains("\"name\":\"Ethereum\""));
    assert!(json.contains("\"nativeSymbol\":\"ETH\""));
    assert!(json.contains("\"isTestnet\":false"));

    // Deserialize back
    let deserialized: ChainInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.chain_id, 1);
    assert_eq!(deserialized.name, "Ethereum");
}

// ============================================================
// Test 10: Chain handler DTO serialization (BridgeQuoteRequest)
// ============================================================

#[test]
fn test_bridge_quote_request_dto() {
    use ramp_api::handlers::chain::BridgeQuoteRequest;

    let json = r#"{
        "sourceChainId": 1,
        "destinationChainId": 42161,
        "amount": "1000000000000000000",
        "senderAddress": "0x1234567890123456789012345678901234567890"
    }"#;
    let req: BridgeQuoteRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.source_chain_id, ChainId::ETHEREUM.0);
    assert_eq!(req.destination_chain_id, ChainId::ARBITRUM.0);
    assert!(req.token_address.is_none());
    assert!(req.recipient_address.is_none());
}

// ============================================================
// Test 11: Chain handler DTO serialization (BridgeRequest)
// ============================================================

#[test]
fn test_bridge_request_dto() {
    use ramp_api::handlers::chain::BridgeRequest;

    let json = r#"{
        "quoteId": "quote_abc123",
        "sourceChainId": 1,
        "destinationChainId": 8453,
        "amount": "500000000000000000",
        "senderAddress": "0x1234567890123456789012345678901234567890",
        "recipientAddress": "0x0987654321098765432109876543210987654321"
    }"#;
    let req: BridgeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.quote_id, "quote_abc123");
    assert_eq!(req.source_chain_id, ChainId::ETHEREUM.0);
    assert_eq!(req.destination_chain_id, ChainId::BASE.0);
    assert_eq!(req.recipient_address, "0x0987654321098765432109876543210987654321");
}

// ============================================================
// Test 12: ChainListResponse and ChainDetailResponse serialization
// ============================================================

#[test]
fn test_chain_response_dtos() {
    use ramp_api::handlers::chain::{ChainListResponse, ChainDetailResponse};

    let list_resp = ChainListResponse {
        chains: vec![
            ChainInfo {
                chain_id: 1,
                name: "Ethereum".to_string(),
                chain_type: ChainType::Evm,
                native_symbol: "ETH".to_string(),
                is_testnet: false,
                explorer_url: "https://etherscan.io".to_string(),
            },
            ChainInfo {
                chain_id: ChainId::SOLANA_MAINNET.0,
                name: "Solana".to_string(),
                chain_type: ChainType::Solana,
                native_symbol: "SOL".to_string(),
                is_testnet: false,
                explorer_url: "https://solscan.io".to_string(),
            },
        ],
        total: 2,
    };

    let json = serde_json::to_string(&list_resp).unwrap();
    assert!(json.contains("\"total\":2"));
    assert!(json.contains("\"Ethereum\""));
    assert!(json.contains("\"Solana\""));

    let detail_resp = ChainDetailResponse {
        info: ChainInfo {
            chain_id: 1,
            name: "Ethereum".to_string(),
            chain_type: ChainType::Evm,
            native_symbol: "ETH".to_string(),
            is_testnet: false,
            explorer_url: "https://etherscan.io".to_string(),
        },
        status: "active".to_string(),
    };

    let json = serde_json::to_string(&detail_resp).unwrap();
    assert!(json.contains("\"status\":\"active\""));
    assert!(json.contains("\"chainId\":1"));
}
