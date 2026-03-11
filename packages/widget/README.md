# @rampos/widget

Embeddable RampOS Widget SDK for checkout, KYC, and wallet functionality.

## Installation

```bash
npm install @rampos/widget
# or
yarn add @rampos/widget
# or
pnpm add @rampos/widget
```

## React Integration

```tsx
import { RampOSCheckout, RampOSKYC, RampOSWallet } from '@rampos/widget';

function App() {
  return (
    <RampOSCheckout
      apiKey="your-api-key"
      asset="USDC"
      amount={100}
      theme={{
        primaryColor: '#2563eb',
        backgroundColor: '#ffffff',
        borderRadius: '12px',
      }}
      onSuccess={(result) => {
        console.log('Payment successful:', result.transactionId);
      }}
      onError={(error) => {
        console.error('Payment failed:', error.message);
      }}
      onClose={() => {
        console.log('Widget closed');
      }}
    />
  );
}
```

### KYC Component

```tsx
<RampOSKYC
  apiKey="your-api-key"
  level="basic"
  onApproved={(result) => console.log('KYC approved:', result)}
  onRejected={(result) => console.log('KYC rejected:', result)}
  onClose={() => console.log('KYC closed')}
/>
```

### Wallet Component

```tsx
<RampOSWallet
  apiKey="your-api-key"
  defaultNetwork="polygon"
  showBalance={true}
  allowSend={true}
  allowReceive={true}
  onConnected={(wallet) => console.log('Connected:', wallet.address)}
  onTransactionSent={(tx) => console.log('TX sent:', tx.txHash)}
/>
```

## Vue Integration

```vue
<template>
  <rampos-checkout
    api-key="your-api-key"
    asset="USDC"
    amount="100"
    theme-primary="#2563eb"
    @rampos-success="handleSuccess"
    @rampos-error="handleError"
    @rampos-close="handleClose"
  />
</template>

<script setup>
import '@rampos/widget/dist/rampos-widget.umd.js';

function handleSuccess(event) {
  console.log('Payment successful:', event.detail);
}

function handleError(event) {
  console.error('Payment failed:', event.detail);
}

function handleClose() {
  console.log('Widget closed');
}
</script>
```

## Angular Integration

```typescript
// app.module.ts
import { CUSTOM_ELEMENTS_SCHEMA, NgModule } from '@angular/core';

@NgModule({
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class AppModule {}
```

```html
<!-- app.component.html -->
<rampos-checkout
  api-key="your-api-key"
  asset="USDC"
  amount="100"
  theme-primary="#2563eb"
  (rampos-success)="onSuccess($event)"
  (rampos-error)="onError($event)"
>
</rampos-checkout>
```

```typescript
// app.component.ts
import '@rampos/widget/dist/rampos-widget.umd.js';

export class AppComponent {
  onSuccess(event: CustomEvent) {
    console.log('Success:', event.detail);
  }
  onError(event: CustomEvent) {
    console.error('Error:', event.detail);
  }
}
```

## Vanilla HTML (CDN)

```html
<!DOCTYPE html>
<html>
<head>
  <script src="https://cdn.rampos.io/widget/v1/rampos-widget.umd.js"></script>
</head>
<body>
  <!-- Checkout Widget -->
  <rampos-checkout
    api-key="your-api-key"
    asset="USDC"
    amount="100"
    theme-primary="#2563eb"
    theme-bg="#ffffff"
    theme-radius="12px"
  ></rampos-checkout>

  <!-- KYC Widget -->
  <rampos-kyc
    api-key="your-api-key"
    level="basic"
  ></rampos-kyc>

  <!-- Wallet Widget -->
  <rampos-wallet
    api-key="your-api-key"
    default-network="polygon"
    show-balance="true"
    allow-send="true"
    allow-receive="true"
  ></rampos-wallet>

  <script>
    // Listen for events
    document.querySelector('rampos-checkout')
      .addEventListener('rampos-success', (e) => {
        console.log('Payment successful:', e.detail);
      });

    // Or use the global SDK
    RampOSWidget.init({ apiKey: 'your-api-key', environment: 'sandbox' });
  </script>
</body>
</html>
```

## CI/CD widget publish path

The repository includes a dedicated workflow at `.github/workflows/widget-cdn-publish.yml`.

What the workflow does:
- Builds `@rampos/widget` (`npm ci` + `npm run build` in `packages/widget`)
- Packages the npm tarball via `npm pack`
- Uploads both `dist/` bundles and tarball as GitHub artifact `widget-cdn-build`
- Publishes to npm on either:
  - tag refs matching `widget-v*`, or
  - manual dispatch (`workflow_dispatch`) with `publish_to_npm=true`

