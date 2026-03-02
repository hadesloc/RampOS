import React from 'react';
import ReactDOM from 'react-dom/client';
import RampOSWallet from '../components/RampOSWallet';
import type { WidgetTheme, Network } from '../types/index';

export class RampOSWalletElement extends HTMLElement {
  private root: ReactDOM.Root | null = null;
  private mountPoint: HTMLDivElement;

  static get observedAttributes() {
    return [
      'api-key', 'user-id', 'default-network', 'environment',
      'show-balance', 'allow-send', 'allow-receive',
      'theme-primary', 'theme-bg', 'theme-text', 'theme-radius', 'theme-font',
    ];
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.mountPoint = document.createElement('div');
    this.shadowRoot!.appendChild(this.mountPoint);
  }

  connectedCallback() {
    this.renderComponent();
  }

  attributeChangedCallback() {
    this.renderComponent();
  }

  disconnectedCallback() {
    if (this.root) {
      this.root.unmount();
      this.root = null;
    }
  }

  private getTheme(): WidgetTheme {
    return {
      primaryColor: this.getAttribute('theme-primary') || undefined,
      backgroundColor: this.getAttribute('theme-bg') || undefined,
      textColor: this.getAttribute('theme-text') || undefined,
      borderRadius: this.getAttribute('theme-radius') || undefined,
      fontFamily: this.getAttribute('theme-font') || undefined,
    };
  }

  private renderComponent() {
    const apiKey = this.getAttribute('api-key');
    if (!apiKey) {
      console.error('[RampOS] api-key attribute is required for <rampos-wallet>');
      return;
    }

    const props = {
      apiKey,
      userId: this.getAttribute('user-id') || undefined,
      defaultNetwork: (this.getAttribute('default-network') || 'polygon') as Network,
      environment: (this.getAttribute('environment') || 'sandbox') as 'sandbox' | 'production',
      showBalance: this.getAttribute('show-balance') !== 'false',
      allowSend: this.getAttribute('allow-send') !== 'false',
      allowReceive: this.getAttribute('allow-receive') !== 'false',
      theme: this.getTheme(),
      onConnected: (wallet: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-connected', { detail: wallet, bubbles: true, composed: true }));
      },
      onDisconnected: () => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-disconnected', { bubbles: true, composed: true }));
      },
      onTransactionSent: (tx: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-tx-sent', { detail: tx, bubbles: true, composed: true }));
      },
      onTransactionConfirmed: (tx: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-tx-confirmed', { detail: tx, bubbles: true, composed: true }));
      },
      onError: (error: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-error', { detail: error, bubbles: true, composed: true }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-close', { bubbles: true, composed: true }));
      },
      onReady: () => {
        this.dispatchEvent(new CustomEvent('rampos-wallet-ready', { bubbles: true, composed: true }));
      },
    };

    if (!this.root) {
      this.root = ReactDOM.createRoot(this.mountPoint);
    }
    this.root.render(React.createElement(RampOSWallet, props));
  }
}

if (typeof customElements !== 'undefined' && !customElements.get('rampos-wallet')) {
  customElements.define('rampos-wallet', RampOSWalletElement);
}
