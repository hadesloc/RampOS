# Compliance Best Practices

This guide provides recommendations for maintaining compliance with Vietnam crypto licensing requirements. Following these best practices will help ensure smooth operations and avoid regulatory issues.

---

## KYC Tier Recommendations

### Tier Structure Overview

RampOS implements a tiered KYC system that controls transaction limits based on verification level. This aligns with SBV requirements under Resolution 05/2025/NQ-CP.

| Tier | Verification Level | Recommended Use Case |
|------|--------------------|-----------------------|
| Tier 0 | Email/Phone only | Account creation, view-only |
| Tier 1 | Basic eKYC | Casual users, low-volume trading |
| Tier 2 | Enhanced KYC | Regular traders, medium volume |
| Tier 3 | KYB/Corporate | Businesses, high-net-worth individuals |

### Tier 0: View Only

**Requirements**:
- Email verification
- Phone verification (SMS OTP)

**Limits**:
- No transactions allowed
- Can view prices and markets
- Can deposit crypto (for identification purposes)

**Best Practices**:
- Use Tier 0 as an onboarding step
- Prompt users to upgrade before first transaction
- Set clear expectations about upgrade requirements

```typescript
// Prompt upgrade flow
if (user.kycTier === 0 && user.requestedTransaction) {
  await client.users.promptKycUpgrade({
    userId: user.id,
    targetTier: 1,
    message: 'Complete ID verification to start trading'
  });
}
```

### Tier 1: Basic eKYC

**Requirements**:
- Valid government ID (CCCD, Passport)
- Liveness check (selfie with ID)
- Name matching

**Limits (Recommended)**:
| Limit Type | Amount (VND) |
|------------|--------------|
| Daily Pay-in | 50,000,000 |
| Daily Pay-out | 20,000,000 |
| Single Transaction | 10,000,000 |
| Monthly Volume | 500,000,000 |

**Best Practices**:
- Accept CCCD (Citizen ID) as primary document for Vietnam residents
- Implement automated OCR for faster processing
- Set clear rejection reasons for failed verifications
- Allow re-submission with guidance on issues

**Document Acceptance Matrix**:
| Document | Vietnam Residents | Foreigners |
|----------|-------------------|------------|
| CCCD | Primary | N/A |
| Old ID Card | Secondary (until 2030) | N/A |
| Passport | Secondary | Primary |
| Driver's License | Not accepted | Not accepted |

### Tier 2: Enhanced KYC

**Requirements**:
- All Tier 1 requirements, plus:
- Proof of address (utility bill, bank statement)
- Source of funds declaration
- Video verification (optional but recommended)

**Limits (Recommended)**:
| Limit Type | Amount (VND) |
|------------|--------------|
| Daily Pay-in | 500,000,000 |
| Daily Pay-out | 200,000,000 |
| Single Transaction | 100,000,000 |
| Monthly Volume | 5,000,000,000 |

**Best Practices**:
- Accept documents dated within 3 months
- Cross-reference address with ID document
- Document source of funds for amounts over 200M VND
- Conduct enhanced due diligence on upgrade requests

**Acceptable Proof of Address**:
| Document | Validity Period |
|----------|-----------------|
| Utility Bill | 3 months |
| Bank Statement | 3 months |
| Tax Document | 12 months |
| Government Letter | 6 months |

### Tier 3: KYB/Corporate

**Requirements**:
- Business registration certificate
- Company charter/articles
- Beneficial ownership declaration (25%+ owners)
- Director ID verification
- Source of funds documentation
- Bank reference letter

**Limits (Recommended)**:
| Limit Type | Amount (VND) |
|------------|--------------|
| Daily Pay-in | Custom/Unlimited |
| Daily Pay-out | Custom/Unlimited |
| Single Transaction | 1,000,000,000 |
| Monthly Volume | Custom |

**Best Practices**:
- Verify all beneficial owners (25%+ ownership)
- Check directors against PEP lists
- Obtain company bank statements (6 months)
- Review business purpose and expected volumes
- Annual re-verification required

---

## Transaction Monitoring Thresholds

