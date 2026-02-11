//! Multi-chain adapter E2E tests
//!
//! Tests the unified Chain trait interface across EVM, Solana, and TON adapters.
//! Validates address formats, chain configs, registry operations, and
//! cross-chain consistency without hitting real RPCs.

use std::collections::HashMap;
use std::sync::Arc;

use ramp_core::chain::{
    Chain, ChainAbstractionLayer, ChainError, ChainId, ChainInfo, ChainRegistry, ChainType,
    EvmChain, EvmChainConfig, SolanaChain, SolanaChainConfig, TonChain,
    TonChainConfig, Transaction, TxHash, UnifiedAddress,
};

// ---------------------------------------------------------------------------
// Helper: create chain instances for testing (no real RPC needed for sync ops)
// ---------------------------------------------------------------------------

fn make_evm_ethereum() -> EvmChain {
    EvmChain::new(EvmChainConfig::ethereum("https://eth.test.local")).unwrap()
}

fn make_evm_arbitrum() -> EvmChain {
    EvmChain::new(EvmChainConfig::arbitrum("https://arb.test.local")).unwrap()
}

fn make_evm_base() -> EvmChain {
    EvmChain::new(EvmChainConfig::base("https://base.test.local")).unwrap()
}

fn make_evm_optimism() -> EvmChain {
    EvmChain::new(EvmChainConfig::optimism("https://op.test.local")).unwrap()
}

fn make_evm_polygon() -> EvmChain {
    EvmChain::new(EvmChainConfig::polygon("https://polygon.test.local")).unwrap()
}

fn make_solana_mainnet() -> SolanaChain {
    SolanaChain::new(SolanaChainConfig::mainnet("https://solana.test.local")).unwrap()
}

fn make_solana_devnet() -> SolanaChain {
    SolanaChain::new(SolanaChainConfig::devnet("https://solana-dev.test.local")).unwrap()
}

fn make_ton_mainnet() -> TonChain {
    TonChain::new(TonChainConfig::mainnet("https://ton.test.local")).unwrap()
}

fn make_ton_testnet() -> TonChain {
    TonChain::new(TonChainConfig::testnet("https://ton-test.test.local")).unwrap()
}

const VALID_EVM_ADDR: &str = "0x1234567890123456789012345678901234567890";
const VALID_SOLANA_ADDR: &str = "7cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy";
const VALID_TON_RAW_ADDR: &str =
    "0:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8";

// ===========================================================================
// 1. Unified Chain trait interface across all chain types
// ===========================================================================

#[test]
fn test_all_chains_implement_chain_trait_consistently() {
    let evm = make_evm_ethereum();
    let sol = make_solana_mainnet();
    let ton = make_ton_mainnet();

    // All chains expose chain_id, name, chain_type, is_testnet, native_symbol, explorer_url
    assert_eq!(evm.chain_id(), ChainId::ETHEREUM);
    assert_eq!(evm.chain_type(), ChainType::Evm);
    assert!(!evm.name().is_empty());
    assert!(!evm.native_symbol().is_empty());
    assert!(!evm.explorer_url().is_empty());

    assert_eq!(sol.chain_id(), ChainId::SOLANA_MAINNET);
    assert_eq!(sol.chain_type(), ChainType::Solana);
    assert!(!sol.name().is_empty());
    assert_eq!(sol.native_symbol(), "SOL");

    assert_eq!(ton.chain_id(), ChainId::TON_MAINNET);
    assert_eq!(ton.chain_type(), ChainType::Ton);
    assert_eq!(ton.native_symbol(), "TON");
}

// ===========================================================================
// 2. EVM address validation (valid + invalid)
// ===========================================================================

#[test]
fn test_evm_address_validation_valid() {
    let chain = make_evm_ethereum();
    let result = chain.validate_address(VALID_EVM_ADDR);
    assert!(result.is_ok());
    let ua = result.unwrap();
    assert_eq!(ua.chain_type, ChainType::Evm);
    // Normalized should be lowercase
    assert_eq!(ua.normalized, VALID_EVM_ADDR.to_lowercase());
}