Required secrets/environment:
- `NPM_TOKEN` (repository secret): npm automation token with publish rights for `@rampos/widget`

Notes:
- Build-only artifact generation works without any secrets.
- npm publish step fails fast with a clear error if `NPM_TOKEN` is missing.

## Theming

Customize the widget appearance using theme props or CSS custom properties:

### React Theme Props

```tsx
<RampOSCheckout
  apiKey="key"
  theme={{
    primaryColor: '#8b5cf6',
    backgroundColor: '#1f2937',
    textColor: '#f9fafb',
    borderRadius: '16px',
    fontFamily: '"Poppins", sans-serif',
    errorColor: '#f43f5e',
    successColor: '#34d399',
  }}
/>
```

### CSS Custom Properties (Web Components)

```html
<rampos-checkout
  api-key="key"
  theme-primary="#8b5cf6"
  theme-bg="#1f2937"
  theme-text="#f9fafb"
  theme-radius="16px"
  theme-font="Poppins, sans-serif"
></rampos-checkout>
```

## Headless Config Layer

W11 adds a thin headless/config layer on top of the existing widget runtime. It does not create a second widget shell.

```ts
import {
  buildHeadlessCheckoutConfig,
  resolveHeadlessCheckoutConfig,
} from '@rampos/widget';

const localConfig = buildHeadlessCheckoutConfig({
  apiKey: 'your-api-key',
  asset: 'USDC',
  amount: 120,
  themeTokens: {
    accentColor: '#0f766e',
    surfaceColor: '#f8fafc',
  },
  headless: {
    emitState: true,
    flowId: 'checkout_headless_demo',
  },
});

const resolved = await resolveHeadlessCheckoutConfig({
  ...localConfig,
  remoteConfig: {
    url: 'https://example.com/widget-config.json',
  },
});
```

Use this layer to normalize checkout config, merge remote config, and map theme tokens while still rendering through `RampOSCheckout`, embed, or web-components.

## Event System

### React Callbacks

| Callback | Component | Description |
|----------|-----------|-------------|
| `onSuccess` | Checkout | Payment completed |
| `onError` | Checkout, KYC, Wallet | Error occurred |
| `onClose` | All | Widget closed |
| `onReady` | All | Widget initialized |
| `onSubmitted` | KYC | Documents submitted |
| `onApproved` | KYC | Identity verified |
| `onRejected` | KYC | Verification failed |
| `onConnected` | Wallet | Wallet connected |
| `onDisconnected` | Wallet | Wallet disconnected |
| `onTransactionSent` | Wallet | Transaction sent |
| `onTransactionConfirmed` | Wallet | Transaction confirmed |

### postMessage API (for iframe usage)

```javascript
import { onRampOSMessage } from '@rampos/widget';

const unsub = onRampOSMessage((event) => {
  switch (event.type) {
    case 'CHECKOUT_SUCCESS':
      console.log('Payment done:', event.payload);
      break;
    case 'CHECKOUT_ERROR':
      console.error('Payment error:', event.payload);
      break;
  }
});

// Cleanup
unsub();
```

## Web Components

| Element | Description |
|---------|-------------|
| `<rampos-checkout>` | Full checkout flow |
| `<rampos-kyc>` | KYC verification |
| `<rampos-wallet>` | Wallet management |

### Checkout Attributes

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `api-key` | string | Yes | Your RampOS API key |
| `amount` | number | No | Pre-filled amount |
| `asset` | string | No | Pre-selected asset (USDC, USDT, ETH, etc.) |
| `network` | string | No | Blockchain network |
| `wallet-address` | string | No | Receiving wallet |
| `environment` | string | No | `sandbox` or `production` |

### KYC Attributes

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `api-key` | string | Yes | Your RampOS API key |
| `user-id` | string | No | User identifier |
| `level` | string | No | `basic`, `advanced`, or `full` |

### Wallet Attributes

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `api-key` | string | Yes | Your RampOS API key |
| `default-network` | string | No | Default network |
| `show-balance` | boolean | No | Show token balances |
| `allow-send` | boolean | No | Enable send functionality |
| `allow-receive` | boolean | No | Enable receive functionality |

## Development

```bash
# Install dependencies
npm install

# Run dev server
npm run dev

# Run tests
npm test

# Build
npm run build

# Type check
npm run type-check
```

## License

MIT