### Recommended Monitoring Rules

Configure your AML engine with these recommended thresholds for Vietnam compliance:

#### 1. Velocity Monitoring

| Rule | Threshold | Window | Severity |
|------|-----------|--------|----------|
| High Transaction Count | >5 transactions | 1 hour | High |
| Daily Volume Spike | >3x average daily | 24 hours | Medium |
| Night Trading Spike | >10 transactions | 10PM-6AM | Medium |

```typescript
// Configure velocity rule
await client.compliance.rules.configure({
  ruleType: 'VELOCITY',
  parameters: {
    maxTransactions: 5,
    windowMinutes: 60,
    minTotalAmount: 50_000_000, // 50M VND
    severity: 'HIGH'
  }
});
```

#### 2. Structuring Detection

| Rule | Threshold | Window | Severity |
|------|-----------|--------|----------|
| Near-Limit Transactions | 80-99% of limit | 24 hours | High |
| Round Number Pattern | Multiple 10M/50M/100M | 24 hours | Medium |
| Split Transactions | Same total, multiple parts | 1 hour | High |

**Detection Logic**:
```
IF transaction_count > 5
AND all_amounts BETWEEN (threshold * 0.80, threshold * 0.99)
AND time_window < 24 hours
THEN flag_as_structuring
```

#### 3. Large Transaction Reporting

| Transaction Type | Reporting Threshold | Action |
|------------------|---------------------|--------|
| Single Pay-in | 400,000,000 VND | CTR filing |
| Single Pay-out | 400,000,000 VND | CTR filing |
| Aggregate Daily | 1,000,000,000 VND | Enhanced monitoring |
| Crypto Withdrawal | 100,000 USDT equivalent | KYT check |

#### 4. Unusual Pattern Detection

| Pattern | Detection Criteria | Severity |
|---------|--------------------| ---------|
| Rapid Withdrawal | Withdraw >80% within 24h of deposit | High |
| Dormant Reactivation | First activity after 90+ days | Medium |
| New Account High Volume | >100M VND in first 7 days | Medium |
| Time Zone Anomaly | Activity outside normal hours | Low |

### Risk Scoring Configuration

| Score Range | Risk Level | Action Required |
|-------------|------------|-----------------|
| 0-30 | Low | Auto-approve |
| 31-50 | Medium | Enhanced monitoring |
| 51-70 | High | Manual review within 24h |
| 71-100 | Critical | Immediate review, possible freeze |

**Composite Risk Score Calculation**:
```
risk_score =
  (velocity_score * 0.25) +
  (structuring_score * 0.25) +
  (amount_score * 0.20) +
  (behavior_score * 0.15) +
  (profile_score * 0.15)
```

---

## Reporting Schedules

### Required Reports Overview

| Report | Frequency | Deadline | Recipient |
|--------|-----------|----------|-----------|
| Daily Transaction Summary | Daily | T+1 09:00 | Internal |
| Suspicious Activity Report (SAR) | As needed | Within 24 hours | SBV |
| Currency Transaction Report (CTR) | As needed | Within 3 days | SBV |
| Monthly AML Report | Monthly | 5th of month | SBV |
| Quarterly Compliance Review | Quarterly | 15 days after quarter | SBV |
| Annual Audit Report | Annual | 90 days after year-end | SBV |

### Daily Transaction Summary

**Content Requirements**:
- Total transaction count (by type)
- Total volume (by currency)
- Flagged transactions count
- Cases opened
- Cases resolved

**Automation**:
```typescript
// Schedule daily report
await client.licensing.reports.schedule({
  reportType: 'DAILY_TRANSACTION',
  schedule: '0 9 * * *', // 9 AM daily
  recipients: ['compliance@yourcompany.vn'],
  autoSubmit: false // Internal only
});
```

### Monthly AML Report

**Content Requirements**:
1. Transaction Statistics
   - Volume by type
   - User activity metrics
   - Cross-border transactions

2. AML Activity
   - Rules triggered (count by rule type)
   - Cases opened/closed
   - SARs filed
   - False positive rate

