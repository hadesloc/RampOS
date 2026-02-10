import React from 'react';
import ReactDOM from 'react-dom/client';
import RampOSKYC from '../components/RampOSKYC';
import type { WidgetTheme, KYCLevel } from '../types/index';

export class RampOSKYCElement extends HTMLElement {
  private root: ReactDOM.Root | null = null;
  private mountPoint: HTMLDivElement;

  static get observedAttributes() {
    return [
      'api-key', 'user-id', 'level', 'environment',
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
      console.error('[RampOS] api-key attribute is required for <rampos-kyc>');
      return;
    }

    const props = {
      apiKey,
      userId: this.getAttribute('user-id') || undefined,
      level: (this.getAttribute('level') || 'basic') as KYCLevel,
      environment: (this.getAttribute('environment') || 'sandbox') as 'sandbox' | 'production',
      theme: this.getTheme(),
      onSubmitted: (result: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-kyc-submitted', { detail: result, bubbles: true, composed: true }));
      },
      onApproved: (result: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-kyc-approved', { detail: result, bubbles: true, composed: true }));
      },
      onRejected: (result: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-kyc-rejected', { detail: result, bubbles: true, composed: true }));
      },
      onError: (error: unknown) => {
        this.dispatchEvent(new CustomEvent('rampos-kyc-error', { detail: error, bubbles: true, composed: true }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent('rampos-kyc-close', { bubbles: true, composed: true }));
      },
      onReady: () => {
        this.dispatchEvent(new CustomEvent('rampos-kyc-ready', { bubbles: true, composed: true }));
      },
    };

    if (!this.root) {
      this.root = ReactDOM.createRoot(this.mountPoint);
    }
    this.root.render(React.createElement(RampOSKYC, props));
  }
}

if (typeof customElements !== 'undefined' && !customElements.get('rampos-kyc')) {
  customElements.define('rampos-kyc', RampOSKYCElement);
}
