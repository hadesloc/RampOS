use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant ID - represents an exchange/platform using RampOS
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl sqlx::Type<sqlx::Postgres> for TenantId {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for TenantId {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        <String as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&self.0, buf)
    }
}

impl sqlx::Decode<'_, sqlx::Postgres> for TenantId {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let s = <String as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Ok(TenantId(s))
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// User ID - represents an end user on a tenant platform
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Intent ID - unique identifier for each intent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentId(pub String);

impl IntentId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn new_payin() -> Self {
        Self(format!("pi_{}", Uuid::now_v7()))
    }

    #[must_use]
    pub fn new_payout() -> Self {
        Self(format!("po_{}", Uuid::now_v7()))
    }

    #[must_use]
    pub fn new_trade() -> Self {
        Self(format!("tr_{}", Uuid::now_v7()))
    }

    #[must_use]
    pub fn new_deposit() -> Self {
        Self(format!("dp_{}", Uuid::now_v7()))
    }

    #[must_use]
    pub fn new_withdraw() -> Self {
        Self(format!("wd_{}", Uuid::now_v7()))
    }
}

impl std::fmt::Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Money amount in VND (stored as Decimal for precision)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VndAmount(pub Decimal);

impl VndAmount {
    #[must_use]
    pub fn new(amount: Decimal) -> Self {
        Self(amount)
    }

    #[must_use]
    pub fn from_i64(amount: i64) -> Self {
        Self(Decimal::from(amount))
    }

    #[must_use]
    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.0.is_sign_positive() && !self.0.is_zero()
    }

    #[must_use]
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }
}

impl std::ops::Add for VndAmount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for VndAmount {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::fmt::Display for VndAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} VND", self.0)
    }
}

/// Crypto amount (stored as Decimal for precision)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CryptoAmount {
    pub amount: Decimal,
    pub symbol: CryptoSymbol,
}

impl CryptoAmount {
    #[must_use]
    pub fn new(amount: Decimal, symbol: CryptoSymbol) -> Self {
        Self { amount, symbol }
    }
}

/// Supported crypto symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoSymbol {
    BTC,
    ETH,
    USDT,
    USDC,
    BNB,
    SOL,
    Other,
}

impl std::fmt::Display for CryptoSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoSymbol::BTC => write!(f, "BTC"),
            CryptoSymbol::ETH => write!(f, "ETH"),
            CryptoSymbol::USDT => write!(f, "USDT"),
            CryptoSymbol::USDC => write!(f, "USDC"),
            CryptoSymbol::BNB => write!(f, "BNB"),
            CryptoSymbol::SOL => write!(f, "SOL"),
            CryptoSymbol::Other => write!(f, "OTHER"),
        }
    }
}

/// Trading pair
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradingPair {
    pub base: CryptoSymbol,
    pub quote: QuoteCurrency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuoteCurrency {
    VND,
    USDT,
}

/// Rails provider identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RailsProvider(pub String);

impl RailsProvider {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Bank account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub bank_code: String,
    pub account_number: String,
    pub account_name: String,
}

/// Virtual account for pay-in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualAccount {
    pub bank: String,
    pub account_number: String,
    pub account_name: String,
}

/// Reference code for tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceCode(pub String);

impl ReferenceCode {
    #[must_use]
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let code: String = (0..12)
            .map(|_| {
                let idx = rng.gen_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'A' + idx - 10) as char
                }
            })
            .collect();
        Self(code)
    }
}

impl std::fmt::Display for ReferenceCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Timestamp wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }

    #[must_use]
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Idempotency key for API requests
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

/// Webhook event ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub String);

impl EventId {
    #[must_use]
    pub fn new() -> Self {
        Self(format!("evt_{}", Uuid::now_v7()))
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain identifier for multi-chain support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChainId {
    Ethereum,
    Polygon,
    BnbChain,
    Arbitrum,
    Optimism,
    Base,
    Solana,
}

impl ChainId {
    #[must_use]
    pub fn evm_chain_id(&self) -> Option<u64> {
        match self {
            ChainId::Ethereum => Some(1),
            ChainId::Polygon => Some(137),
            ChainId::BnbChain => Some(56),
            ChainId::Arbitrum => Some(42161),
            ChainId::Optimism => Some(10),
            ChainId::Base => Some(8453),
            ChainId::Solana => None,
        }
    }
}

/// Wallet address
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalletAddress(pub String);

impl WalletAddress {
    #[must_use]
    pub fn new(address: impl Into<String>) -> Self {
        Self(address.into())
    }

    #[must_use]
    pub fn is_valid_evm(&self) -> bool {
        self.0.starts_with("0x") && self.0.len() == 42
    }
}

impl std::fmt::Display for WalletAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction hash
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxHash(pub String);

impl TxHash {
    #[must_use]
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }
}

impl std::fmt::Display for TxHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn test_tenant_id() {
        let id = TenantId::new("tenant123");
        assert_eq!(id.0, "tenant123");
        assert_eq!(format!("{}", id), "tenant123");
    }

    #[test]
    fn test_user_id() {
        let id = UserId::new("user456");
        assert_eq!(id.0, "user456");
        assert_eq!(format!("{}", id), "user456");
    }

    #[test]
    fn test_intent_id_prefixes() {
        let payin = IntentId::new_payin();
        assert!(payin.0.starts_with("pi_"));

        let payout = IntentId::new_payout();
        assert!(payout.0.starts_with("po_"));

        let trade = IntentId::new_trade();
        assert!(trade.0.starts_with("tr_"));

        let deposit = IntentId::new_deposit();
        assert!(deposit.0.starts_with("dp_"));

        let withdraw = IntentId::new_withdraw();
        assert!(withdraw.0.starts_with("wd_"));
    }

