# RampOS Go Integration Example

This example demonstrates how to integrate with RampOS using the Go SDK.

## Prerequisites

- Go 1.21+
- RampOS SDK (local or module)
- A running RampOS instance

## Setup

1. Configure environment variables in `.env`:
   ```env
   RAMPOS_API_URL=http://localhost:3000
   RAMPOS_TENANT_ID=your-tenant-id
   RAMPOS_API_KEY=your-api-key
   RAMPOS_API_SECRET=your-api-secret
   RAMPOS_WEBHOOK_SECRET=your-webhook-secret
   ```

## Running

### Main Application
```bash
go run main.go
```

### Webhook Server
```bash
go run webhook/handler.go
```

## Features
- Payin flow
- Status polling
- Balance checking
- Payout flow
- Webhook signature verification
