import React, { useState, useEffect } from 'react';
import { WidgetTheme, CheckoutResult } from '../types';
import { RampOSEventEmitter } from '../utils/communication';
import { createPayinIntent } from '../api/checkout-api';

export interface CheckoutProps {
  apiKey: string;
  amount?: number;
  asset?: string;
  theme?: WidgetTheme;
  onSuccess?: (result: CheckoutResult) => void;
  onError?: (error: Error) => void;
  onClose?: () => void;
}

type Step = 'select-asset' | 'enter-amount' | 'summary' | 'processing' | 'success' | 'failed';

const Checkout: React.FC<CheckoutProps> = ({
  apiKey,
  amount: initialAmount,
  asset: initialAsset,
  theme,
  onSuccess,
  onError,
  onClose,
}) => {
  const [step, setStep] = useState<Step>(initialAsset ? (initialAmount ? 'summary' : 'enter-amount') : 'select-asset');
  const [asset, setAsset] = useState<string>(initialAsset || '');
  const [amount, setAmount] = useState<number>(initialAmount || 0);
  const [error, setError] = useState<string | null>(null);

  const emitter = RampOSEventEmitter.getInstance();

  useEffect(() => {
    if (initialAsset) setAsset(initialAsset);
  }, [initialAsset]);

  useEffect(() => {
    if (initialAmount) setAmount(initialAmount);
  }, [initialAmount]);

  useEffect(() => {
    // If both asset and amount are provided via props, and we are in early steps, verify if we should jump
    if (initialAsset && initialAmount && (step === 'select-asset' || step === 'enter-amount')) {
      setStep('summary');
    } else if (initialAsset && step === 'select-asset') {
      setStep('enter-amount');
    }
  }, [initialAsset, initialAmount]);

  const styles = {
    container: {
      fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
      padding: '20px',
      borderRadius: theme?.borderRadius || '8px',
      backgroundColor: theme?.backgroundColor || '#ffffff',
      color: theme?.textColor || '#111827',
      boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
      maxWidth: '400px',
      width: '100%',
      position: 'relative' as 'relative',
    },
    header: {
      fontSize: '18px',
      fontWeight: 600,
      marginBottom: '20px',
      borderBottom: '1px solid #e5e7eb',
      paddingBottom: '10px',
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
    },
    button: {
      backgroundColor: theme?.primaryColor || '#3b82f6',
      color: '#ffffff',
      border: 'none',
      borderRadius: '4px',
      padding: '10px 16px',
      fontSize: '14px',
      fontWeight: 500,
      cursor: 'pointer',
      width: '100%',
      marginTop: '16px',
      transition: 'background-color 0.2s',
    },
    input: {
      width: '100%',
      padding: '8px 12px',
      border: '1px solid #d1d5db',
      borderRadius: '4px',
      fontSize: '14px',
      marginBottom: '10px',
    },
    label: {
      display: 'block',
      fontSize: '14px',
      fontWeight: 500,
      marginBottom: '4px',
      color: '#374151',
    },
    closeButton: {
      background: 'none',
      border: 'none',
      fontSize: '20px',
      cursor: 'pointer',
      color: '#9ca3af',
    },
    select: {
      width: '100%',
      padding: '8px 12px',
      border: '1px solid #d1d5db',
      borderRadius: '4px',
      fontSize: '14px',
      marginBottom: '10px',
      backgroundColor: '#fff',
    },
    error: {
      color: '#ef4444',
      fontSize: '14px',
      marginBottom: '10px',
      padding: '8px',
      backgroundColor: '#fee2e2',
      borderRadius: '4px',
    }
  };

  useEffect(() => {
    emitter.emit('CHECKOUT_READY');
  }, []);

  const handleClose = () => {
    emitter.emit('CHECKOUT_CLOSE');
    if (onClose) onClose();
  };

  const handleConfirm = async () => {
    setStep('processing');
    try {
      const result = await createPayinIntent({
        apiKey,
        amount,
        asset,
      });

      setStep('success');
      emitter.emit('CHECKOUT_SUCCESS', result);
      if (onSuccess) onSuccess(result);
    } catch (err) {
      setStep('failed');
      const errorMessage = err instanceof Error ? err.message : 'Transaction failed';
      setError(errorMessage);
      emitter.emit('CHECKOUT_ERROR', { message: errorMessage });
      if (onError) onError(err instanceof Error ? err : new Error(errorMessage));
    }
  };

  const renderSelectAsset = () => (
    <div>
      <div style={styles.label}>Select Asset</div>
      <select
        style={styles.select}
        value={asset}
        onChange={(e) => setAsset(e.target.value)}
      >
        <option value="">Select...</option>
        <option value="USDC">USDC (Polygon)</option>
        <option value="USDT">USDT (Polygon)</option>
        <option value="ETH">Ethereum (Arbitrum)</option>
      </select>
      <button
        style={{...styles.button, opacity: !asset ? 0.5 : 1}}
        disabled={!asset}
        onClick={() => setStep('enter-amount')}
      >
        Next
      </button>
    </div>
  );

  const renderEnterAmount = () => (
    <div>
      <div style={styles.label}>Amount ({asset})</div>
      <input
        type="number"
        style={styles.input}
        value={amount || ''}
        onChange={(e) => setAmount(parseFloat(e.target.value))}
        placeholder="0.00"
        min="0"
      />
      <button
        style={{...styles.button, opacity: !amount ? 0.5 : 1}}
        disabled={!amount}
        onClick={() => setStep('summary')}
      >
        Review
      </button>
    </div>
  );

  const renderSummary = () => (
    <div>
      <div style={{ marginBottom: '16px', fontSize: '14px', color: '#4b5563' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
          <span>Asset:</span>
          <span style={{ fontWeight: 600 }}>{asset}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
          <span>Amount:</span>
          <span style={{ fontWeight: 600 }}>{amount}</span>
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', borderTop: '1px solid #e5e7eb', paddingTop: '8px', marginTop: '8px' }}>
          <span>Total:</span>
          <span style={{ fontWeight: 600 }}>{amount} {asset}</span>
        </div>
      </div>
      <button style={styles.button} onClick={handleConfirm}>
        Confirm Payment
      </button>
      <button
        style={{...styles.button, backgroundColor: 'transparent', color: '#6b7280', marginTop: '8px', border: '1px solid #d1d5db'}}
        onClick={() => setStep('enter-amount')}
      >
        Back
      </button>
    </div>
  );

  const renderProcessing = () => (
    <div style={{ textAlign: 'center', padding: '20px 0' }}>
      <div style={{
        width: '40px',
        height: '40px',
        border: `3px solid ${theme?.primaryColor || '#3b82f6'}`,
        borderTopColor: 'transparent',
        borderRadius: '50%',
        margin: '0 auto 16px',
        animation: 'spin 1s linear infinite'
      }} />
      <div style={{ fontWeight: 500 }}>Processing transaction...</div>
      <style>{`
        @keyframes spin {
          0% { transform: rotate(0deg); }
          100% { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );

  const renderSuccess = () => (
    <div style={{ textAlign: 'center', padding: '10px 0' }}>
      <div style={{
        color: '#10b981',
        fontSize: '48px',
        marginBottom: '10px'
      }}>✓</div>
      <h3 style={{ margin: '0 0 10px', color: '#111827' }}>Payment Successful!</h3>
      <p style={{ color: '#6b7280', fontSize: '14px' }}>Your transaction has been processed.</p>
      <button style={styles.button} onClick={handleClose}>
        Done
      </button>
    </div>
  );

  const renderFailed = () => (
    <div style={{ textAlign: 'center', padding: '10px 0' }}>
      <div style={{
        color: '#ef4444',
        fontSize: '48px',
        marginBottom: '10px'
      }}>✕</div>
      <h3 style={{ margin: '0 0 10px', color: '#111827' }}>Payment Failed</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '16px' }}>
        {error || 'Something went wrong. Please try again.'}
      </p>
      <button style={styles.button} onClick={() => setStep('summary')}>
        Try Again
      </button>
    </div>
  );

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <span>RampOS Checkout</span>
        <button style={styles.closeButton} onClick={handleClose}>×</button>
      </div>

      {step === 'select-asset' && renderSelectAsset()}
      {step === 'enter-amount' && renderEnterAmount()}
      {step === 'summary' && renderSummary()}
      {step === 'processing' && renderProcessing()}
      {step === 'success' && renderSuccess()}
      {step === 'failed' && renderFailed()}

      <div style={{
        marginTop: '20px',
        textAlign: 'center',
        fontSize: '12px',
        color: '#9ca3af'
      }}>
        Powered by RampOS
      </div>
    </div>
  );
};

export default Checkout;