#[test]
fn test_evm_address_validation_invalid_no_prefix() {
    let chain = make_evm_ethereum();
    let result = chain.validate_address("1234567890123456789012345678901234567890");
    assert!(result.is_err());
}

#[test]
fn test_evm_address_validation_invalid_too_short() {
    let chain = make_evm_ethereum();
    let result = chain.validate_address("0x1234");
    assert!(result.is_err());
}

#[test]
fn test_evm_address_validation_invalid_too_long() {
    let chain = make_evm_ethereum();
    let result = chain.validate_address("0x12345678901234567890123456789012345678901234");
    assert!(result.is_err());
}

#[test]
fn test_evm_address_validation_empty() {
    let chain = make_evm_ethereum();
    let result = chain.validate_address("");
    assert!(result.is_err());
}

// ===========================================================================
// 3. Solana address validation (valid + invalid)
// ===========================================================================

#[test]
fn test_solana_address_validation_valid() {
    let chain = make_solana_mainnet();
    let result = chain.validate_address(VALID_SOLANA_ADDR);
    assert!(result.is_ok());
    let ua = result.unwrap();
    assert_eq!(ua.chain_type, ChainType::Solana);
    assert_eq!(ua.address, VALID_SOLANA_ADDR);
}

#[test]
fn test_solana_address_system_program() {
    let chain = make_solana_mainnet();
    // System program: 32 '1's decodes to 32 zero bytes
    let result = chain.validate_address("11111111111111111111111111111111");
    assert!(result.is_ok());
}

#[test]
fn test_solana_address_validation_invalid_too_short() {
    let chain = make_solana_mainnet();
    let result = chain.validate_address("abc123");
    assert!(result.is_err());
}

#[test]
fn test_solana_address_validation_invalid_base58_chars() {
    let chain = make_solana_mainnet();
    // '0' (zero), 'O', 'I', 'l' are NOT valid base58 characters
    let result = chain.validate_address("0cVfgArCheMR6Cs4t6vz5rfnqd56vZq4ndaBrY5xkxXy");
    assert!(result.is_err());
}

// ===========================================================================
// 4. TON address validation (raw + user-friendly)
// ===========================================================================

#[test]
fn test_ton_raw_address_validation_valid() {
    let chain = make_ton_mainnet();
    let result = chain.validate_address(VALID_TON_RAW_ADDR);
    assert!(result.is_ok());
    let ua = result.unwrap();
    assert_eq!(ua.chain_type, ChainType::Ton);
}

#[test]
fn test_ton_user_friendly_address_validation() {
    let chain = make_ton_mainnet();
    let result = chain.validate_address("EQDtFpEwcFAEcRe5mLVh2N6C0x-_hJEM7W61_JLnSF74p4q2");
    assert!(result.is_ok());
}

#[test]
fn test_ton_masterchain_address() {
    let chain = make_ton_mainnet();
    let result = chain.validate_address(
        "-1:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8",
    );
    assert!(result.is_ok());
}

#[test]
fn test_ton_address_invalid_workchain() {
    let chain = make_ton_mainnet();
    let result = chain.validate_address(
        "2:83dfd552e63729b472fcbcc8c45ebcc6691702558b68ec7527e1ba403a0f31a8",
    );
    assert!(result.is_err());
}

#[test]
fn test_ton_address_invalid_too_short() {
    let chain = make_ton_mainnet();
    let result = chain.validate_address("EQDtFpEwcFAE");
    assert!(result.is_err());
}

// ===========================================================================
// 5. ChainRegistry: register, get, list, filter
// ===========================================================================

#[test]
fn test_chain_registry_register_and_get() {
    let mut registry = ChainRegistry::new();
    assert_eq!(registry.count(), 0);

    let eth = Arc::new(make_evm_ethereum()) as Arc<dyn Chain>;
    registry.register(eth);
    assert_eq!(registry.count(), 1);
    assert!(registry.has(ChainId::ETHEREUM));
    assert!(!registry.has(ChainId::ARBITRUM));

    let chain = registry.get(ChainId::ETHEREUM);
    assert!(chain.is_some());
    assert_eq!(chain.unwrap().name(), "Ethereum");
}

