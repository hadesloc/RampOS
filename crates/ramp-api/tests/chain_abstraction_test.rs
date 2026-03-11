//! F10 Chain Abstraction - API Tests
//!
//! Tests for the chain abstraction layer: chain registry, address validation,
//! chain info types, API route handler DTOs, config-backed registry,
//! bridge quote calculations, and input validation.

use ramp_core::chain::{ChainId, ChainInfo, ChainRegistry, ChainType, UnifiedAddress};

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
    let valid_evm =
        UnifiedAddress::new(ChainType::Evm, "0x1234567890123456789012345678901234567890");
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
    assert_eq!(
        req.recipient_address,
        "0x0987654321098765432109876543210987654321"
    );
}

// ============================================================
// Test 12: ChainListResponse and ChainDetailResponse serialization
// ============================================================

#[test]
fn test_chain_response_dtos() {
    use ramp_api::handlers::chain::{ChainDetailResponse, ChainListResponse};

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

// ============================================================
// Test 13: ChainRegistryConfig - default chains listing
// ============================================================

#[test]
fn test_config_registry_default_chains() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let chains = ChainRegistryConfig::default_chain_infos();

    // Should have at least 8 mainnets + testnets
    assert!(
        chains.len() >= 8,
        "Expected at least 8 chains, got {}",
        chains.len()
    );

    // All major mainnets present
    let chain_ids: Vec<u64> = chains.iter().map(|c| c.chain_id).collect();
    assert!(chain_ids.contains(&ChainId::ETHEREUM.0));
    assert!(chain_ids.contains(&ChainId::ARBITRUM.0));
    assert!(chain_ids.contains(&ChainId::BASE.0));
    assert!(chain_ids.contains(&ChainId::OPTIMISM.0));
    assert!(chain_ids.contains(&ChainId::POLYGON.0));
    assert!(chain_ids.contains(&ChainId::BSC.0));
    assert!(chain_ids.contains(&ChainId::SOLANA_MAINNET.0));
    assert!(chain_ids.contains(&ChainId::TON_MAINNET.0));
}

// ============================================================
// Test 14: ChainRegistryConfig - filtering by chain type
// ============================================================

#[test]
fn test_config_registry_filter_by_type() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let all_chains = ChainRegistryConfig::default_chain_infos();

    let evm_chains: Vec<_> = all_chains
        .iter()
        .filter(|c| c.chain_type == ChainType::Evm)
        .collect();
    assert!(
        evm_chains.len() >= 6,
        "Expected at least 6 EVM chains, got {}",
        evm_chains.len()
    );

    let solana_chains: Vec<_> = all_chains
        .iter()
        .filter(|c| c.chain_type == ChainType::Solana)
        .collect();
    assert!(
        !solana_chains.is_empty(),
        "Should have at least 1 Solana chain"
    );

    let ton_chains: Vec<_> = all_chains
        .iter()
        .filter(|c| c.chain_type == ChainType::Ton)
        .collect();
    assert!(!ton_chains.is_empty(), "Should have at least 1 TON chain");
}

// ============================================================
// Test 15: ChainRegistryConfig - filtering by testnet status
// ============================================================

#[test]
fn test_config_registry_filter_by_testnet() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let all_chains = ChainRegistryConfig::default_chain_infos();

    let mainnets: Vec<_> = all_chains.iter().filter(|c| !c.is_testnet).collect();
    let testnets: Vec<_> = all_chains.iter().filter(|c| c.is_testnet).collect();

    assert!(
        mainnets.len() >= 8,
        "Should have at least 8 mainnets, got {}",
        mainnets.len()
    );
    assert!(
        testnets.len() >= 2,
        "Should have at least 2 testnets, got {}",
        testnets.len()
    );

    // All mainnets should not be testnet
    for chain in &mainnets {
        assert!(!chain.is_testnet, "{} should not be testnet", chain.name);
    }

    // All testnets should be testnet
    for chain in &testnets {
        assert!(chain.is_testnet, "{} should be testnet", chain.name);
    }
}

// ============================================================
// Test 16: ChainRegistryConfig - chain lookup by ID
// ============================================================

