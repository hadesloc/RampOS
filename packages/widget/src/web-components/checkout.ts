import React from 'react';
import ReactDOM from 'react-dom/client';
import Checkout from '../components/Checkout';
import { WidgetTheme } from '../types';

export class RampOSCheckoutElement extends HTMLElement {
  private root: ReactDOM.Root | null = null;
  private mountPoint: HTMLDivElement;

  static get observedAttributes() {
    return ['api-key', 'amount', 'asset', 'theme', 'theme-primary', 'theme-bg', 'theme-text'];
  }

  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this.mountPoint = document.createElement('div');
    this.shadowRoot?.appendChild(this.mountPoint);
  }

  connectedCallback() {
    this.render();
  }

  attributeChangedCallback() {
    this.render();
  }

  disconnectedCallback() {
    if (this.root) {
      this.root.unmount();
      this.root = null;
    }
  }

  private getTheme(): WidgetTheme {
    let themeConfig: WidgetTheme = {};
    const themeAttr = this.getAttribute('theme');
    if (themeAttr) {
      try {
        themeConfig = JSON.parse(themeAttr);
      } catch (e) {
        console.warn('RampOS Widget: Invalid JSON in theme attribute');
      }
    }

    return {
      ...themeConfig,
      primaryColor: this.getAttribute('theme-primary') || themeConfig.primaryColor,
      backgroundColor: this.getAttribute('theme-bg') || themeConfig.backgroundColor,
      textColor: this.getAttribute('theme-text') || themeConfig.textColor,
    };
  }

  private render() {
    const apiKey = this.getAttribute('api-key');
    const amount = parseFloat(this.getAttribute('amount') || '0');
    const asset = this.getAttribute('asset') || undefined;

    if (!apiKey) {
      console.error('RampOS Widget: api-key attribute is required');
      return;
    }

    const props = {
      apiKey,
      amount,
      asset,
      theme: this.getTheme(),
      onSuccess: (result: any) => {
        this.dispatchEvent(new CustomEvent('rampos-success', { detail: result, bubbles: true, composed: true }));
      },
      onError: (error: any) => {
        this.dispatchEvent(new CustomEvent('rampos-error', { detail: error, bubbles: true, composed: true }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent('rampos-close', { bubbles: true, composed: true }));
      }
    };

    if (!this.root) {
      this.root = ReactDOM.createRoot(this.mountPoint);
    }

    this.root.render(React.createElement(Checkout, props));
  }
}

// Auto-register
if (!customElements.get('rampos-checkout')) {
  customElements.define('rampos-checkout', RampOSCheckoutElement);
}
