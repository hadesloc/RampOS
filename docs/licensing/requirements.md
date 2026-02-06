# Vietnam Crypto Licensing Requirements

This document provides a comprehensive checklist of requirements for obtaining a cryptocurrency exchange license in Vietnam under Resolution 05/2025/NQ-CP.

---

## Resolution 05/2025/NQ-CP Summary

### Purpose

Resolution 05/2025/NQ-CP establishes the legal framework for:
- Licensing and supervision of cryptocurrency exchanges
- Customer protection requirements
- AML/CFT compliance standards
- Technical and operational standards
- Reporting and audit requirements

### Scope

The resolution applies to:
- Cryptocurrency-to-fiat exchanges
- Cryptocurrency-to-cryptocurrency exchanges operating in Vietnam
- Custody service providers
- Stablecoin issuers

### Key Provisions

| Article | Title | Summary |
|---------|-------|---------|
| Art. 5 | Licensing Authority | SBV is the primary licensing authority |
| Art. 7 | Capital Requirements | Minimum capital based on license type |
| Art. 10 | AML/CFT Obligations | Mandatory AML program and reporting |
| Art. 15 | Customer Protection | Segregated accounts, insurance requirements |
| Art. 20 | Technology Standards | Security, uptime, and audit requirements |
| Art. 25 | Reporting Obligations | Daily, monthly, and annual reports |
| Art. 30 | Penalties | Fines and license revocation conditions |

---

## Document Checklist

### 1. Corporate Documents

| # | Document | Description | Format |
|---|----------|-------------|--------|
| 1.1 | Business Registration Certificate | Valid enterprise registration | Certified copy |
| 1.2 | Company Charter | Current articles of association | Certified copy |
| 1.3 | Shareholder Register | Complete list with ownership % | Original |
| 1.4 | Board Resolution | Authorizing license application | Original, signed |
| 1.5 | Organization Chart | Current structure with names | PDF |
| 1.6 | Proof of Registered Office | Lease or ownership documents | Certified copy |

### 2. Capital and Financial Documents

| # | Document | Description | Format |
|---|----------|-------------|--------|
| 2.1 | Capital Proof | Bank statement showing minimum capital | Bank certified |
| 2.2 | Audited Financials | Last 2 years if operating | Auditor signed |
| 2.3 | Source of Funds | Documentation of capital origin | Notarized |
| 2.4 | Bank References | From Vietnamese banks | Original letters |
| 2.5 | Insurance Certificate | Required coverage for operations | Original policy |
| 2.6 | Reserve Fund Plan | Plan for 10% reserve maintenance | Business plan |

### 3. Key Personnel Documents

| # | Document | Description | Format |
|---|----------|-------------|--------|
| 3.1 | Director CVs | All directors with qualifications | Signed originals |
| 3.2 | Director ID Copies | CCCD/Passport for all directors | Certified copies |
| 3.3 | Police Clearances | Criminal background checks | Official certificates |
| 3.4 | Compliance Officer CV | Designated MLRO/Compliance head | Signed original |
| 3.5 | IT Security Lead CV | Technical security officer | Signed original |
| 3.6 | Professional References | For key personnel | Original letters |

### 4. AML/CFT Documents

| # | Document | Description | Format |
|---|----------|-------------|--------|
| 4.1 | AML/CFT Policy | Comprehensive AML program | PDF, board approved |
| 4.2 | KYC Procedures | Customer identification procedures | PDF |
| 4.3 | Transaction Monitoring | Rules and thresholds | PDF |
| 4.4 | SAR Filing Procedures | Suspicious activity reporting | PDF |
| 4.5 | Sanctions Screening | Screening procedures and providers | PDF |
| 4.6 | Training Program | Staff AML training plan | PDF |
| 4.7 | Risk Assessment | ML/TF risk assessment | PDF, dated |

### 5. Technical Documents

| # | Document | Description | Format |
|---|----------|-------------|--------|
| 5.1 | System Architecture | Technical infrastructure overview | PDF/Diagrams |
| 5.2 | Security Policy | Information security program | PDF |
| 5.3 | Penetration Test Report | Recent security assessment | Third-party report |
| 5.4 | Business Continuity Plan | Disaster recovery procedures | PDF |
| 5.5 | Data Protection Policy | GDPR/PDPA compliance | PDF |
| 5.6 | Uptime SLA | Service availability commitments | PDF |

