import React, { useState, useEffect, useCallback } from 'react';
import type { WalletConfig, WalletCallbacks, WalletInfo, TokenBalance, TransactionRecord, Network, WidgetTheme } from '../types/index';
import { RampOSEventEmitter } from '../utils/events';
import { resolveTheme } from './shared/theme';
import Button from './shared/Button';
import Input from './shared/Input';

export interface RampOSWalletProps extends WalletCallbacks {
  apiKey: string;
  userId?: string;
  defaultNetwork?: Network;
  theme?: WidgetTheme;
  environment?: 'sandbox' | 'production';
  showBalance?: boolean;
  allowSend?: boolean;
  allowReceive?: boolean;
}

type WalletStep = 'connect' | 'dashboard' | 'send' | 'receive' | 'history';

const MOCK_BALANCES: TokenBalance[] = [
  { asset: 'USDC', balance: '1,250.00', decimals: 6, usdValue: 1250.0 },
  { asset: 'ETH', balance: '0.5432', decimals: 18, usdValue: 1358.0 },
  { asset: 'MATIC', balance: '500.00', decimals: 18, usdValue: 450.0 },
  { asset: 'VND_TOKEN', balance: '25,000,000', decimals: 18, usdValue: 1000.0 },
];

const MOCK_HISTORY: TransactionRecord[] = [
  { id: 'tx1', type: 'receive', asset: 'USDC', amount: '500', from: '0xabc...def', to: '0x123...456', status: 'confirmed', timestamp: Date.now() - 86400000, txHash: '0xfeed...' },
  { id: 'tx2', type: 'send', asset: 'ETH', amount: '0.1', from: '0x123...456', to: '0xdef...abc', status: 'confirmed', timestamp: Date.now() - 172800000, txHash: '0xbeef...' },
  { id: 'tx3', type: 'receive', asset: 'MATIC', amount: '200', from: '0xabc...789', to: '0x123...456', status: 'pending', timestamp: Date.now() - 3600000 },
];

const NETWORKS: { value: Network; label: string }[] = [
  { value: 'polygon', label: 'Polygon' },
  { value: 'arbitrum', label: 'Arbitrum' },
  { value: 'optimism', label: 'Optimism' },
  { value: 'ethereum', label: 'Ethereum' },
  { value: 'base', label: 'Base' },
];