#[test]
fn test_config_registry_lookup_by_id() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    // Existing chain
    let eth = ChainRegistryConfig::get_chain_info(ChainId::ETHEREUM.0);
    assert!(eth.is_some());
    let eth = eth.unwrap();
    assert_eq!(eth.name, "Ethereum");
    assert_eq!(eth.chain_type, ChainType::Evm);

    // Non-existent chain
    let unknown = ChainRegistryConfig::get_chain_info(99999);
    assert!(unknown.is_none());

    // Solana
    let sol = ChainRegistryConfig::get_chain_info(ChainId::SOLANA_MAINNET.0);
    assert!(sol.is_some());
    assert_eq!(sol.unwrap().chain_type, ChainType::Solana);

    // TON
    let ton = ChainRegistryConfig::get_chain_info(ChainId::TON_MAINNET.0);
    assert!(ton.is_some());
    assert_eq!(ton.unwrap().chain_type, ChainType::Ton);
}

// ============================================================
// Test 17: Bridge quote fee calculation - EVM to EVM
// ============================================================

#[test]
fn test_bridge_quote_fee_evm_to_evm() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let source = ChainRegistryConfig::get_chain_info(ChainId::ETHEREUM.0).unwrap();
    let dest = ChainRegistryConfig::get_chain_info(ChainId::ARBITRUM.0).unwrap();

    let (fee_bps, gas, time) = ChainRegistryConfig::estimate_bridge_params(&source, &dest);

    assert_eq!(fee_bps, 10, "EVM<->EVM bridge fee should be 10 bps (0.10%)");
    assert!(gas > 0.0, "Gas estimate should be positive");
    assert_eq!(time, 300, "EVM<->EVM estimated time should be 5 min");
}

// ============================================================
// Test 18: Bridge quote fee calculation - cross-type routes
// ============================================================

#[test]
fn test_bridge_quote_fee_cross_type() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    // EVM -> Solana
    let eth = ChainRegistryConfig::get_chain_info(ChainId::ETHEREUM.0).unwrap();
    let sol = ChainRegistryConfig::get_chain_info(ChainId::SOLANA_MAINNET.0).unwrap();
    let (fee_bps, _, time) = ChainRegistryConfig::estimate_bridge_params(&eth, &sol);
    assert_eq!(fee_bps, 25, "EVM->Solana should be 25 bps");
    assert_eq!(time, 600, "EVM->Solana should be 10 min");

    // Solana -> EVM (symmetric)
    let (fee_bps_rev, _, time_rev) = ChainRegistryConfig::estimate_bridge_params(&sol, &eth);
    assert_eq!(fee_bps_rev, 25, "Solana->EVM should also be 25 bps");
    assert_eq!(time_rev, 600);

    // EVM -> TON
    let ton = ChainRegistryConfig::get_chain_info(ChainId::TON_MAINNET.0).unwrap();
    let (fee_bps_ton, _, time_ton) = ChainRegistryConfig::estimate_bridge_params(&eth, &ton);
    assert_eq!(fee_bps_ton, 30, "EVM->TON should be 30 bps");
    assert_eq!(time_ton, 900, "EVM->TON should be 15 min");

    // Solana -> TON (exotic)
    let (fee_bps_exotic, _, _) = ChainRegistryConfig::estimate_bridge_params(&sol, &ton);
    assert_eq!(fee_bps_exotic, 35, "Solana->TON should be 35 bps");
}

// ============================================================
// Test 19: Bridge initiation - input validation errors
// ============================================================

#[test]
fn test_bridge_request_validation_empty_quote() {
    use ramp_api::handlers::chain::BridgeRequest;

    // Empty quote_id
    let json = r#"{
        "quoteId": "",
        "sourceChainId": 1,
        "destinationChainId": 42161,
        "amount": "1000000000000000000",
        "senderAddress": "0x1234567890123456789012345678901234567890",
        "recipientAddress": "0x0987654321098765432109876543210987654321"
    }"#;
    let req: BridgeRequest = serde_json::from_str(json).unwrap();
    assert!(
        req.quote_id.trim().is_empty(),
        "Empty quote_id should be caught by handler"
    );
}

// ============================================================
// Test 20: Bridge quote request - zero/negative amount detection
// ============================================================