#[test]
fn test_chain_registry_get_or_error() {
    let registry = ChainRegistry::new();
    let result = registry.get_or_error(ChainId::ETHEREUM);
    assert!(result.is_err());
    let err_msg = format!("{}", result.err().unwrap());
    assert!(err_msg.contains("1"), "Error should contain chain id '1', got: {}", err_msg);
}

#[test]
fn test_chain_registry_list_all() {
    let mut registry = ChainRegistry::new();
    registry.register(Arc::new(make_evm_ethereum()));
    registry.register(Arc::new(make_solana_mainnet()));
    registry.register(Arc::new(make_ton_mainnet()));

    let all = registry.list();
    assert_eq!(all.len(), 3);
}

#[test]
fn test_chain_registry_list_by_type() {
    let mut registry = ChainRegistry::new();
    registry.register(Arc::new(make_evm_ethereum()));
    registry.register(Arc::new(make_evm_arbitrum()));
    registry.register(Arc::new(make_solana_mainnet()));
    registry.register(Arc::new(make_ton_mainnet()));

    let evm_chains = registry.list_by_type(ChainType::Evm);
    assert_eq!(evm_chains.len(), 2);

    let sol_chains = registry.list_by_type(ChainType::Solana);
    assert_eq!(sol_chains.len(), 1);

    let ton_chains = registry.list_by_type(ChainType::Ton);
    assert_eq!(ton_chains.len(), 1);
}

// ===========================================================================
// 6. ChainAbstractionLayer: multi-chain management
// ===========================================================================

#[test]
fn test_chain_abstraction_layer_register_and_get() {
    let mut cal = ChainAbstractionLayer::new();
    cal.register(Arc::new(make_evm_ethereum()));
    cal.register(Arc::new(make_solana_mainnet()));

    assert!(cal.get_chain(ChainId::ETHEREUM).is_ok());
    assert!(cal.get_chain(ChainId::SOLANA_MAINNET).is_ok());
    assert!(cal.get_chain(ChainId::TON_MAINNET).is_err());
}

#[test]
fn test_chain_abstraction_layer_validate_address_cross_chain() {
    let mut cal = ChainAbstractionLayer::new();
    cal.register(Arc::new(make_evm_ethereum()));
    cal.register(Arc::new(make_solana_mainnet()));
    cal.register(Arc::new(make_ton_mainnet()));

    // EVM address on Ethereum chain
    assert!(cal.validate_address(ChainId::ETHEREUM, VALID_EVM_ADDR).is_ok());

    // Solana address on Solana chain
    assert!(cal.validate_address(ChainId::SOLANA_MAINNET, VALID_SOLANA_ADDR).is_ok());

    // TON address on TON chain
    assert!(cal.validate_address(ChainId::TON_MAINNET, VALID_TON_RAW_ADDR).is_ok());

    // Wrong address format on wrong chain should fail
    assert!(cal.validate_address(ChainId::ETHEREUM, VALID_SOLANA_ADDR).is_err());
    assert!(cal.validate_address(ChainId::SOLANA_MAINNET, VALID_EVM_ADDR).is_err());
}

#[test]
fn test_chain_abstraction_layer_supported_chains() {
    let mut cal = ChainAbstractionLayer::new();
    cal.register(Arc::new(make_evm_ethereum()));
    cal.register(Arc::new(make_evm_base()));
    cal.register(Arc::new(make_solana_mainnet()));

    let supported = cal.supported_chains();
    assert_eq!(supported.len(), 3);
}

// ===========================================================================
// 7. EVM chain configs: correct parameters
// ===========================================================================

