use chrono::{TimeZone, Utc};
use ramp_common::types::TenantId;
use ramp_compliance::reports::ctr::{
    CtrReport, CtrTransaction, CustomerInfo, FilingInstitution, TransactionType,
};
use ramp_compliance::reports::types::Report;

#[test]
fn test_ctr_report_structure() {
    let tenant_id = TenantId::new("tenant_1");
    let report_date = Utc.with_ymd_and_hms(2023, 10, 25, 12, 0, 0).unwrap();

    let institution = FilingInstitution {
        name: "Test Bank".to_string(),
        tax_id: "123".to_string(),
        address: "Test St".to_string(),
    };

    let transaction = CtrTransaction {
        transaction_id: "tx_1".to_string(),
        transaction_date: report_date,
        amount: 250_000_000,
        transaction_type: TransactionType::Deposit,
        customer: CustomerInfo {
            name: "John Doe".to_string(),
            id_number: "ID123".to_string(),
            id_type: "Passport".to_string(),
            nationality: "VN".to_string(),
            address: "Hanoi".to_string(),
        },
    };

    let report = CtrReport {
        report_id: "CTR-001".to_string(),
        report_date,
        filing_institution: institution,
        transactions: vec![transaction],
        total_amount: 250_000_000,
        currency: "VND".to_string(),
        tenant_id,
    };

    assert_eq!(report.report_type(), "ctr");
    assert_eq!(report.title(), "Currency Transaction Report - CTR-001");

    let header = report.format_csv_header();
    assert_eq!(header.len(), 8);
    assert_eq!(header[3], "Amount");

    let row = report.format_csv_row();
    assert_eq!(row[0], "CTR-001");
    assert_eq!(row[3], "250000000");
}

// NOTE: Integration tests for DB logic (filtering, date ranges) would typically go here
// using sqlx::test, but require a running Postgres instance.
//
// Example logic test plan:
// 1. Insert 3 transactions:
//    - Tx1: 150M VND (below threshold)
//    - Tx2: 250M VND (above threshold)
//    - Tx3: 300M VND (above threshold, but outside date range if testing range)
// 2. Run generate_ctr_report with threshold 200M
// 3. Assert Report contains only Tx2 (and Tx3 if in range)
// 4. Verify total_amount sum