### 6. Operational Documents

| # | Document | Description | Format |
|---|----------|-------------|--------|
| 6.1 | Business Plan | 3-year operational plan | PDF |
| 6.2 | Customer Terms | User agreement template | PDF |
| 6.3 | Fee Schedule | All fees and charges | PDF |
| 6.4 | Complaint Procedure | Customer complaint handling | PDF |
| 6.5 | Custody Procedures | Asset safekeeping procedures | PDF |
| 6.6 | Hot/Cold Wallet Policy | Crypto custody arrangements | PDF |

---

## Timeline and Deadlines

### Application Process

```
+------------+     +------------+     +------------+     +------------+
| Document   |---->| Submit to  |---->| SBV Review |---->| License    |
| Preparation|     | SBV        |     | Period     |     | Decision   |
+------------+     +------------+     +------------+     +------------+
   30-90 days          1 day          60-90 days         Final

                                                    |
                                    +---------------+---------------+
                                    |               |               |
                                    v               v               v
                               +--------+      +--------+      +--------+
                               |APPROVED|      |REJECTED|      |DEFERRED|
                               +--------+      +--------+      +--------+
```

### Key Deadlines

| Phase | Duration | Notes |
|-------|----------|-------|
| Document Preparation | 30-90 days | Gather and certify all documents |
| Initial Submission | Day 0 | Submit complete application package |
| Completeness Check | 5 business days | SBV confirms all documents received |
| Deficiency Response | 15 business days | If additional info requested |
| SBV Review | 60-90 days | Full review of application |
| Site Inspection | During review | SBV may conduct on-site visit |
| Decision | Day 90 (max) | License granted, rejected, or deferred |
| Appeal Period | 30 days | If rejected, can appeal |

### Grace Period for Existing Operators

| Date | Requirement |
|------|-------------|
| July 1, 2025 | Must submit application if currently operating |
| September 30, 2025 | Application must be complete (no deficiencies) |
| December 31, 2025 | Must have license or cease operations |

---

## Common Pitfalls and How to Avoid Them

### 1. Incomplete Documentation

**Problem**: Applications rejected for missing or incomplete documents.

**Solution**:
- Use the checklist above as a master tracker
- Have legal counsel review before submission
- Ensure all certifications are current (within 3 months)

**RampOS Feature**: Document tracking dashboard shows completion status.

```typescript
// Check document completeness
const status = await client.licensing.documents.getStatus();
console.log(`Documents complete: ${status.isComplete}`);
console.log(`Missing: ${status.missing.join(', ')}`);
```

### 2. Insufficient Capital Proof

**Problem**: Bank statements don't clearly show minimum capital is available and unencumbered.

**Solution**:
- Obtain a specific bank letter confirming available balance
- Ensure capital is not pledged as collateral
- Show capital has been in account for at least 30 days

**Example Bank Letter Request**:
```
To: [Bank Name]
Subject: Capital Confirmation for Regulatory Licensing

Please provide a letter confirming:
1. Account holder: [Company Name]
2. Account number: [XXXXXX]
3. Available balance as of [Date]: [Amount] VND
4. Confirmation funds are unencumbered
5. Statement that balance has been maintained for 30+ days
```

### 3. Inadequate AML Program

**Problem**: AML policy doesn't meet SBV specific requirements.

**Solution**:
- Include all elements required by Article 10
- Reference specific thresholds for Vietnam
- Document all screening providers used
- Include training records

**Required AML Policy Sections**:
1. Risk Assessment Methodology
2. Customer Due Diligence (CDD/EDD)
3. Transaction Monitoring Thresholds
4. Suspicious Activity Reporting
5. Sanctions Screening
6. Record Keeping
7. Training Program
8. Independent Audit
9. Governance and Oversight

### 4. Key Personnel Gaps

**Problem**: Directors or key staff don't meet fit and proper requirements.

**Requirements**:
- No criminal convictions related to fraud, money laundering, or financial crimes
- At least 5 years experience in financial services
- At least 2 directors must be Vietnam residents
- Compliance officer must have AML certification