#[test]
fn test_evm_chain_configs_correct_parameters() {
    let eth = EvmChainConfig::ethereum("https://rpc");
    assert_eq!(eth.chain_id, ChainId::ETHEREUM);
    assert_eq!(eth.native_symbol, "ETH");
    assert!(!eth.is_testnet);
    assert!(eth.eip1559);
    assert_eq!(eth.block_time_secs, 12);

    let arb = EvmChainConfig::arbitrum("https://rpc");
    assert_eq!(arb.chain_id, ChainId::ARBITRUM);
    assert_eq!(arb.native_symbol, "ETH");
    assert_eq!(arb.block_time_secs, 1);

    let base = EvmChainConfig::base("https://rpc");
    assert_eq!(base.chain_id, ChainId::BASE);
    assert_eq!(base.native_symbol, "ETH");
    assert_eq!(base.block_time_secs, 2);

    let op = EvmChainConfig::optimism("https://rpc");
    assert_eq!(op.chain_id, ChainId::OPTIMISM);
    assert_eq!(op.native_symbol, "ETH");

    let poly = EvmChainConfig::polygon("https://rpc");
    assert_eq!(poly.chain_id, ChainId::POLYGON);
    assert_eq!(poly.native_symbol, "MATIC");

    let bsc = EvmChainConfig::bsc("https://rpc");
    assert_eq!(bsc.chain_id, ChainId::BSC);
    assert_eq!(bsc.native_symbol, "BNB");
    assert!(!bsc.eip1559); // BSC doesn't use EIP-1559

    let sepolia = EvmChainConfig::sepolia("https://rpc");
    assert_eq!(sepolia.chain_id, ChainId::SEPOLIA);
    assert!(sepolia.is_testnet);
}

// ===========================================================================
// 8. Solana chain configs
// ===========================================================================

#[test]
fn test_solana_chain_configs() {
    let mainnet = SolanaChainConfig::mainnet("https://rpc");
    assert_eq!(mainnet.chain_id, ChainId::SOLANA_MAINNET);
    assert!(!mainnet.is_testnet);

    let devnet = SolanaChainConfig::devnet("https://rpc");
    assert_eq!(devnet.chain_id, ChainId::SOLANA_DEVNET);
    assert!(devnet.is_testnet);
}

// ===========================================================================
// 9. TON chain configs
// ===========================================================================

#[test]
fn test_ton_chain_configs() {
    let mainnet = TonChainConfig::mainnet("https://api");
    assert_eq!(mainnet.chain_id, ChainId::TON_MAINNET);
    assert!(!mainnet.is_testnet);

    let testnet = TonChainConfig::testnet("https://api");
    assert_eq!(testnet.chain_id, ChainId::TON_TESTNET);
    assert!(testnet.is_testnet);
}

// ===========================================================================
// 10. ChainId constants: no collisions, correct values
// ===========================================================================

#[test]
fn test_chain_id_constants_no_collisions() {
    let ids = vec![
        ChainId::ETHEREUM,
        ChainId::GOERLI,
        ChainId::SEPOLIA,
        ChainId::ARBITRUM,
        ChainId::ARBITRUM_SEPOLIA,
        ChainId::BASE,
        ChainId::BASE_SEPOLIA,
        ChainId::OPTIMISM,
        ChainId::OPTIMISM_SEPOLIA,
        ChainId::POLYGON,
        ChainId::POLYGON_AMOY,
        ChainId::POLYGON_ZKEVM,
        ChainId::BSC,
        ChainId::AVALANCHE,
        ChainId::SOLANA_MAINNET,
        ChainId::SOLANA_DEVNET,
        ChainId::TON_MAINNET,
        ChainId::TON_TESTNET,
    ];

    // All IDs must be unique
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        assert!(seen.insert(id.0), "Duplicate ChainId: {}", id.0);
    }
}

#[test]
fn test_chain_id_well_known_values() {
    assert_eq!(ChainId::ETHEREUM.0, 1);
    assert_eq!(ChainId::ARBITRUM.0, 42161);
    assert_eq!(ChainId::BASE.0, 8453);
    assert_eq!(ChainId::OPTIMISM.0, 10);
    assert_eq!(ChainId::POLYGON.0, 137);
    assert_eq!(ChainId::BSC.0, 56);
    assert_eq!(ChainId::AVALANCHE.0, 43114);
    // Non-EVM chains use high IDs
    assert!(ChainId::SOLANA_MAINNET.0 > 100_000);
    assert!(ChainId::TON_MAINNET.0 > 100_000);
}

