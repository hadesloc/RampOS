//! VietQR Service for Off-Ramp (F16.07)
//!
//! Generates VietQR codes following the EMVCo QR Code standard.
//! Used for generating payment QR codes that Vietnamese bank apps can scan.
//!
//! This module is standalone and does NOT require modification of existing mod.rs/lib.rs.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// VietQR code data following EMVCo standard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VietQRData {
    /// The full EMVCo-formatted QR string
    pub qr_string: String,
    /// Bank code (BIN) used
    pub bank_bin: String,
    /// Account number
    pub account_number: String,
    /// Amount in VND (if specified)
    pub amount_vnd: Option<Decimal>,
    /// Transaction reference
    pub reference: String,
    /// Whether this is a static or dynamic QR
    pub is_dynamic: bool,
}

/// VietQR bank BIN mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VietQRBank {
    pub code: String,
    pub bin: String,
    pub name: String,
    pub short_name: String,
    pub swift_code: Option<String>,
}

// ============================================================================
// VietQR Service
// ============================================================================

/// Service for generating VietQR codes following EMVCo specification
pub struct VietQROffRampService {
    /// Known bank BIN mappings
    banks: Vec<VietQRBank>,
}

impl VietQROffRampService {
    /// Create a new VietQR service with known Vietnamese bank BINs
    pub fn new() -> Self {
        let banks = vec![
            VietQRBank {
                code: "VCB".to_string(),
                bin: "970436".to_string(),
                name: "Joint Stock Commercial Bank for Foreign Trade of Vietnam".to_string(),
                short_name: "Vietcombank".to_string(),
                swift_code: Some("BFTVVNVX".to_string()),
            },
            VietQRBank {
                code: "TCB".to_string(),
                bin: "970407".to_string(),
                name: "Vietnam Technological and Commercial Joint Stock Bank".to_string(),
                short_name: "Techcombank".to_string(),
                swift_code: Some("VTCBVNVX".to_string()),
            },
            VietQRBank {
                code: "ACB".to_string(),
                bin: "970416".to_string(),
                name: "Asia Commercial Joint Stock Bank".to_string(),
                short_name: "ACB".to_string(),
                swift_code: Some("ASCBVNVX".to_string()),
            },
            VietQRBank {
                code: "MBB".to_string(),
                bin: "970422".to_string(),
                name: "Military Commercial Joint Stock Bank".to_string(),
                short_name: "MB Bank".to_string(),
                swift_code: Some("MSCBVNVX".to_string()),
            },
            VietQRBank {
                code: "VPB".to_string(),
                bin: "970432".to_string(),
                name: "Vietnam Prosperity Joint-Stock Commercial Bank".to_string(),
                short_name: "VPBank".to_string(),
                swift_code: Some("VPBKVNVX".to_string()),
            },
            VietQRBank {
                code: "BIDV".to_string(),
                bin: "970418".to_string(),
                name: "Bank for Investment and Development of Vietnam".to_string(),
                short_name: "BIDV".to_string(),
                swift_code: Some("BIDVVNVX".to_string()),
            },
            VietQRBank {
                code: "VTB".to_string(),
                bin: "970415".to_string(),
                name: "Vietnam Joint Stock Commercial Bank for Industry and Trade".to_string(),
                short_name: "VietinBank".to_string(),
                swift_code: Some("ICBVVNVX".to_string()),
            },
            VietQRBank {
                code: "TPB".to_string(),
                bin: "970423".to_string(),
                name: "Tien Phong Commercial Joint Stock Bank".to_string(),
                short_name: "TPBank".to_string(),
                swift_code: Some("TPBVVNVX".to_string()),
            },
        ];

        Self { banks }
    }

