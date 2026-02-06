# Vietnam Crypto Licensing Guide

Welcome to the RampOS Vietnam crypto licensing documentation. This guide helps exchange operators understand and navigate the Vietnam cryptocurrency licensing requirements under Resolution 05/2025/NQ-CP.

---

## Quick Navigation

| Document | Description | Audience |
|----------|-------------|----------|
| [Requirements Checklist](./requirements.md) | SBV requirements and document checklist | Compliance Officers |
| [API Guide](./api-guide.md) | License management API usage | Developers |
| [Best Practices](./compliance-best-practices.md) | Compliance recommendations | All Teams |

---

## Overview

Vietnam's State Bank (SBV) established a regulatory framework for cryptocurrency exchanges through Resolution 05/2025/NQ-CP, effective from July 1, 2025. This framework creates a licensing regime that allows exchanges to operate legally within Vietnam.

### Key Dates

| Milestone | Date | Description |
|-----------|------|-------------|
| Resolution Published | January 15, 2025 | Resolution 05/2025/NQ-CP published |
| Applications Open | April 1, 2025 | SBV begins accepting license applications |
| Framework Effective | July 1, 2025 | Full regulatory framework in effect |
| Grace Period Ends | December 31, 2025 | Existing operators must be licensed |

### License Types

| License Type | Code | Description | Min Capital (VND) |
|--------------|------|-------------|-------------------|
| Full Exchange | CEX-A | Full trading operations | 500 billion |
| Limited Exchange | CEX-B | VND on/off-ramp only | 100 billion |
| Custody Services | CUST | Digital asset custody | 200 billion |
| Stablecoin Issuer | STBL | VND-backed stablecoin | 1 trillion |

---

## RampOS Integration

RampOS provides built-in support for Vietnam licensing compliance:

### Features

- **License Status Tracking**: Monitor application status via API
- **Document Management**: Store and manage required documents
- **Compliance Reporting**: Generate required regulatory reports
- **AML Integration**: Built-in AML rules aligned with SBV requirements
- **KYC Tiers**: Preconfigured tiers matching SBV guidelines

### Architecture

```
+------------------+     +------------------+     +------------------+
|   Exchange       |---->|     RampOS       |---->|  SBV Reporting   |
|   Operations     |     |  Compliance API  |     |    Gateway       |
+------------------+     +--------+---------+     +------------------+
                                  |
                    +-------------+-------------+
                    |             |             |
                    v             v             v
             +----------+  +-----------+  +------------+
             | License  |  | Document  |  | Compliance |
             | Status   |  | Storage   |  |  Reports   |
             +----------+  +-----------+  +------------+
```

---

## Getting Started

### Step 1: Understand Requirements

Start by reviewing the [Requirements Checklist](./requirements.md) to understand what documents and processes you need.

### Step 2: Prepare Documentation

Gather all required documents:
- Business license
- AML/CFT policy
- Capital proof
- Technical infrastructure documentation
- Key personnel information

### Step 3: Configure RampOS

Set up your RampOS instance for Vietnam compliance:

```bash
# Enable Vietnam licensing module
export RAMPOS_LICENSING_REGION=VN
export RAMPOS_LICENSING_TYPE=CEX-B

# Start with licensing features
docker-compose -f docker-compose.licensing.yml up -d
```

### Step 4: Submit Application

Use the RampOS API to manage your license application:

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  region: 'VN'
});

// Check license status
const status = await client.licensing.getStatus();
console.log(`License status: ${status.state}`);
```

---

## Compliance Requirements Summary

### Capital Requirements

| Requirement | CEX-A | CEX-B | CUST |
|-------------|-------|-------|------|
| Minimum Capital | 500B VND | 100B VND | 200B VND |
| Reserve Fund | 10% | 10% | 5% |
| Insurance/Bond | 50B VND | 10B VND | 20B VND |

### Operational Requirements

- **Local Entity**: Must operate through a Vietnam-registered company
- **Local Directors**: At least 2 directors must be Vietnam residents
- **Local Custody**: Customer VND funds must be held in Vietnam banks
- **Data Localization**: Customer data must be stored in Vietnam

### Reporting Requirements

| Report | Frequency | Deadline |
|--------|-----------|----------|
| Transaction Summary | Daily | T+1 |
| Suspicious Activity | Immediate | Within 24 hours |
| AML Compliance | Monthly | 5th of month |
| Quarterly Review | Quarterly | 15 days after quarter |
| Annual Audit | Annual | 90 days after year-end |

---

## Support

- **Documentation**: This guide
- **Compliance Hotline**: compliance@ramp.vn
- **SBV Inquiries**: Forward to regulatory@ramp.vn
- **Technical Support**: api-support@ramp.vn

---

## Related Documentation

- [Compliance Architecture](../architecture/compliance.md) - Technical compliance implementation
- [API Reference](../api/README.md) - Complete API documentation
- [Security Guide](../security/README.md) - Security best practices

---

## Disclaimer

This documentation is for informational purposes only and does not constitute legal advice. Exchange operators should consult with qualified legal counsel regarding licensing requirements and compliance obligations.

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