// ===========================================================================
// 11. UnifiedAddress: creation and normalization
// ===========================================================================

#[test]
fn test_unified_address_evm_normalizes_to_lowercase() {
    let addr = "0xABCDEF1234567890ABCDEF1234567890ABCDEF12";
    let ua = UnifiedAddress::new(ChainType::Evm, addr).unwrap();
    assert_eq!(ua.normalized, addr.to_lowercase());
    assert_eq!(ua.address, addr);
    assert_eq!(ua.chain_type, ChainType::Evm);
}

#[test]
fn test_unified_address_solana_preserves_case() {
    let ua = UnifiedAddress::new(ChainType::Solana, VALID_SOLANA_ADDR).unwrap();
    assert_eq!(ua.normalized, VALID_SOLANA_ADDR);
}

#[test]
fn test_unified_address_ton_preserves_format() {
    let ua = UnifiedAddress::new(ChainType::Ton, VALID_TON_RAW_ADDR).unwrap();
    assert_eq!(ua.normalized, VALID_TON_RAW_ADDR);
}

// ===========================================================================
// 12. ChainInfo from Chain trait
// ===========================================================================

#[test]
fn test_chain_info_from_chain() {
    let eth = make_evm_ethereum();
    let info = ChainInfo::from_chain(&eth);
    assert_eq!(info.chain_id, 1);
    assert_eq!(info.name, "Ethereum");
    assert_eq!(info.chain_type, ChainType::Evm);
    assert_eq!(info.native_symbol, "ETH");
    assert!(!info.is_testnet);
    assert!(!info.explorer_url.is_empty());
}

#[test]
fn test_chain_info_from_solana() {
    let sol = make_solana_mainnet();
    let info = ChainInfo::from_chain(&sol);
    assert_eq!(info.chain_id, ChainId::SOLANA_MAINNET.0);
    assert_eq!(info.chain_type, ChainType::Solana);
    assert_eq!(info.native_symbol, "SOL");
}

#[test]
fn test_chain_info_from_ton() {
    let ton = make_ton_mainnet();
    let info = ChainInfo::from_chain(&ton);
    assert_eq!(info.chain_id, ChainId::TON_MAINNET.0);
    assert_eq!(info.chain_type, ChainType::Ton);
    assert_eq!(info.native_symbol, "TON");
}

// ===========================================================================
// 13. Explorer URLs: tx and address URLs
// ===========================================================================

#[test]
fn test_explorer_urls_evm() {
    let eth = make_evm_ethereum();
    let hash = TxHash("0xabc123".to_string());
    let tx_url = eth.tx_url(&hash);
    assert!(tx_url.contains("etherscan.io"));
    assert!(tx_url.contains("0xabc123"));

    let addr_url = eth.address_url(VALID_EVM_ADDR);
    assert!(addr_url.contains("etherscan.io"));
    assert!(addr_url.contains(VALID_EVM_ADDR));
}

#[test]
fn test_explorer_urls_solana_mainnet_vs_devnet() {
    let mainnet = make_solana_mainnet();
    let devnet = make_solana_devnet();
    let hash = TxHash("sig123".to_string());

    let mainnet_url = mainnet.tx_url(&hash);
    assert!(!mainnet_url.contains("cluster=devnet"));

    let devnet_url = devnet.tx_url(&hash);
    assert!(devnet_url.contains("cluster=devnet"));

    let devnet_addr_url = devnet.address_url(VALID_SOLANA_ADDR);
    assert!(devnet_addr_url.contains("cluster=devnet"));
}

#[test]
fn test_explorer_urls_ton() {
    let ton = make_ton_mainnet();
    let hash = TxHash("tonhash123".to_string());
    let tx_url = ton.tx_url(&hash);
    assert!(tx_url.contains("tonscan.org"));
}

// ===========================================================================
// 14. Testnet vs mainnet detection
// ===========================================================================