    /// Generate a VietQR code string in EMVCo format
    ///
    /// # Arguments
    /// * `bank_code` - Vietnamese bank code (e.g., "VCB", "TCB")
    /// * `account` - Bank account number
    /// * `amount` - Amount in VND (optional for static QR)
    /// * `reference` - Transaction reference/memo
    ///
    /// # Returns
    /// EMVCo-formatted QR data string
    pub fn generate_qr(
        &self,
        bank_code: &str,
        account: &str,
        amount: Option<Decimal>,
        reference: &str,
    ) -> Result<VietQRData, String> {
        // Look up bank BIN
        let bank = self
            .banks
            .iter()
            .find(|b| b.code == bank_code.to_uppercase())
            .ok_or_else(|| format!("Unknown bank code: {}", bank_code))?;

        // Build EMVCo QR code string
        let qr_string = self.build_emvco_string(&bank.bin, account, amount, reference);

        let is_dynamic = amount.is_some();

        Ok(VietQRData {
            qr_string,
            bank_bin: bank.bin.clone(),
            account_number: account.to_string(),
            amount_vnd: amount,
            reference: reference.to_string(),
            is_dynamic,
        })
    }

    /// Get bank information by code
    pub fn get_bank(&self, bank_code: &str) -> Option<&VietQRBank> {
        self.banks
            .iter()
            .find(|b| b.code == bank_code.to_uppercase())
    }

    /// List all supported banks
    pub fn list_banks(&self) -> &[VietQRBank] {
        &self.banks
    }

    /// Build an EMVCo-compliant QR code string
    ///
    /// EMVCo format: TLV (Tag-Length-Value) encoding
    /// Key tags:
    /// - 00: Payload Format Indicator
    /// - 01: Point of Initiation Method (11=static, 12=dynamic)
    /// - 38: Merchant Account Information (VietQR)
    ///   - 00: GUID (A000000727)
    ///   - 01: Member ID (bank BIN)
    ///   - 02: Service Code (QRIBFTTA for interbank)
    /// - 52: Merchant Category Code
    /// - 53: Transaction Currency (704 = VND)
    /// - 54: Transaction Amount
    /// - 58: Country Code (VN)
    /// - 62: Additional Data
    ///   - 08: Reference/Purpose
    /// - 63: CRC (CRC-16/CCITT-FALSE)
    fn build_emvco_string(
        &self,
        bank_bin: &str,
        account: &str,
        amount: Option<Decimal>,
        reference: &str,
    ) -> String {
        let mut parts = Vec::new();

        // 00 - Payload Format Indicator
        parts.push(Self::tlv("00", "01"));

        // 01 - Point of Initiation Method
        let method = if amount.is_some() { "12" } else { "11" };
        parts.push(Self::tlv("01", method));

        // 38 - Merchant Account Information (VietQR)
        let guid = Self::tlv("00", "A000000727");
        let member_id = Self::tlv("01", bank_bin);
        let service_code = Self::tlv("02", "QRIBFTTA");
        let merchant_info = format!("{}{}{}", guid, member_id, service_code);
        parts.push(Self::tlv("38", &merchant_info));

        // 52 - Merchant Category Code
        parts.push(Self::tlv("52", "5812"));

        // 53 - Transaction Currency (704 = VND)
        parts.push(Self::tlv("53", "704"));

        // 54 - Transaction Amount (if specified)
        if let Some(amt) = amount {
            let amount_str = amt.trunc().to_string();
            parts.push(Self::tlv("54", &amount_str));
        }

        // 58 - Country Code
        parts.push(Self::tlv("58", "VN"));

        // 62 - Additional Data Field
        // Sub-field 05: Reference Label (account number)
        // Sub-field 08: Purpose of Transaction
        let acct_field = Self::tlv("05", account);
        let purpose_field = Self::tlv("08", reference);
        let additional = format!("{}{}", acct_field, purpose_field);
        parts.push(Self::tlv("62", &additional));

        // Build the string without CRC first
        let mut payload = parts.join("");

        // 63 - CRC placeholder (will be computed)
        payload.push_str("6304");
        let crc = Self::crc16_ccitt(payload.as_bytes());
        let crc_hex = format!("{:04X}", crc);

        // Replace placeholder with actual CRC
        payload.truncate(payload.len() - 4);
        payload.push_str(&crc_hex);

        payload
    }

    /// Create a TLV (Tag-Length-Value) encoded string
    fn tlv(tag: &str, value: &str) -> String {
        format!("{}{:02}{}", tag, value.len(), value)
    }

