# Licensing API Guide

This guide covers the RampOS API endpoints for managing Vietnam crypto licensing workflows, document tracking, and compliance reporting.

---

## Authentication

### API Key Authentication

All licensing API requests require Bearer token authentication:

```bash
curl -X GET https://api.ramp.vn/v1/licensing/status \
  -H "Authorization: Bearer ramp_live_sk_your_api_key" \
  -H "Content-Type: application/json"
```

### API Key Types

| Key Type | Prefix | Permissions |
|----------|--------|-------------|
| Live Secret | `ramp_live_sk_` | Full access to licensing API |
| Test Secret | `ramp_test_sk_` | Sandbox licensing features |
| Read-only | `ramp_live_ro_` | Read-only access |

### HMAC Signature (Optional)

For enhanced security, sign requests with HMAC-SHA256:

```typescript
import crypto from 'crypto';

function signRequest(
  secretKey: string,
  timestamp: number,
  method: string,
  path: string,
  body: string
): string {
  const payload = `${timestamp}.${method}.${path}.${body}`;
  return crypto
    .createHmac('sha256', secretKey)
    .update(payload)
    .digest('hex');
}

// Usage
const timestamp = Math.floor(Date.now() / 1000);
const signature = signRequest(
  hmacSecret,
  timestamp,
  'POST',
  '/v1/licensing/documents',
  JSON.stringify(requestBody)
);

// Add headers
headers['X-Timestamp'] = timestamp.toString();
headers['X-Signature'] = signature;
```

---

## Endpoints

### License Status

#### Get Current License Status

```http
GET /v1/licensing/status
```

**Response**:
```json
{
  "licenseId": "lic_vn_2025_00042",
  "tenantId": "tenant_abc123",
  "licenseType": "CEX-B",
  "status": "ACTIVE",
  "issuedAt": "2025-07-15T00:00:00Z",
  "expiresAt": "2026-07-14T23:59:59Z",
  "conditions": [
    "VND on/off-ramp only",
    "Maximum 100B VND daily volume"
  ],
  "nextRenewalDate": "2026-06-15T00:00:00Z",
  "compliance": {
    "lastAudit": "2025-10-01T00:00:00Z",
    "nextAuditDue": "2026-01-01T00:00:00Z",
    "openIssues": 0
  }
}
```

**Status Values**:

| Status | Description |
|--------|-------------|
| `PENDING_APPLICATION` | Application not yet submitted |
| `UNDER_REVIEW` | Application submitted, awaiting decision |
| `ADDITIONAL_INFO_REQUIRED` | SBV requested additional information |
| `APPROVED_PENDING_ISSUANCE` | Approved, license being issued |
| `ACTIVE` | License is active and valid |
| `SUSPENDED` | License temporarily suspended |
| `REVOKED` | License permanently revoked |
| `EXPIRED` | License has expired |

---

### Application Management

#### Create License Application

```http
POST /v1/licensing/applications
```

**Request**:
```json
{
  "licenseType": "CEX-B",
  "applicant": {
    "companyName": "Vietnam Crypto Exchange JSC",
    "registrationNumber": "0123456789",
    "registeredAddress": "123 Nguyen Hue, District 1, HCMC",
    "contactEmail": "compliance@vce.vn",
    "contactPhone": "+84-28-1234-5678"
  },
  "directors": [
    {
      "fullName": "Nguyen Van A",
      "position": "CEO",
      "nationality": "VN",
      "idNumber": "079123456789",
      "isResident": true
    },
    {
      "fullName": "Tran Thi B",
      "position": "CFO",
      "nationality": "VN",
      "idNumber": "079987654321",
      "isResident": true
    }
  ],
  "capitalAmount": 100000000000,
  "capitalCurrency": "VND"
}
```

**Response**:
```json
{
  "applicationId": "app_vn_2025_00123",
  "status": "DRAFT",
  "createdAt": "2026-02-06T10:30:00Z",
  "requiredDocuments": [
    {
      "documentType": "BUSINESS_REGISTRATION",
      "status": "PENDING",
      "deadline": "2026-03-06T23:59:59Z"
    },
    {
      "documentType": "AML_POLICY",
      "status": "PENDING",
      "deadline": "2026-03-06T23:59:59Z"
    }
  ],
  "nextSteps": [
    "Upload all required documents",
    "Complete director verification",
    "Submit capital proof"
  ]
}
```

#### Get Application Status

```http
GET /v1/licensing/applications/{applicationId}
```