#[test]
fn test_testnet_detection() {
    assert!(!make_evm_ethereum().is_testnet());
    assert!(!make_evm_arbitrum().is_testnet());
    assert!(!make_solana_mainnet().is_testnet());
    assert!(!make_ton_mainnet().is_testnet());

    assert!(make_solana_devnet().is_testnet());
    assert!(make_ton_testnet().is_testnet());

    // Sepolia is a testnet
    let sepolia = EvmChain::new(EvmChainConfig::sepolia("https://rpc")).unwrap();
    assert!(sepolia.is_testnet());
}

// ===========================================================================
// 15. Chain type display formatting
// ===========================================================================

#[test]
fn test_chain_type_display() {
    assert_eq!(format!("{}", ChainType::Evm), "EVM");
    assert_eq!(format!("{}", ChainType::Solana), "Solana");
    assert_eq!(format!("{}", ChainType::Ton), "TON");
}

#[test]
fn test_chain_id_display() {
    assert_eq!(format!("{}", ChainId::ETHEREUM), "1");
    assert_eq!(format!("{}", ChainId::ARBITRUM), "42161");
    assert_eq!(format!("{}", ChainId::SOLANA_MAINNET), "900001");
}

// ===========================================================================
// 16. Solana fee estimation (sync, no RPC)
// ===========================================================================