**Solution**:
- Conduct background checks before appointment
- Obtain police clearances early (can take 2-4 weeks)
- Ensure compliance officer has recognized certification (CAMS, ICA, etc.)

### 5. Technical Infrastructure Gaps

**Problem**: Systems don't meet security or uptime requirements.

**Requirements**:
- 99.9% uptime SLA
- Annual penetration testing by approved vendor
- Data stored in Vietnam
- Encryption at rest and in transit
- Multi-signature for hot wallets

**Solution**:
- Use RampOS which is pre-configured for compliance
- Conduct penetration test 60 days before application
- Document all security controls

### 6. Missing Local Presence

**Problem**: No genuine local operations in Vietnam.

**Requirements**:
- Registered office in Vietnam
- At least 2 local directors
- Local bank accounts
- Local compliance officer (can be shared)

**Solution**:
- Establish local entity well before application
- Hire or appoint local directors
- Open accounts with major Vietnamese banks

### 7. Fee Structure Non-Compliance

**Problem**: Fee schedule doesn't meet transparency requirements.

**Requirements**:
- All fees must be disclosed upfront
- No hidden fees
- Fee changes require 30-day notice
- Maximum spread disclosures

**Solution**:
- Create comprehensive fee schedule
- Include in customer terms
- Implement fee change notification system

---

## SBV Contact Information

| Department | Contact | Purpose |
|------------|---------|---------|
| Licensing Division | licensing@sbv.gov.vn | Application submission |
| Compliance Division | compliance@sbv.gov.vn | Ongoing compliance |
| Technical Review | techreview@sbv.gov.vn | System inspections |
| General Inquiries | info@sbv.gov.vn | General questions |

**Physical Submission Address**:
```
State Bank of Vietnam
Licensing Division - Digital Assets
49 Ly Thai To Street
Hoan Kiem District
Hanoi, Vietnam
```

---

## Application Fees

| Fee Type | Amount (VND) | Payment Method |
|----------|--------------|----------------|
| Application Fee | 50,000,000 | Bank transfer |
| License Fee (CEX-A) | 500,000,000 | Bank transfer |
| License Fee (CEX-B) | 200,000,000 | Bank transfer |
| Annual Renewal | 100,000,000 | Bank transfer |
| Amendment Fee | 20,000,000 | Bank transfer |

---

## Checklist Template

Use this checklist to track your application progress:

```markdown
## License Application Checklist

### Corporate Documents
- [ ] 1.1 Business Registration Certificate
- [ ] 1.2 Company Charter
- [ ] 1.3 Shareholder Register
- [ ] 1.4 Board Resolution
- [ ] 1.5 Organization Chart
- [ ] 1.6 Proof of Registered Office

### Capital Documents
- [ ] 2.1 Capital Proof (Bank Statement)
- [ ] 2.2 Audited Financials
- [ ] 2.3 Source of Funds Declaration
- [ ] 2.4 Bank References
- [ ] 2.5 Insurance Certificate
- [ ] 2.6 Reserve Fund Plan

### Key Personnel
- [ ] 3.1 Director CVs
- [ ] 3.2 Director ID Copies
- [ ] 3.3 Police Clearances
- [ ] 3.4 Compliance Officer CV
- [ ] 3.5 IT Security Lead CV
- [ ] 3.6 Professional References

### AML/CFT Documents
- [ ] 4.1 AML/CFT Policy
- [ ] 4.2 KYC Procedures
- [ ] 4.3 Transaction Monitoring Rules
- [ ] 4.4 SAR Filing Procedures
- [ ] 4.5 Sanctions Screening
- [ ] 4.6 Training Program
- [ ] 4.7 Risk Assessment

### Technical Documents
- [ ] 5.1 System Architecture
- [ ] 5.2 Security Policy
- [ ] 5.3 Penetration Test Report
- [ ] 5.4 Business Continuity Plan
- [ ] 5.5 Data Protection Policy
- [ ] 5.6 Uptime SLA

### Operational Documents
- [ ] 6.1 Business Plan
- [ ] 6.2 Customer Terms
- [ ] 6.3 Fee Schedule
- [ ] 6.4 Complaint Procedure
- [ ] 6.5 Custody Procedures
- [ ] 6.6 Hot/Cold Wallet Policy
```

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
