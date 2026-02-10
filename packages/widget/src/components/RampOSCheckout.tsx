import React, { useState, useEffect, useCallback } from 'react';
import type { CheckoutConfig, CheckoutResult, CheckoutCallbacks, CryptoAsset, Network, PaymentMethod, WidgetTheme } from '../types/index';
import { RampOSEventEmitter } from '../utils/events';
import { resolveTheme } from './shared/theme';
import Button from './shared/Button';
import Input from './shared/Input';

export interface RampOSCheckoutProps extends CheckoutCallbacks {
  apiKey: string;
  amount?: number;
  asset?: CryptoAsset | string;
  fiatCurrency?: string;
  network?: Network;
  walletAddress?: string;
  theme?: WidgetTheme;
  environment?: 'sandbox' | 'production';
}

type CheckoutStep = 'select-asset' | 'enter-amount' | 'kyc-check' | 'payment-method' | 'summary' | 'processing' | 'success' | 'failed';

const SUPPORTED_ASSETS: { value: string; label: string; network: string }[] = [
  { value: 'USDC', label: 'USDC', network: 'polygon' },
  { value: 'USDT', label: 'USDT', network: 'polygon' },
  { value: 'ETH', label: 'Ethereum', network: 'arbitrum' },
  { value: 'MATIC', label: 'MATIC', network: 'polygon' },
  { value: 'VND_TOKEN', label: 'VND Token', network: 'polygon' },
];

const PAYMENT_METHODS: { value: PaymentMethod; label: string }[] = [
  { value: 'bank_transfer', label: 'Bank Transfer' },
  { value: 'card', label: 'Credit / Debit Card' },
  { value: 'mobile_money', label: 'Mobile Money (MoMo, ZaloPay)' },
];