3. KYC Metrics
   - New users by tier
   - Upgrade/downgrade requests
   - Rejection rates and reasons

4. Risk Assessment
   - High-risk user count
   - Geographic risk distribution
   - Transaction risk distribution

**Deadline Management**:
```typescript
// Set up deadline reminders
await client.licensing.reports.setReminders({
  reportType: 'MONTHLY_AML',
  reminders: [
    { daysBefore: 5, channel: 'email' },
    { daysBefore: 2, channel: 'slack' },
    { daysBefore: 1, channel: 'sms' }
  ]
});
```

### Suspicious Activity Report (SAR)

**When to File**:
- Transaction involves known/suspected criminal activity
- Transaction has no apparent lawful purpose
- Transaction involves potential money laundering
- User behavior indicates fraud
- Sanctions match detected

**Filing Timeline**:
| Trigger | Filing Deadline |
|---------|-----------------|
| Suspected ongoing crime | Immediate (within 4 hours) |
| Completed suspicious transaction | Within 24 hours |
| Pattern detected during review | Within 48 hours |

**SAR Content Requirements**:
```typescript
interface SARReport {
  // Subject Information
  subject: {
    name: string;
    idNumber: string;
    address: string;
    accountNumbers: string[];
  };

  // Suspicious Activity
  activity: {
    type: string;
    dateRange: { start: Date; end: Date };
    amount: number;
    currency: string;
    description: string;
  };

  // Supporting Evidence
  evidence: {
    transactions: TransactionReference[];
    documents: DocumentReference[];
    narrative: string; // Detailed explanation
  };

  // Filer Information
  filer: {
    name: string;
    title: string;
    contactInfo: string;
    filingDate: Date;
  };
}
```

---

## Audit Preparation

### Annual Audit Requirements

SBV requires an annual independent audit covering:

1. **AML Program Effectiveness**
   - Rule coverage assessment
   - False positive/negative analysis
   - Case resolution timeliness

2. **KYC Process Review**
   - Verification accuracy
   - Document validity
   - Re-verification compliance

3. **Technology Controls**
   - System security
   - Data protection
   - Disaster recovery testing

4. **Training Records**
   - Staff training completion
   - Training content adequacy
   - Ongoing education

### Pre-Audit Checklist

```markdown
## 30 Days Before Audit

### Documentation
- [ ] Compile all AML policies and procedures
- [ ] Gather training records for all staff
- [ ] Prepare case file samples (10% random selection)
- [ ] Document all system changes since last audit
- [ ] Collect rule tuning records and justifications

### Testing
- [ ] Run internal compliance review
- [ ] Test disaster recovery procedures
- [ ] Verify data retention compliance
- [ ] Check report filing records

### Personnel
- [ ] Confirm key personnel availability
- [ ] Prepare org chart with roles/responsibilities
- [ ] Brief relevant staff on audit process

## 7 Days Before Audit

- [ ] Confirm auditor access credentials
- [ ] Set up dedicated audit workspace
- [ ] Prepare executive summary of year's activities
- [ ] Stage all requested documents
- [ ] Test remote access if applicable
```

### Common Audit Findings (and How to Avoid Them)

#### 1. Inadequate Transaction Monitoring

**Finding**: Rules too broad, high false positive rate

**Prevention**:
- Tune rules quarterly based on false positive data
- Document all tuning decisions
- Maintain rule change log with justification

```typescript
// Track rule effectiveness
const metrics = await client.compliance.rules.getMetrics({
  ruleId: 'velocity_check',
  period: 'last_quarter'
});

if (metrics.falsePositiveRate > 0.30) {
  // Alert compliance team to review thresholds
  await client.alerts.create({
    type: 'RULE_TUNING_NEEDED',
    ruleId: 'velocity_check',
    message: `False positive rate ${metrics.falsePositiveRate * 100}% exceeds target`
  });
}
```

#### 2. Incomplete KYC Records

**Finding**: Missing documents, expired verifications

**Prevention**:
- Implement document expiry tracking
- Automate re-verification reminders
- Block transactions for expired KYC