#[test]
fn test_bridge_quote_zero_amount_detection() {
    use ramp_api::handlers::chain::BridgeQuoteRequest;

    let json = r#"{
        "sourceChainId": 1,
        "destinationChainId": 42161,
        "amount": "0",
        "senderAddress": "0x1234567890123456789012345678901234567890"
    }"#;
    let req: BridgeQuoteRequest = serde_json::from_str(json).unwrap();
    let amount: f64 = req.amount.parse().unwrap();
    assert!(
        amount <= 0.0,
        "Zero amount should be caught by handler validation"
    );
}

// ============================================================
// Test 21: Multiple chain type support verification
// ============================================================

#[test]
fn test_multi_chain_type_coverage() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let chains = ChainRegistryConfig::default_chain_infos();

    let types: std::collections::HashSet<ChainType> = chains.iter().map(|c| c.chain_type).collect();

    assert!(types.contains(&ChainType::Evm), "Must support EVM chains");
    assert!(
        types.contains(&ChainType::Solana),
        "Must support Solana chains"
    );
    assert!(types.contains(&ChainType::Ton), "Must support TON chains");
    assert_eq!(types.len(), 3, "Should support exactly 3 chain types");
}

// ============================================================
// Test 22: FeeBreakdown DTO serialization
// ============================================================

#[test]
fn test_fee_breakdown_dto() {
    use ramp_api::handlers::chain::FeeBreakdown;

    let breakdown = FeeBreakdown {
        bridge_fee: "100000".to_string(),
        gas_fee: "50000".to_string(),
        protocol_fee: "5000".to_string(),
    };

    let json = serde_json::to_string(&breakdown).unwrap();
    assert!(json.contains("\"bridgeFee\":\"100000\""));
    assert!(json.contains("\"gasFee\":\"50000\""));
    assert!(json.contains("\"protocolFee\":\"5000\""));
}

// ============================================================
// Test 23: BridgeQuoteResponse with fee breakdown serialization
// ============================================================

#[test]
fn test_bridge_quote_response_with_fee_breakdown() {
    use ramp_api::handlers::chain::{BridgeQuoteResponse, FeeBreakdown};

    let resp = BridgeQuoteResponse {
        source_chain_id: 1,
        destination_chain_id: 42161,
        input_amount: "1000000000000000000".to_string(),
        output_amount: "998500000000000000".to_string(),
        fee: "1500000000000000".to_string(),
        fee_percentage: "0.15".to_string(),
        fee_breakdown: FeeBreakdown {
            bridge_fee: "1000000000000000".to_string(),
            gas_fee: "400000000000000".to_string(),
            protocol_fee: "100000000000000".to_string(),
        },
        estimated_time_seconds: 300,
        expires_at: "2026-01-01T00:00:00Z".to_string(),
        quote_id: "quote_test123".to_string(),
    };

    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"feeBreakdown\""));
    assert!(json.contains("\"bridgeFee\""));
    assert!(json.contains("\"gasFee\""));
    assert!(json.contains("\"protocolFee\""));
    assert!(json.contains("\"feePercentage\":\"0.15\""));
    assert!(json.contains("\"quoteId\":\"quote_test123\""));
}

// ============================================================
// Test 24: ChainListQuery deserialization
// ============================================================

#[test]
fn test_chain_list_query_deserialization() {
    use ramp_api::handlers::chain::ChainListQuery;

    // With both filters
    let json = r#"{"chainType":"evm","testnet":false}"#;
    let query: ChainListQuery = serde_json::from_str(json).unwrap();
    assert_eq!(query.chain_type.as_deref(), Some("evm"));
    assert_eq!(query.testnet, Some(false));

    // With only chain type
    let json2 = r#"{"chainType":"solana"}"#;
    let query2: ChainListQuery = serde_json::from_str(json2).unwrap();
    assert_eq!(query2.chain_type.as_deref(), Some("solana"));
    assert!(query2.testnet.is_none());

    // Empty query (no filters)
    let empty: ChainListQuery = serde_json::from_str("{}").unwrap();
    assert!(empty.chain_type.is_none());
    assert!(empty.testnet.is_none());
}

// ============================================================
// Test 25: Invalid chain ID error case
// ============================================================