**Response**:
```json
{
  "applicationId": "app_vn_2025_00123",
  "status": "UNDER_REVIEW",
  "submittedAt": "2026-02-15T14:00:00Z",
  "reviewStage": "TECHNICAL_REVIEW",
  "estimatedDecisionDate": "2026-04-15T00:00:00Z",
  "documents": {
    "submitted": 24,
    "approved": 20,
    "pendingReview": 4,
    "rejected": 0
  },
  "timeline": [
    {
      "event": "APPLICATION_CREATED",
      "timestamp": "2026-02-06T10:30:00Z"
    },
    {
      "event": "DOCUMENTS_UPLOADED",
      "timestamp": "2026-02-10T16:45:00Z"
    },
    {
      "event": "APPLICATION_SUBMITTED",
      "timestamp": "2026-02-15T14:00:00Z"
    },
    {
      "event": "REVIEW_STARTED",
      "timestamp": "2026-02-20T09:00:00Z"
    }
  ]
}
```

#### Submit Application

```http
POST /v1/licensing/applications/{applicationId}/submit
```

**Request**:
```json
{
  "confirmations": {
    "documentsComplete": true,
    "informationAccurate": true,
    "termsAccepted": true
  },
  "submittedBy": {
    "name": "Nguyen Van A",
    "title": "CEO",
    "email": "ceo@vce.vn"
  }
}
```

**Response**:
```json
{
  "applicationId": "app_vn_2025_00123",
  "status": "SUBMITTED",
  "submittedAt": "2026-02-15T14:00:00Z",
  "referenceNumber": "SBV-2025-CEX-00123",
  "message": "Application submitted successfully. Expect review within 60-90 days."
}
```

---

### Document Management

#### List Required Documents

```http
GET /v1/licensing/applications/{applicationId}/documents
```

**Response**:
```json
{
  "applicationId": "app_vn_2025_00123",
  "documents": [
    {
      "documentType": "BUSINESS_REGISTRATION",
      "category": "CORPORATE",
      "status": "APPROVED",
      "uploadedAt": "2026-02-08T10:00:00Z",
      "approvedAt": "2026-02-20T11:30:00Z",
      "fileId": "file_abc123"
    },
    {
      "documentType": "AML_POLICY",
      "category": "AML_CFT",
      "status": "PENDING_REVIEW",
      "uploadedAt": "2026-02-10T14:00:00Z",
      "fileId": "file_def456"
    },
    {
      "documentType": "PENETRATION_TEST",
      "category": "TECHNICAL",
      "status": "NOT_UPLOADED",
      "required": true,
      "deadline": "2026-03-06T23:59:59Z"
    }
  ],
  "summary": {
    "total": 24,
    "uploaded": 22,
    "approved": 18,
    "pendingReview": 4,
    "rejected": 0,
    "notUploaded": 2
  }
}
```

#### Upload Document

```http
POST /v1/licensing/applications/{applicationId}/documents
Content-Type: multipart/form-data
```

**Request**:
```
--boundary
Content-Disposition: form-data; name="file"; filename="aml_policy.pdf"
Content-Type: application/pdf

<binary file data>
--boundary
Content-Disposition: form-data; name="documentType"

AML_POLICY
--boundary
Content-Disposition: form-data; name="metadata"

{"version": "2.0", "approvedBy": "Board of Directors", "approvedDate": "2026-01-15"}
--boundary--
```

**Response**:
```json
{
  "fileId": "file_ghi789",
  "documentType": "AML_POLICY",
  "filename": "aml_policy.pdf",
  "size": 2458624,
  "uploadedAt": "2026-02-10T14:00:00Z",
  "status": "PENDING_REVIEW",
  "checksum": "sha256:abc123def456..."
}
```

#### Get Document Status

```http
GET /v1/licensing/documents/{fileId}
```

**Response**:
```json
{
  "fileId": "file_ghi789",
  "documentType": "AML_POLICY",
  "filename": "aml_policy.pdf",
  "status": "APPROVED",
  "uploadedAt": "2026-02-10T14:00:00Z",
  "reviewedAt": "2026-02-22T09:30:00Z",
  "reviewedBy": "SBV Compliance Division",
  "comments": null,
  "downloadUrl": "https://storage.ramp.vn/docs/file_ghi789?token=xxx",
  "downloadUrlExpiresAt": "2026-02-06T11:30:00Z"
}
```

#### Replace Document

```http
PUT /v1/licensing/documents/{fileId}
Content-Type: multipart/form-data
```

Use this endpoint when a document is rejected and needs to be replaced.

---

### Compliance Reporting

#### Generate Compliance Report

```http
POST /v1/licensing/reports
```

**Request**:
```json
{
  "reportType": "MONTHLY_AML",
  "period": {
    "start": "2026-01-01T00:00:00Z",
    "end": "2026-01-31T23:59:59Z"
  },
  "format": "PDF"
}
```