    #[test]
    fn test_vnd_amount_operations() {
        let a = VndAmount::from_i64(1000);
        let b = VndAmount::from_i64(500);

        let sum = a + b;
        assert_eq!(sum.0, Decimal::from(1500));

        let diff = a - b;
        assert_eq!(diff.0, Decimal::from(500));
    }

    #[test]
    fn test_vnd_amount_properties() {
        let zero = VndAmount::zero();
        assert!(zero.is_zero());
        assert!(!zero.is_positive());

        let positive = VndAmount::from_i64(100);
        assert!(!positive.is_zero());
        assert!(positive.is_positive());

        let negative = VndAmount::new(dec!(-100));
        assert!(!negative.is_positive());
        assert_eq!(negative.abs().0, dec!(100));
    }

    #[test]
    fn test_vnd_amount_display() {
        let amount = VndAmount::from_i64(1000000);
        assert_eq!(format!("{}", amount), "1000000 VND");
    }

    #[test]
    fn test_crypto_amount() {
        let btc = CryptoAmount::new(dec!(0.5), CryptoSymbol::BTC);
        assert_eq!(btc.amount, dec!(0.5));
        assert_eq!(btc.symbol, CryptoSymbol::BTC);
    }

    #[test]
    fn test_crypto_symbol_display() {
        assert_eq!(format!("{}", CryptoSymbol::BTC), "BTC");
        assert_eq!(format!("{}", CryptoSymbol::ETH), "ETH");
        assert_eq!(format!("{}", CryptoSymbol::USDT), "USDT");
        assert_eq!(format!("{}", CryptoSymbol::USDC), "USDC");
        assert_eq!(format!("{}", CryptoSymbol::BNB), "BNB");
        assert_eq!(format!("{}", CryptoSymbol::SOL), "SOL");
        assert_eq!(format!("{}", CryptoSymbol::Other), "OTHER");
    }

    #[test]
    fn test_rails_provider() {
        let provider = RailsProvider::new("VIETCOMBANK");
        assert_eq!(provider.0, "VIETCOMBANK");
    }

    #[test]
    fn test_bank_account() {
        let account = BankAccount {
            bank_code: "VCB".to_string(),
            account_number: "1234567890".to_string(),
            account_name: "NGUYEN VAN A".to_string(),
        };

        assert_eq!(account.bank_code, "VCB");
        assert_eq!(account.account_number, "1234567890");
    }

    #[test]
    fn test_reference_code_generation() {
        let code1 = ReferenceCode::generate();
        let code2 = ReferenceCode::generate();

        // Should be 12 characters
        assert_eq!(code1.0.len(), 12);
        assert_eq!(code2.0.len(), 12);

        // Should be alphanumeric
        assert!(code1.0.chars().all(|c| c.is_ascii_alphanumeric()));

        // Should be unique (very high probability)
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::now();
        let default_ts = Timestamp::default();

        // Both should be recent timestamps
        assert!(ts.0 <= Utc::now());
        assert!(default_ts.0 <= Utc::now());
    }

    #[test]
    fn test_idempotency_key() {
        let key = IdempotencyKey::new("unique-key-123");
        assert_eq!(key.0, "unique-key-123");
    }

    #[test]
    fn test_event_id() {
        let id1 = EventId::new();
        let id2 = EventId::default();

        assert!(id1.0.starts_with("evt_"));
        assert!(id2.0.starts_with("evt_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_chain_id_evm() {
        assert_eq!(ChainId::Ethereum.evm_chain_id(), Some(1));
        assert_eq!(ChainId::Polygon.evm_chain_id(), Some(137));
        assert_eq!(ChainId::BnbChain.evm_chain_id(), Some(56));
        assert_eq!(ChainId::Arbitrum.evm_chain_id(), Some(42161));
        assert_eq!(ChainId::Optimism.evm_chain_id(), Some(10));
        assert_eq!(ChainId::Base.evm_chain_id(), Some(8453));
        assert_eq!(ChainId::Solana.evm_chain_id(), None);
    }

    #[test]
    fn test_wallet_address() {
        let valid_evm = WalletAddress::new("0x1234567890123456789012345678901234567890");
        assert!(valid_evm.is_valid_evm());

        let invalid_short = WalletAddress::new("0x1234");
        assert!(!invalid_short.is_valid_evm());

        let invalid_prefix = WalletAddress::new("1234567890123456789012345678901234567890ab");
        assert!(!invalid_prefix.is_valid_evm());
    }

    #[test]
    fn test_tx_hash() {
        let hash = TxHash::new("0xabc123");
        assert_eq!(hash.0, "0xabc123");
        assert_eq!(format!("{}", hash), "0xabc123");
    }

    #[test]
    fn test_trading_pair() {
        let pair = TradingPair {
            base: CryptoSymbol::BTC,
            quote: QuoteCurrency::VND,
        };

        assert_eq!(pair.base, CryptoSymbol::BTC);
        assert_eq!(pair.quote, QuoteCurrency::VND);
    }

    #[test]
    fn test_virtual_account() {
        let va = VirtualAccount {
            bank: "VCB".to_string(),
            account_number: "RAMP123456".to_string(),
            account_name: "RAMP OS CO LTD".to_string(),
        };

        assert_eq!(va.bank, "VCB");
        assert_eq!(va.account_number, "RAMP123456");
    }
}