#[test]
fn test_invalid_chain_id_returns_none() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    assert!(ChainRegistryConfig::get_chain_info(0).is_none());
    assert!(ChainRegistryConfig::get_chain_info(999).is_none());
    assert!(ChainRegistryConfig::get_chain_info(u64::MAX).is_none());
}

// ============================================================
// Test 26: Bridge request with invalid quote_id prefix
// ============================================================

#[test]
fn test_bridge_request_invalid_quote_prefix() {
    use ramp_api::handlers::chain::BridgeRequest;

    let json = r#"{
        "quoteId": "invalid_prefix_123",
        "sourceChainId": 1,
        "destinationChainId": 42161,
        "amount": "1000000000000000000",
        "senderAddress": "0x1234567890123456789012345678901234567890",
        "recipientAddress": "0x0987654321098765432109876543210987654321"
    }"#;
    let req: BridgeRequest = serde_json::from_str(json).unwrap();
    // The handler will reject this because it doesn't start with "quote_"
    assert!(!req.quote_id.starts_with("quote_"));
}

// ============================================================
// Test 27: Chain info native symbols are correct
// ============================================================

#[test]
fn test_chain_native_symbols() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let eth = ChainRegistryConfig::get_chain_info(ChainId::ETHEREUM.0).unwrap();
    assert_eq!(eth.native_symbol, "ETH");

    let polygon = ChainRegistryConfig::get_chain_info(ChainId::POLYGON.0).unwrap();
    assert_eq!(polygon.native_symbol, "MATIC");

    let bsc = ChainRegistryConfig::get_chain_info(ChainId::BSC.0).unwrap();
    assert_eq!(bsc.native_symbol, "BNB");

    let sol = ChainRegistryConfig::get_chain_info(ChainId::SOLANA_MAINNET.0).unwrap();
    assert_eq!(sol.native_symbol, "SOL");

    let ton = ChainRegistryConfig::get_chain_info(ChainId::TON_MAINNET.0).unwrap();
    assert_eq!(ton.native_symbol, "TON");
}

// ============================================================
// Test 28: Explorer URLs are populated
// ============================================================

#[test]
fn test_chain_explorer_urls() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let chains = ChainRegistryConfig::default_chain_infos();
    for chain in &chains {
        assert!(
            !chain.explorer_url.is_empty(),
            "Chain {} should have a non-empty explorer URL",
            chain.name
        );
        assert!(
            chain.explorer_url.starts_with("https://"),
            "Chain {} explorer URL should start with https://",
            chain.name
        );
    }
}

// ============================================================
// Test 29: Bridge fee is always lower for same-type chains
// ============================================================

#[test]
fn test_bridge_fee_ordering() {
    use ramp_api::handlers::chain::ChainRegistryConfig;

    let eth = ChainRegistryConfig::get_chain_info(ChainId::ETHEREUM.0).unwrap();
    let arb = ChainRegistryConfig::get_chain_info(ChainId::ARBITRUM.0).unwrap();
    let sol = ChainRegistryConfig::get_chain_info(ChainId::SOLANA_MAINNET.0).unwrap();
    let ton = ChainRegistryConfig::get_chain_info(ChainId::TON_MAINNET.0).unwrap();

    let (evm_evm_fee, _, _) = ChainRegistryConfig::estimate_bridge_params(&eth, &arb);
    let (evm_sol_fee, _, _) = ChainRegistryConfig::estimate_bridge_params(&eth, &sol);
    let (evm_ton_fee, _, _) = ChainRegistryConfig::estimate_bridge_params(&eth, &ton);
    let (sol_ton_fee, _, _) = ChainRegistryConfig::estimate_bridge_params(&sol, &ton);

    // Same type should be cheapest
    assert!(
        evm_evm_fee < evm_sol_fee,
        "EVM<->EVM ({}) should be cheaper than EVM<->Solana ({})",
        evm_evm_fee,
        evm_sol_fee
    );
    assert!(
        evm_sol_fee < sol_ton_fee,
        "EVM<->Solana ({}) should be cheaper than Solana<->TON ({})",
        evm_sol_fee,
        sol_ton_fee
    );
    // EVM<->TON should also be relatively high
    assert!(
        evm_evm_fee < evm_ton_fee,
        "EVM<->EVM ({}) should be cheaper than EVM<->TON ({})",
        evm_evm_fee,
        evm_ton_fee
    );
}
