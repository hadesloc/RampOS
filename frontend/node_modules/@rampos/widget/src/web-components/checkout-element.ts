import React from 'react';
import ReactDOM from 'react-dom/client';
import RampOSCheckout from '../components/RampOSCheckout';
import type { WidgetTheme, CryptoAsset, Network } from '../types/index';

export class RampOSCheckoutElement extends HTMLElement {
  private root: ReactDOM.Root | null = null;
  private mountPoint: HTMLDivElement;

  static get observedAttributes() {
    return [
      'api-key', 'amount', 'asset', 'network', 'wallet-address',
      'fiat-currency', 'environment',
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
      console.error('[RampOS] api-key attribute is required for <rampos-checkout>');
      return;
    }

    const amountStr = this.getAttribute('amount');
    const props = {
      apiKey,
      amount: amountStr ? parseFloat(amountStr) : undefined,
      asset: (this.getAttribute('asset') || undefined) as CryptoAsset | undefined,
      network: (this.getAttribute('network') || undefined) as Network | undefined,
      walletAddress: this.getAttribute('wallet-address') || undefined,
      fiatCurrency: this.getAttribute('fiat-currency') || undefined,
      environment: (this.getAttribute('environment') || 'sandbox') as 'sandbox' | 'production',
      theme: this.getTheme(),
      onSuccess: (result: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-success', { detail: result, bubbles: true, composed: true }));
      },
      onError: (error: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-error', { detail: error, bubbles: true, composed: true }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent('rampos-close', { bubbles: true, composed: true }));
      },
      onReady: () => {
        this.dispatchEvent(new CustomEvent('rampos-ready', { bubbles: true, composed: true }));
      },
    };

    if (!this.root) {
      this.root = ReactDOM.createRoot(this.mountPoint);
    }
    this.root.render(React.createElement(RampOSCheckout, props));
  }
}

if (typeof customElements !== 'undefined' && !customElements.get('rampos-checkout')) {
  customElements.define('rampos-checkout', RampOSCheckoutElement);
}