**Response**:
```json
{
  "reportId": "rpt_2026_01_aml",
  "reportType": "MONTHLY_AML",
  "status": "GENERATING",
  "estimatedCompletionTime": "2026-02-06T10:35:00Z"
}
```

**Report Types**:

| Type | Description | Frequency |
|------|-------------|-----------|
| `DAILY_TRANSACTION` | Daily transaction summary | Daily |
| `MONTHLY_AML` | Monthly AML compliance report | Monthly |
| `QUARTERLY_REVIEW` | Quarterly compliance review | Quarterly |
| `ANNUAL_AUDIT` | Annual audit report | Annual |
| `SAR` | Suspicious Activity Report | As needed |
| `CTR` | Currency Transaction Report | As needed |

#### Get Report Status

```http
GET /v1/licensing/reports/{reportId}
```

**Response**:
```json
{
  "reportId": "rpt_2026_01_aml",
  "reportType": "MONTHLY_AML",
  "status": "COMPLETED",
  "generatedAt": "2026-02-06T10:34:00Z",
  "downloadUrl": "https://reports.ramp.vn/rpt_2026_01_aml.pdf?token=xxx",
  "downloadUrlExpiresAt": "2026-02-06T22:34:00Z",
  "submittedToSbv": true,
  "sbvSubmissionDate": "2026-02-05T08:00:00Z",
  "sbvReferenceNumber": "SBV-RPT-2026-01-00456"
}
```

#### List Reports

```http
GET /v1/licensing/reports?reportType=MONTHLY_AML&year=2026
```

**Response**:
```json
{
  "reports": [
    {
      "reportId": "rpt_2026_01_aml",
      "reportType": "MONTHLY_AML",
      "period": "2026-01",
      "status": "SUBMITTED",
      "submittedAt": "2026-02-05T08:00:00Z"
    }
  ],
  "pagination": {
    "total": 1,
    "page": 1,
    "perPage": 20
  }
}
```

---

### Director Management

#### List Directors

```http
GET /v1/licensing/directors
```

**Response**:
```json
{
  "directors": [
    {
      "directorId": "dir_001",
      "fullName": "Nguyen Van A",
      "position": "CEO",
      "nationality": "VN",
      "isResident": true,
      "verificationStatus": "VERIFIED",
      "appointedAt": "2024-01-15T00:00:00Z",
      "documents": {
        "idVerified": true,
        "backgroundCheckPassed": true,
        "qualificationsVerified": true
      }
    }
  ]
}
```

#### Add Director

```http
POST /v1/licensing/directors
```

**Request**:
```json
{
  "fullName": "Le Van C",
  "position": "CTO",
  "nationality": "VN",
  "idType": "CCCD",
  "idNumber": "079111222333",
  "dateOfBirth": "1985-06-15",
  "residentialAddress": "456 Le Loi, District 1, HCMC",
  "isResident": true,
  "appointmentDate": "2026-02-01"
}
```

---

## Error Handling

### Error Response Format

```json
{
  "error": {
    "code": "DOCUMENT_REJECTED",
    "message": "The uploaded document was rejected",
    "details": {
      "documentType": "AML_POLICY",
      "reason": "Document is older than 6 months",
      "suggestion": "Please upload a current version dated within the last 6 months"
    }
  },
  "requestId": "req_xyz789"
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_API_KEY` | 401 | API key is invalid or expired |
| `INSUFFICIENT_PERMISSIONS` | 403 | API key lacks required permissions |
| `APPLICATION_NOT_FOUND` | 404 | Application ID not found |
| `DOCUMENT_REJECTED` | 400 | Uploaded document was rejected |
| `DUPLICATE_APPLICATION` | 409 | Active application already exists |
| `DEADLINE_PASSED` | 400 | Submission deadline has passed |
| `VALIDATION_ERROR` | 422 | Request validation failed |
| `RATE_LIMITED` | 429 | Too many requests |
| `SBV_UNAVAILABLE` | 503 | SBV gateway temporarily unavailable |

### Retry Strategy

```typescript
async function callWithRetry<T>(
  fn: () => Promise<T>,
  maxRetries: number = 3
): Promise<T> {
  let lastError: Error;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await fn();
    } catch (error: any) {
      lastError = error;

      // Don't retry on client errors
      if (error.status >= 400 && error.status < 500) {
        throw error;
      }

      // Exponential backoff
      const delay = Math.min(1000 * Math.pow(2, attempt), 30000);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }

  throw lastError!;
}
```

---

## Webhooks Integration

### Configure Webhook Endpoint

```http
POST /v1/licensing/webhooks
```