const RampOSWallet: React.FC<RampOSWalletProps> = ({
  apiKey,
  userId,
  defaultNetwork = 'polygon',
  theme: themeProp,
  showBalance = true,
  allowSend = true,
  allowReceive = true,
  onConnected,
  onDisconnected,
  onTransactionSent,
  onTransactionConfirmed,
  onError,
  onClose,
  onReady,
}) => {
  const theme = resolveTheme(themeProp);
  const emitter = RampOSEventEmitter.getInstance();

  const [step, setStep] = useState<WalletStep>('connect');
  const [network, setNetwork] = useState<Network>(defaultNetwork);
  const [walletAddress, setWalletAddress] = useState('');
  const [balances, setBalances] = useState<TokenBalance[]>([]);
  const [history, setHistory] = useState<TransactionRecord[]>([]);
  const [sendTo, setSendTo] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [sendAsset, setSendAsset] = useState('USDC');
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    emitter.emit('WALLET_READY');
    onReady?.();
  }, []);

  const handleClose = useCallback(() => {
    emitter.emit('WALLET_CLOSE');
    onClose?.();
  }, [emitter, onClose]);

  const handleConnect = async () => {
    try {
      // Simulate wallet connection
      await new Promise(resolve => setTimeout(resolve, 1000));
      const address = '0x' + Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join('');
      setWalletAddress(address);
      setBalances(MOCK_BALANCES);
      setHistory(MOCK_HISTORY);
      setStep('dashboard');

      const walletInfo: WalletInfo = {
        address,
        network,
        balances: MOCK_BALANCES,
      };

      emitter.emit('WALLET_CONNECTED', walletInfo);
      onConnected?.(walletInfo);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Connection failed';
      setError(message);
      emitter.emit('WALLET_ERROR', { message });
      onError?.(err instanceof Error ? err : new Error(message));
    }
  };

  const handleDisconnect = () => {
    setWalletAddress('');
    setBalances([]);
    setHistory([]);
    setStep('connect');
    emitter.emit('WALLET_DISCONNECTED');
    onDisconnected?.();
  };

  const handleSend = async () => {
    if (!sendTo || !sendAmount) {
      setError('Please fill in all fields');
      return;
    }
    setSending(true);
    setError(null);
    try {
      await new Promise(resolve => setTimeout(resolve, 2000));

      const tx: TransactionRecord = {
        id: `tx_${Date.now().toString(36)}`,
        type: 'send',
        asset: sendAsset,
        amount: sendAmount,
        from: walletAddress,
        to: sendTo,
        status: 'pending',
        timestamp: Date.now(),
        txHash: '0x' + Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join(''),
      };

      setHistory(prev => [tx, ...prev]);
      emitter.emit('WALLET_TX_SENT', tx);
      onTransactionSent?.(tx);

      // Simulate confirmation
      setTimeout(() => {
        const confirmed = { ...tx, status: 'confirmed' as const };
        setHistory(prev => prev.map(t => t.id === tx.id ? confirmed : t));
        emitter.emit('WALLET_TX_CONFIRMED', confirmed);
        onTransactionConfirmed?.(confirmed);
      }, 3000);

      setSendTo('');
      setSendAmount('');
      setStep('dashboard');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Transaction failed';
      setError(message);
      emitter.emit('WALLET_ERROR', { message });
      onError?.(err instanceof Error ? err : new Error(message));
    } finally {
      setSending(false);
    }
  };

  const containerStyle: React.CSSProperties = {
    fontFamily: theme.fontFamily,
    padding: '24px',
    borderRadius: theme.borderRadius,
    backgroundColor: theme.backgroundColor,
    color: theme.textColor,
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1)',
    maxWidth: '420px',
    width: '100%',
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

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: '8px 16px',
    fontSize: '13px',
    fontWeight: active ? 600 : 400,
    color: active ? theme.primaryColor : '#6b7280',
    borderBottom: active ? `2px solid ${theme.primaryColor}` : '2px solid transparent',
    cursor: 'pointer',
    background: 'none',
    border: 'none',
    transition: 'all 0.15s',
  });

  const balanceCardStyle: React.CSSProperties = {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '12px 0',
    borderBottom: '1px solid #f3f4f6',
  };

  const renderConnect = () => (
    <div style={{ textAlign: 'center', padding: '24px 0' }}>
      <div style={{ fontSize: '32px', marginBottom: '12px', color: theme.primaryColor }}>W</div>
      <h3 style={{ margin: '0 0 8px', color: '#111827' }}>Connect Wallet</h3>
      <p style={{ color: '#6b7280', fontSize: '14px', marginBottom: '16px' }}>
        Connect your RampOS wallet to view balances and send transactions.
      </p>
      <div style={{ marginBottom: '16px' }}>
        <label style={{ fontSize: '13px', fontWeight: 500, color: '#374151', display: 'block', marginBottom: '6px' }}>Network</label>
        <select
          value={network}
          onChange={e => setNetwork(e.target.value as Network)}
          style={{
            width: '100%', padding: '8px 12px', border: '1px solid #d1d5db',
            borderRadius: '6px', fontSize: '14px', backgroundColor: '#fff',
          }}
        >
          {NETWORKS.map(n => <option key={n.value} value={n.value}>{n.label}</option>)}
        </select>
      </div>
      {error && (
        <div style={{ color: theme.errorColor, fontSize: '13px', padding: '8px 12px', backgroundColor: '#fee2e2', borderRadius: '6px', marginBottom: '12px' }}>
          {error}
        </div>
      )}
      <Button onClick={handleConnect} primaryColor={theme.primaryColor}>
        Connect Wallet
      </Button>
    </div>
  );

  const renderDashboard = () => {
    const totalUsd = balances.reduce((sum, b) => sum + (b.usdValue || 0), 0);

    return (
      <div>
        {/* Wallet address */}
        <div style={{ backgroundColor: '#f9fafb', borderRadius: '8px', padding: '12px', marginBottom: '16px', fontSize: '13px' }}>
          <div style={{ color: '#6b7280', marginBottom: '4px' }}>Wallet Address</div>
          <div style={{ fontWeight: 500, wordBreak: 'break-all' }}>
            {walletAddress.substring(0, 10)}...{walletAddress.substring(walletAddress.length - 8)}
          </div>
          <div style={{ color: '#9ca3af', fontSize: '12px', marginTop: '4px' }}>Network: {network}</div>
        </div>

        {/* Tabs */}
        <div style={{ display: 'flex', borderBottom: '1px solid #e5e7eb', marginBottom: '16px' }}>
          <button style={tabStyle(step === 'dashboard')} onClick={() => setStep('dashboard')}>Balances</button>
          <button style={tabStyle(step === 'history')} onClick={() => setStep('history')}>History</button>
        </div>

        {/* Total Balance */}
        {showBalance && (
          <div style={{ textAlign: 'center', marginBottom: '16px' }}>
            <div style={{ color: '#6b7280', fontSize: '13px' }}>Total Balance</div>
            <div style={{ fontSize: '28px', fontWeight: 700, color: '#111827' }}>
              ${totalUsd.toLocaleString('en-US', { minimumFractionDigits: 2 })}
            </div>
          </div>
        )}

        {/* Balances list */}
        <div style={{ marginBottom: '16px' }}>
          {balances.map(b => (
            <div key={b.asset} style={balanceCardStyle}>
              <div>
                <div style={{ fontWeight: 600 }}>{b.asset}</div>
                <div style={{ fontSize: '12px', color: '#9ca3af' }}>{b.balance}</div>
              </div>
              {b.usdValue !== undefined && (
                <div style={{ fontWeight: 500, color: '#374151' }}>
                  ${b.usdValue.toLocaleString('en-US', { minimumFractionDigits: 2 })}
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Actions */}
        <div style={{ display: 'flex', gap: '8px' }}>
          {allowSend && (
            <Button onClick={() => setStep('send')} primaryColor={theme.primaryColor}>Send</Button>
          )}
          {allowReceive && (
            <Button variant="secondary" onClick={() => setStep('receive')} primaryColor={theme.primaryColor}>Receive</Button>
          )}
        </div>

        <div style={{ marginTop: '12px', textAlign: 'center' }}>
          <button
            onClick={handleDisconnect}
            style={{ background: 'none', border: 'none', fontSize: '13px', color: '#ef4444', cursor: 'pointer' }}
          >
            Disconnect
          </button>
        </div>
      </div>
    );
  };

  const renderHistory = () => (
    <div>
      {/* Tabs */}
      <div style={{ display: 'flex', borderBottom: '1px solid #e5e7eb', marginBottom: '16px' }}>
        <button style={tabStyle(step === 'dashboard')} onClick={() => setStep('dashboard')}>Balances</button>
        <button style={tabStyle(step === 'history')} onClick={() => setStep('history')}>History</button>
      </div>

      {history.length === 0 ? (
        <div style={{ textAlign: 'center', padding: '24px 0', color: '#9ca3af', fontSize: '14px' }}>
          No transactions yet
        </div>
      ) : (
        history.map(tx => (
          <div key={tx.id} style={{ padding: '12px 0', borderBottom: '1px solid #f3f4f6', fontSize: '14px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
              <span style={{ fontWeight: 500, textTransform: 'capitalize' }}>{tx.type}</span>
              <span style={{ fontWeight: 600, color: tx.type === 'receive' ? theme.successColor : '#374151' }}>
                {tx.type === 'receive' ? '+' : '-'}{tx.amount} {tx.asset}
              </span>
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', color: '#9ca3af' }}>
              <span>{new Date(tx.timestamp).toLocaleDateString()}</span>
              <span style={{
                padding: '2px 6px',
                borderRadius: '4px',
                fontSize: '11px',
                backgroundColor: tx.status === 'confirmed' ? '#f0fdf4' : tx.status === 'pending' ? '#fefce8' : '#fee2e2',
                color: tx.status === 'confirmed' ? '#16a34a' : tx.status === 'pending' ? '#ca8a04' : '#dc2626',
              }}>
                {tx.status}
              </span>
            </div>
          </div>
        ))
      )}
    </div>
  );

  const renderSend = () => (
    <div>
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '12px', color: '#374151' }}>Send Tokens</div>
      <div style={{ marginBottom: '12px' }}>
        <label style={{ fontSize: '13px', fontWeight: 500, color: '#374151', display: 'block', marginBottom: '6px' }}>Asset</label>
        <select
          value={sendAsset}
          onChange={e => setSendAsset(e.target.value)}
          style={{
            width: '100%', padding: '8px 12px', border: '1px solid #d1d5db',
            borderRadius: '6px', fontSize: '14px', backgroundColor: '#fff',
          }}
        >
          {balances.map(b => <option key={b.asset} value={b.asset}>{b.asset} ({b.balance})</option>)}
        </select>
      </div>
      <Input label="Recipient Address" value={sendTo} onChange={e => setSendTo(e.target.value)} placeholder="0x..." />
      <Input label="Amount" type="number" value={sendAmount} onChange={e => setSendAmount(e.target.value)} placeholder="0.00" min="0" />
      {error && (
        <div style={{ color: theme.errorColor, fontSize: '13px', padding: '8px 12px', backgroundColor: '#fee2e2', borderRadius: '6px', marginBottom: '12px' }}>
          {error}
        </div>
      )}
      <div style={{ display: 'flex', gap: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('dashboard')} primaryColor={theme.primaryColor}>Cancel</Button>
        <Button onClick={handleSend} loading={sending} primaryColor={theme.primaryColor}>Send</Button>
      </div>
    </div>
  );

  const renderReceive = () => (
    <div style={{ textAlign: 'center' }}>
      <div style={{ fontSize: '14px', fontWeight: 500, marginBottom: '16px', color: '#374151' }}>Receive Tokens</div>
      <div style={{
        backgroundColor: '#f9fafb',
        borderRadius: '8px',
        padding: '20px',
        marginBottom: '16px',
        wordBreak: 'break-all',
      }}>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '8px' }}>Your Address ({network})</div>
        <div style={{ fontWeight: 500, fontSize: '14px', color: '#111827', fontFamily: 'monospace' }}>
          {walletAddress}
        </div>
      </div>
      <p style={{ color: '#6b7280', fontSize: '13px', marginBottom: '16px' }}>
        Send tokens to the address above on the {network} network.
      </p>
      <Button
        onClick={() => {
          if (typeof navigator !== 'undefined' && navigator.clipboard) {
            navigator.clipboard.writeText(walletAddress);
          }
        }}
        primaryColor={theme.primaryColor}
      >
        Copy Address
      </Button>
      <div style={{ marginTop: '8px' }}>
        <Button variant="secondary" onClick={() => setStep('dashboard')} primaryColor={theme.primaryColor}>Back</Button>
      </div>
    </div>
  );

  return (
    <div style={containerStyle} data-testid="rampos-wallet">
      <div style={headerStyle}>
        <span>RampOS Wallet</span>
        <button
          onClick={handleClose}
          style={{ background: 'none', border: 'none', fontSize: '20px', cursor: 'pointer', color: '#9ca3af' }}
          aria-label="Close"
        >
          x
        </button>
      </div>

      {step === 'connect' && renderConnect()}
      {step === 'dashboard' && renderDashboard()}
      {step === 'history' && renderHistory()}
      {step === 'send' && renderSend()}
      {step === 'receive' && renderReceive()}

      <div style={{ marginTop: '20px', textAlign: 'center', fontSize: '11px', color: '#9ca3af' }}>
        Powered by RampOS
      </div>
    </div>
  );
};

export default RampOSWallet;