    /// Calculate CRC-16/CCITT-FALSE checksum
    fn crc16_ccitt(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= (byte as u16) << 8;
            for _ in 0..8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }
}

impl Default for VietQROffRampService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_generate_qr_with_amount() {
        let service = VietQROffRampService::new();
        let qr = service
            .generate_qr("VCB", "1234567890", Some(dec!(1_000_000)), "RAMP-PAY-001")
            .unwrap();

        assert!(!qr.qr_string.is_empty());
        assert_eq!(qr.bank_bin, "970436");
        assert_eq!(qr.account_number, "1234567890");
        assert_eq!(qr.amount_vnd, Some(dec!(1_000_000)));
        assert!(qr.is_dynamic);
    }

    #[test]
    fn test_generate_qr_static() {
        let service = VietQROffRampService::new();
        let qr = service
            .generate_qr("TCB", "9876543210", None, "DONATE")
            .unwrap();

        assert!(!qr.qr_string.is_empty());
        assert_eq!(qr.bank_bin, "970407");
        assert!(!qr.is_dynamic);
        assert!(qr.amount_vnd.is_none());
    }

    #[test]
    fn test_generate_qr_unknown_bank() {
        let service = VietQROffRampService::new();
        let result = service.generate_qr("UNKNOWN", "123", None, "REF");
        assert!(result.is_err());
    }

    #[test]
    fn test_emvco_format() {
        let service = VietQROffRampService::new();
        let qr = service
            .generate_qr("VCB", "123456", Some(dec!(500_000)), "TEST")
            .unwrap();

        let s = &qr.qr_string;

        // Should start with payload format indicator
        assert!(s.starts_with("000201"));
        // Should contain dynamic initiation (12)
        assert!(s.contains("010212"));
        // Should contain VND currency code (704)
        assert!(s.contains("5303704"));
        // Should contain country code (VN)
        assert!(s.contains("5802VN"));
        // Should end with 4-char CRC
        assert!(s.len() > 4);
    }

    #[test]
    fn test_get_bank() {
        let service = VietQROffRampService::new();

        let bank = service.get_bank("VCB");
        assert!(bank.is_some());
        let bank = bank.unwrap();
        assert_eq!(bank.bin, "970436");
        assert_eq!(bank.short_name, "Vietcombank");

        let unknown = service.get_bank("UNKNOWN");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_list_banks() {
        let service = VietQROffRampService::new();
        let banks = service.list_banks();
        assert!(banks.len() >= 8);

        let codes: Vec<&str> = banks.iter().map(|b| b.code.as_str()).collect();
        assert!(codes.contains(&"VCB"));
        assert!(codes.contains(&"TCB"));
        assert!(codes.contains(&"BIDV"));
    }

    #[test]
    fn test_crc16_ccitt() {
        // Known test vector: "123456789" -> 0x29B1
        let crc = VietQROffRampService::crc16_ccitt(b"123456789");
        assert_eq!(crc, 0x29B1);
    }

    #[test]
    fn test_tlv_encoding() {
        let result = VietQROffRampService::tlv("00", "01");
        assert_eq!(result, "000201");

        let result = VietQROffRampService::tlv("58", "VN");
        assert_eq!(result, "5802VN");
    }

    #[test]
    fn test_qr_data_serializable() {
        let service = VietQROffRampService::new();
        let qr = service
            .generate_qr("MBB", "1111222233", Some(dec!(100_000)), "REF123")
            .unwrap();

        let json = serde_json::to_string(&qr).unwrap();
        assert!(json.contains("\"qr_string\""));
        assert!(json.contains("\"bank_bin\""));
        assert!(json.contains("\"reference\""));
    }

    #[test]
    fn test_case_insensitive_bank_code() {
        let service = VietQROffRampService::new();
        let qr1 = service.generate_qr("vcb", "123", None, "REF");
        let qr2 = service.generate_qr("VCB", "123", None, "REF");
        assert!(qr1.is_ok());
        assert!(qr2.is_ok());
        assert_eq!(qr1.unwrap().bank_bin, qr2.unwrap().bank_bin);
    }
}