**Request**:
```json
{
  "url": "https://your-server.com/webhooks/licensing",
  "events": [
    "application.status_changed",
    "document.reviewed",
    "license.issued",
    "license.expiring_soon",
    "report.due"
  ],
  "secret": "whsec_your_webhook_secret"
}
```

### Webhook Events

| Event | Description |
|-------|-------------|
| `application.status_changed` | Application status changed |
| `application.info_requested` | SBV requested additional information |
| `document.reviewed` | Document review completed |
| `document.expiring` | Document expiring soon |
| `license.issued` | License has been issued |
| `license.expiring_soon` | License expiring in 30 days |
| `license.suspended` | License has been suspended |
| `report.due` | Compliance report due soon |
| `report.overdue` | Compliance report is overdue |

### Webhook Payload Example

```json
{
  "id": "evt_123456",
  "type": "application.status_changed",
  "timestamp": "2026-02-20T09:00:00Z",
  "data": {
    "applicationId": "app_vn_2025_00123",
    "previousStatus": "SUBMITTED",
    "newStatus": "UNDER_REVIEW",
    "message": "Your application is now under review by SBV"
  }
}
```

### Verify Webhook Signature

```typescript
import crypto from 'crypto';

function verifyWebhookSignature(
  payload: string,
  signature: string,
  secret: string
): boolean {
  const [timestamp, hash] = signature.split(',');
  const expectedHash = crypto
    .createHmac('sha256', secret)
    .update(`${timestamp}.${payload}`)
    .digest('hex');

  return crypto.timingSafeEqual(
    Buffer.from(hash),
    Buffer.from(expectedHash)
  );
}
```

---

## SDK Examples

### TypeScript SDK

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: process.env.RAMPOS_API_KEY!,
  region: 'VN'
});

// Check license status
async function checkLicenseStatus() {
  const status = await client.licensing.getStatus();

  if (status.status === 'ACTIVE') {
    console.log(`License active until ${status.expiresAt}`);
  } else if (status.status === 'EXPIRED') {
    console.log('License has expired. Renewal required.');
  }
}

// Upload document
async function uploadDocument(filePath: string, docType: string) {
  const file = await fs.readFile(filePath);

  const result = await client.licensing.documents.upload({
    applicationId: 'app_vn_2025_00123',
    documentType: docType,
    file: file,
    filename: path.basename(filePath)
  });

  console.log(`Document uploaded: ${result.fileId}`);
  return result;
}

// Generate monthly report
async function generateMonthlyReport(year: number, month: number) {
  const report = await client.licensing.reports.create({
    reportType: 'MONTHLY_AML',
    period: {
      start: new Date(year, month - 1, 1),
      end: new Date(year, month, 0)
    }
  });

  // Wait for generation
  let status = report.status;
  while (status === 'GENERATING') {
    await new Promise(r => setTimeout(r, 5000));
    const updated = await client.licensing.reports.get(report.reportId);
    status = updated.status;
  }

  return await client.licensing.reports.get(report.reportId);
}
```

### Go SDK

```go
package main

import (
    "context"
    "fmt"
    "os"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    client := rampos.NewClient(os.Getenv("RAMPOS_API_KEY"))
    client.SetRegion("VN")

    ctx := context.Background()

    // Check license status
    status, err := client.Licensing.GetStatus(ctx)
    if err != nil {
        panic(err)
    }

    fmt.Printf("License status: %s\n", status.Status)

    // Upload document
    file, _ := os.Open("aml_policy.pdf")
    defer file.Close()

    doc, err := client.Licensing.Documents.Upload(ctx, &rampos.DocumentUploadRequest{
        ApplicationID: "app_vn_2025_00123",
        DocumentType:  "AML_POLICY",
        File:          file,
        Filename:      "aml_policy.pdf",
    })
    if err != nil {
        panic(err)
    }

    fmt.Printf("Document uploaded: %s\n", doc.FileID)
}
```

---

## Rate Limiting

### Limits

| Endpoint Category | Rate Limit | Window |
|-------------------|------------|--------|
| Status Queries | 100 requests | 1 minute |
| Document Upload | 10 requests | 1 minute |
| Report Generation | 5 requests | 1 hour |
| Application Submit | 1 request | 1 hour |

### Response Headers

```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1707216600
```

### Handling Rate Limits

```typescript
async function handleRateLimit(response: Response): Promise<void> {
  if (response.status === 429) {
    const resetTime = response.headers.get('X-RateLimit-Reset');
    const waitMs = (parseInt(resetTime!) * 1000) - Date.now();

    console.log(`Rate limited. Waiting ${waitMs}ms`);
    await new Promise(resolve => setTimeout(resolve, waitMs));
  }
}
```

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