const RampOSCheckout: React.FC<RampOSCheckoutProps> = ({
  apiKey,
  amount: initialAmount,
  asset: initialAsset,
  network: initialNetwork,
  walletAddress: initialWallet,
  theme: themeProp,
  onSuccess,
  onError,
  onClose,
  onReady,
}) => {
  const theme = resolveTheme(themeProp);
  const emitter = RampOSEventEmitter.getInstance();

  const [step, setStep] = useState<CheckoutStep>(() => {
    if (initialAsset && initialAmount) return 'payment-method';
    if (initialAsset) return 'enter-amount';
    return 'select-asset';
  });
  const [asset, setAsset] = useState(initialAsset || '');
  const [network, setNetwork] = useState(initialNetwork || '');
  const [amount, setAmount] = useState(initialAmount || 0);
  const [walletAddress, setWalletAddress] = useState(initialWallet || '');
  const [paymentMethod, setPaymentMethod] = useState<PaymentMethod | ''>('');
  const [error, setError] = useState<string | null>(null);
  const [kycVerified, setKycVerified] = useState(false);

  useEffect(() => {
    emitter.emit('CHECKOUT_READY');
    onReady?.();
  }, []);

  const handleClose = useCallback(() => {
    emitter.emit('CHECKOUT_CLOSE');
    onClose?.();
  }, [emitter, onClose]);

  const handleSelectAsset = (selectedAsset: string) => {
    setAsset(selectedAsset);
    const found = SUPPORTED_ASSETS.find(a => a.value === selectedAsset);
    if (found) setNetwork(found.network);
    setStep('enter-amount');
  };

  const handleAmountNext = () => {
    if (amount <= 0) {
      setError('Please enter a valid amount');
      return;
    }
    setError(null);
    // If amount > threshold, require KYC
    if (amount > 1000 && !kycVerified) {
      setStep('kyc-check');
    } else {
      setStep('payment-method');
    }
  };

  const handleKYCComplete = () => {
    setKycVerified(true);
    setStep('payment-method');
  };

  const handlePaymentMethodSelect = (method: PaymentMethod) => {
    setPaymentMethod(method);
    setStep('summary');
  };

  const handleConfirm = async () => {
    setStep('processing');
    setError(null);
    try {
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 2000));

      const result: CheckoutResult = {
        transactionId: `tx_${Date.now().toString(36)}_${Math.random().toString(36).substring(2, 8)}`,
        status: 'success',
        amount,
        asset,
        network,
        walletAddress,
        timestamp: Date.now(),
      };

      setStep('success');
      emitter.emit('CHECKOUT_SUCCESS', result);
      onSuccess?.(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Transaction failed';
      setError(message);
      setStep('failed');
      emitter.emit('CHECKOUT_ERROR', { message });
      onError?.(err instanceof Error ? err : new Error(message));
    }
  };

  // Styles
  const containerStyle: React.CSSProperties = {
    fontFamily: theme.fontFamily,
    padding: '24px',
    borderRadius: theme.borderRadius,
    backgroundColor: theme.backgroundColor,
    color: theme.textColor,
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
    maxWidth: '420px',
    width: '100%',
    position: 'relative',
  };

  const headerStyle: React.CSSProperties = {
    fontSize: '18px',
    fontWeight: 600,
    marginBottom: '20px',
    borderBottom: '1px solid #e5e7eb',
    paddingBottom: '12px',
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
  };

  const selectStyle: React.CSSProperties = {
    width: '100%',
    padding: '10px 12px',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    fontSize: '14px',
    marginBottom: '12px',
    backgroundColor: '#fff',
    cursor: 'pointer',
  };

  const assetCardStyle = (selected: boolean): React.CSSProperties => ({
    padding: '12px 16px',
    border: `2px solid ${selected ? theme.primaryColor : '#e5e7eb'}`,
    borderRadius: '8px',
    marginBottom: '8px',
    cursor: 'pointer',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: selected ? `${theme.primaryColor}08` : '#fff',
    transition: 'all 0.15s',
  });

  const paymentCardStyle = (selected: boolean): React.CSSProperties => ({
    ...assetCardStyle(selected),
  });

  const summaryRowStyle: React.CSSProperties = {
    display: 'flex',
    justifyContent: 'space-between',
    marginBottom: '8px',
    fontSize: '14px',
    color: '#4b5563',
  };

  const errorBoxStyle: React.CSSProperties = {
    color: theme.errorColor,
    fontSize: '13px',
    padding: '8px 12px',
    backgroundColor: '#fee2e2',
    borderRadius: '6px',
    marginBottom: '12px',
  };

  // Step renders
  const renderSelectAsset = () => (
    <div>
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '12px', color: '#374151' }}>
        Select an asset to purchase
      </div>
      {SUPPORTED_ASSETS.map(a => (
        <div
          key={a.value}
          style={assetCardStyle(asset === a.value)}
          onClick={() => handleSelectAsset(a.value)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') handleSelectAsset(a.value); }}
        >
          <div>
            <div style={{ fontWeight: 600 }}>{a.label}</div>
            <div style={{ fontSize: '12px', color: '#9ca3af' }}>{a.network}</div>
          </div>
          {asset === a.value && <span style={{ color: theme.primaryColor, fontWeight: 700 }}>&#10003;</span>}
        </div>
      ))}
    </div>
  );

  const renderEnterAmount = () => (
    <div>
      <Input
        label={`Amount (${asset})`}
        type="number"
        value={amount || ''}
        onChange={(e) => setAmount(parseFloat(e.target.value) || 0)}
        placeholder="0.00"
        min="0"
        error={error || undefined}
      />
      <Input
        label="Wallet Address"
        type="text"
        value={walletAddress}
        onChange={(e) => setWalletAddress(e.target.value)}
        placeholder="0x..."
        helpText="Your receiving wallet address"
      />
      <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('select-asset')} primaryColor={theme.primaryColor}>
          Back
        </Button>
        <Button onClick={handleAmountNext} primaryColor={theme.primaryColor}>
          Continue
        </Button>
      </div>
    </div>
  );

  const renderKYCCheck = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{ fontSize: '24px', marginBottom: '12px' }}>ID</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Identity Verification Required</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '20px' }}>
        Transactions over $1,000 require KYC verification. This is a quick process.
      </p>
      <Button onClick={handleKYCComplete} primaryColor={theme.primaryColor}>
        Complete Verification
      </Button>
      <div style={{ marginTop: '8px' }}>
        <Button variant="ghost" onClick={() => setStep('enter-amount')} primaryColor={theme.primaryColor}>
          Go Back
        </Button>
      </div>
    </div>
  );

  const renderPaymentMethod = () => (
    <div>
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '12px', color: '#374151' }}>
        Select payment method
      </div>
      {PAYMENT_METHODS.map(m => (
        <div
          key={m.value}
          style={paymentCardStyle(paymentMethod === m.value)}
          onClick={() => handlePaymentMethodSelect(m.value)}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') handlePaymentMethodSelect(m.value); }}
        >
          <span style={{ fontWeight: 500 }}>{m.label}</span>
          {paymentMethod === m.value && <span style={{ color: theme.primaryColor, fontWeight: 700 }}>&#10003;</span>}
        </div>
      ))}
      <div style={{ marginTop: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('enter-amount')} primaryColor={theme.primaryColor}>
          Back
        </Button>
      </div>
    </div>
  );

  const renderSummary = () => (
    <div>
      <div style={{ marginBottom: '16px' }}>
        <div style={summaryRowStyle}>
          <span>Asset</span>
          <span style={{ fontWeight: 600 }}>{asset}</span>
        </div>
        <div style={summaryRowStyle}>
          <span>Network</span>
          <span style={{ fontWeight: 600 }}>{network}</span>
        </div>
        <div style={summaryRowStyle}>
          <span>Amount</span>
          <span style={{ fontWeight: 600 }}>{amount} {asset}</span>
        </div>
        <div style={summaryRowStyle}>
          <span>Payment</span>
          <span style={{ fontWeight: 600 }}>{PAYMENT_METHODS.find(m => m.value === paymentMethod)?.label}</span>
        </div>
        {walletAddress && (
          <div style={summaryRowStyle}>
            <span>Wallet</span>
            <span style={{ fontWeight: 600, fontSize: '12px', wordBreak: 'break-all' }}>
              {walletAddress.substring(0, 6)}...{walletAddress.substring(walletAddress.length - 4)}
            </span>
          </div>
        )}
        <div style={{ ...summaryRowStyle, borderTop: '1px solid #e5e7eb', paddingTop: '8px', marginTop: '8px', fontWeight: 600 }}>
          <span>Total</span>
          <span>{amount} {asset}</span>
        </div>
      </div>
      <Button onClick={handleConfirm} primaryColor={theme.primaryColor}>
        Confirm Payment
      </Button>
      <div style={{ marginTop: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('payment-method')} primaryColor={theme.primaryColor}>
          Back
        </Button>
      </div>
    </div>
  );

  const renderProcessing = () => (
    <div style={{ textAlign: 'center', padding: '24px 0' }}>
      <div style={{
        width: '44px',
        height: '44px',
        border: `3px solid ${theme.primaryColor}`,
        borderTopColor: 'transparent',
        borderRadius: '50%',
        margin: '0 auto 16px',
        animation: 'rampos-spin 0.8s linear infinite',
      }} />
      <div style={{ fontWeight: 500, color: '#374151' }}>Processing your transaction...</div>
      <div style={{ fontSize: '13px', color: '#9ca3af', marginTop: '4px' }}>This may take a moment</div>
      <style>{`@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }`}</style>
    </div>
  );

  const renderSuccess = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{ color: theme.successColor, fontSize: '48px', marginBottom: '8px' }}>&#10003;</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Payment Successful!</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '20px' }}>
        Your {amount} {asset} purchase has been processed.
      </p>
      <Button onClick={handleClose} primaryColor={theme.primaryColor}>Done</Button>
    </div>
  );

  const renderFailed = () => (
    <div style={{ textAlign: 'center', padding: '16px 0' }}>
      <div style={{ color: theme.errorColor, fontSize: '48px', marginBottom: '8px' }}>&#10007;</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Payment Failed</h3>
      {error && <div style={errorBoxStyle}>{error}</div>}
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '20px' }}>
        Something went wrong. Please try again.
      </p>
      <Button onClick={() => setStep('summary')} primaryColor={theme.primaryColor}>Try Again</Button>
      <div style={{ marginTop: '8px' }}>
        <Button variant="ghost" onClick={handleClose} primaryColor={theme.primaryColor}>Cancel</Button>
      </div>
    </div>
  );

  return (
    <div style={containerStyle} data-testid="rampos-checkout">
      <div style={headerStyle}>
        <span>RampOS Checkout</span>
        <button
          onClick={handleClose}
          style={{ background: 'none', border: 'none', fontSize: '20px', cursor: 'pointer', color: '#9ca3af' }}
          aria-label="Close"
        >
          x
        </button>
      </div>

      {step === 'select-asset' && renderSelectAsset()}
      {step === 'enter-amount' && renderEnterAmount()}
      {step === 'kyc-check' && renderKYCCheck()}
      {step === 'payment-method' && renderPaymentMethod()}
      {step === 'summary' && renderSummary()}
      {step === 'processing' && renderProcessing()}
      {step === 'success' && renderSuccess()}
      {step === 'failed' && renderFailed()}

      <div style={{ marginTop: '20px', textAlign: 'center', fontSize: '11px', color: '#9ca3af' }}>
        Powered by RampOS
      </div>
    </div>
  );
};

export default RampOSCheckout;
