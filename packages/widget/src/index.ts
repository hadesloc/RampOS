// ============================================================
// @rampos/widget - Main Exports
// ============================================================

// React Components
export { default as RampOSCheckout } from './components/RampOSCheckout';
export { default as RampOSKYC } from './components/RampOSKYC';
export { default as RampOSWallet } from './components/RampOSWallet';

// Legacy alias
export { default as Checkout } from './components/RampOSCheckout';

// Shared UI
export { default as Button } from './components/shared/Button';
export { default as Input } from './components/shared/Input';
export { default as Modal } from './components/shared/Modal';

// Utilities
export { RampOSEventEmitter, onRampOSMessage } from './utils/events';
export { RampOSApiClient } from './utils/api';
export { resolveTheme, themeToCSS, themeToCSSVars } from './components/shared/theme';
export {
  buildHeadlessCheckoutConfig,
  resolveHeadlessCheckoutConfig,
} from './headless/index';
export { fetchRemoteCheckoutConfig, mergeCheckoutConfig } from './config/remote-config';
export {
  DEFAULT_THEME_TOKENS,
  themeTokensToTheme,
  resolveThemeTokens,
} from './config/theme-tokens';

// Types
export type {
  FiatCurrency,
  CryptoAsset,
  Network,
  KYCLevel,
  KYCStatus,
  PaymentMethod,
  WidgetTheme,
  WidgetThemeTokens,
  CheckoutConfig,
  HeadlessCheckoutConfig,
  HeadlessCheckoutOptions,
  CheckoutResult,
  RemoteWidgetConfig,
  KYCConfig,
  KYCResult,
  KYCDocument,
  WalletConfig,
  WalletInfo,
  TokenBalance,
  TransactionRecord,
  WidgetEventType,
  WidgetEvent,
  CheckoutCallbacks,
  KYCCallbacks,
  WalletCallbacks,
} from './types/index';

export { DEFAULT_THEME } from './types/index';