#[tokio::test]
async fn test_solana_fee_estimation_structure() {
    let chain = make_solana_mainnet();
    let tx = Transaction {
        from: VALID_SOLANA_ADDR.to_string(),
        to: "11111111111111111111111111111111".to_string(),
        value: "1000000000".to_string(),
        data: None,
        gas_limit: Some(200_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None,
    };

    let fee = chain.estimate_fee(&tx).await.unwrap();
    assert_eq!(fee.gas_units, 200_000);
    // Slow should be cheapest, fast should be most expensive
    let slow_cost: u64 = fee.slow.total_cost.parse().unwrap();
    let standard_cost: u64 = fee.standard.total_cost.parse().unwrap();
    let fast_cost: u64 = fee.fast.total_cost.parse().unwrap();
    assert!(slow_cost <= standard_cost);
    assert!(standard_cost <= fast_cost);
    // All should have priority_fee for Solana
    assert!(fee.slow.priority_fee.is_some());
    assert!(fee.standard.priority_fee.is_some());
    assert!(fee.fast.priority_fee.is_some());
}

// ===========================================================================
// 17. TON fee estimation fallback (no API key)
// ===========================================================================

#[tokio::test]
async fn test_ton_fee_estimation_fallback() {
    std::env::remove_var("TON_API_KEY");
    let chain = make_ton_mainnet();
    let tx = Transaction {
        from: VALID_TON_RAW_ADDR.to_string(),
        to: VALID_TON_RAW_ADDR.to_string(),
        value: "1000000000".to_string(),
        data: None,
        gas_limit: None,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None,
    };

    let fee = chain.estimate_fee(&tx).await.unwrap();
    // Fallback fees should be reasonable TON values
    let slow_cost: u64 = fee.slow.total_cost.parse().unwrap();
    let fast_cost: u64 = fee.fast.total_cost.parse().unwrap();
    assert!(slow_cost > 0);
    assert!(fast_cost >= slow_cost);
}

// ===========================================================================
// 18. Chain error variants
// ===========================================================================

#[test]
fn test_chain_error_display_messages() {
    let err = ChainError::ChainNotFound("42".to_string());
    assert!(format!("{}", err).contains("42"));

    let err = ChainError::InvalidAddress("bad addr".to_string());
    assert!(format!("{}", err).contains("bad addr"));

    let err = ChainError::InsufficientBalance {
        required: "100".to_string(),
        available: "50".to_string(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("100"));
    assert!(msg.contains("50"));

    let err = ChainError::Timeout;
    assert!(format!("{}", err).contains("Timeout"));

    let err = ChainError::NotSupported("feature".to_string());
    assert!(format!("{}", err).contains("feature"));
}

// ===========================================================================
// 19. Multi-chain registry with all chain types
// ===========================================================================

#[test]
fn test_full_multi_chain_registry() {
    let mut registry = ChainRegistry::new();

    // Register multiple EVM chains
    registry.register(Arc::new(make_evm_ethereum()));
    registry.register(Arc::new(make_evm_arbitrum()));
    registry.register(Arc::new(make_evm_base()));
    registry.register(Arc::new(make_evm_optimism()));
    registry.register(Arc::new(make_evm_polygon()));

    // Register Solana chains
    registry.register(Arc::new(make_solana_mainnet()));
    registry.register(Arc::new(make_solana_devnet()));

    // Register TON chains
    registry.register(Arc::new(make_ton_mainnet()));
    registry.register(Arc::new(make_ton_testnet()));

    assert_eq!(registry.count(), 9);

    // Filter by type
    let evm = registry.list_by_type(ChainType::Evm);
    assert_eq!(evm.len(), 5);

    let solana = registry.list_by_type(ChainType::Solana);
    assert_eq!(solana.len(), 2);

    let ton = registry.list_by_type(ChainType::Ton);
    assert_eq!(ton.len(), 2);

    // Verify each chain is retrievable
    assert!(registry.get(ChainId::ETHEREUM).is_some());
    assert!(registry.get(ChainId::ARBITRUM).is_some());
    assert!(registry.get(ChainId::BASE).is_some());
    assert!(registry.get(ChainId::OPTIMISM).is_some());
    assert!(registry.get(ChainId::POLYGON).is_some());
    assert!(registry.get(ChainId::SOLANA_MAINNET).is_some());
    assert!(registry.get(ChainId::SOLANA_DEVNET).is_some());
    assert!(registry.get(ChainId::TON_MAINNET).is_some());
    assert!(registry.get(ChainId::TON_TESTNET).is_some());
}

// ===========================================================================
// 20. Cross-chain address rejection
// ===========================================================================

#[test]
fn test_cross_chain_address_rejection() {
    let evm = make_evm_ethereum();
    let sol = make_solana_mainnet();
    let ton = make_ton_mainnet();

    // EVM chain rejects Solana address
    assert!(evm.validate_address(VALID_SOLANA_ADDR).is_err());
    // EVM chain rejects TON address
    assert!(evm.validate_address(VALID_TON_RAW_ADDR).is_err());
    // Solana chain rejects EVM address
    assert!(sol.validate_address(VALID_EVM_ADDR).is_err());
    // TON chain rejects EVM address (too short for user-friendly, no colon for raw)
    assert!(ton.validate_address(VALID_EVM_ADDR).is_err());
}

// ===========================================================================
// 21. TON to_raw_address conversion
// ===========================================================================

#[test]
fn test_ton_to_raw_address_passthrough() {
    let chain = make_ton_mainnet();
    let raw = chain.to_raw_address(VALID_TON_RAW_ADDR).unwrap();
    assert_eq!(raw, VALID_TON_RAW_ADDR);
}

// ===========================================================================
// 22. ChainAbstractionLayer with_defaults (empty map = no chains registered)
// ===========================================================================

#[test]
fn test_chain_abstraction_with_defaults_empty() {
    let cal = ChainAbstractionLayer::with_defaults(HashMap::new()).unwrap();
    assert_eq!(cal.supported_chains().len(), 0);
    assert!(cal.get_chain(ChainId::ETHEREUM).is_err());
}

// ===========================================================================
// 23. Polygon zkEVM config
// ===========================================================================

#[test]
fn test_polygon_zkevm_config() {
    let config = EvmChainConfig::polygon_zkevm("https://rpc");
    assert_eq!(config.chain_id, ChainId::POLYGON_ZKEVM);
    assert_eq!(config.native_symbol, "ETH");
    assert!(!config.is_testnet);
    assert_eq!(config.block_time_secs, 5);
}

// ===========================================================================
// 24. TxHash display
// ===========================================================================

#[test]
fn test_tx_hash_display_and_equality() {
    let h1 = TxHash("0xabc".to_string());
    let h2 = TxHash("0xabc".to_string());
    let h3 = TxHash("0xdef".to_string());

    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
    assert_eq!(format!("{}", h1), "0xabc");
}