```typescript
// Schedule KYC refresh checks
await client.compliance.kyc.scheduleRefresh({
  checkFrequency: 'daily',
  expiryWarningDays: 30,
  autoBlockOnExpiry: true
});
```

#### 3. Delayed SAR Filing

**Finding**: SARs filed beyond 24-hour requirement

**Prevention**:
- Automate SAR generation for high-severity cases
- Set up escalation alerts
- Train staff on filing triggers

#### 4. Insufficient Training Records

**Finding**: Staff training not documented or outdated

**Prevention**:
- Implement training tracking system
- Require annual refresher training
- Test comprehension with quizzes

```typescript
// Training compliance check
const trainingStatus = await client.compliance.training.getStatus();

for (const staff of trainingStatus.staffMembers) {
  if (staff.lastTrainingDate < oneYearAgo) {
    await client.compliance.training.assignCourse({
      staffId: staff.id,
      courseId: 'AML_REFRESHER_2026',
      deadline: thirtyDaysFromNow
    });
  }
}
```

### Audit Documentation Retention

| Document Type | Retention Period | Storage Requirement |
|---------------|------------------|---------------------|
| Transaction Records | 10 years | Immutable, indexed |
| KYC Documents | 10 years after relationship end | Encrypted, accessible |
| SAR Filings | 10 years | Secure, auditable |
| Training Records | 5 years | Standard archive |
| Audit Reports | 10 years | Secure archive |
| Policy Versions | Indefinite | Version controlled |

---

## Ongoing Compliance Maintenance

### Weekly Tasks

- [ ] Review flagged transactions queue
- [ ] Close or escalate open cases
- [ ] Check document expiry alerts
- [ ] Verify system monitoring health

### Monthly Tasks

- [ ] Generate and review AML report
- [ ] Tune monitoring rules if needed
- [ ] Review staff training status
- [ ] Update risk assessments

### Quarterly Tasks

- [ ] Submit quarterly report to SBV
- [ ] Conduct internal compliance review
- [ ] Update policies if regulations changed
- [ ] Review and update risk scoring model

### Annual Tasks

- [ ] Prepare for external audit
- [ ] Submit annual report
- [ ] Renew staff certifications
- [ ] Complete enterprise-wide risk assessment
- [ ] Update training program content

---

## Emergency Procedures

### Sanctions Match Detected

1. **Immediately freeze** all user accounts
2. **Document** the match with evidence
3. **Notify** compliance officer within 1 hour
4. **File SAR** within 4 hours
5. **Do not** inform the user
6. **Report** to SBV same day

### Data Breach Response

1. **Contain** the breach immediately
2. **Notify** DPO and legal within 1 hour
3. **Document** scope and impact
4. **Notify** SBV within 72 hours
5. **Notify** affected users if required
6. **Conduct** post-incident review

### System Failure

1. **Activate** business continuity plan
2. **Switch** to backup systems if available
3. **Document** all transactions during outage
4. **Notify** SBV if outage >4 hours
5. **Reconcile** all transactions post-recovery

---

## Compliance Metrics Dashboard

Track these KPIs for ongoing compliance health:

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| SAR Filing Time | <24h | >18h | >24h |
| Case Resolution Time | <72h | >48h | >72h |
| KYC Approval Rate | >85% | <75% | <60% |
| False Positive Rate | <30% | >40% | >50% |
| Training Completion | 100% | <90% | <80% |
| Document Validity | 100% | <95% | <90% |
| Report On-time Rate | 100% | <100% | <90% |

---

## Resources

### Internal Resources
- [Compliance Architecture](../architecture/compliance.md)
- [API Reference](../api/README.md)
- [Webhooks Guide](../api/webhooks.md)

### External Resources
- SBV Licensing Division: licensing@sbv.gov.vn
- Vietnam AML Authority: aml@sbv.gov.vn
- FATF Recommendations: fatf-gafi.org

### Training Resources
- CAMS Certification: acams.org
- ICA Certification: int-comp.org
- Local Training: Contact compliance@ramp.vn

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
