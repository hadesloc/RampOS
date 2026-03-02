var E = Object.defineProperty;
var g = (t, e, r) => e in t ? E(t, e, { enumerable: !0, configurable: !0, writable: !0, value: r }) : t[e] = r;
var p = (t, e, r) => g(t, typeof e != "symbol" ? e + "" : e, r);
const d = class d {
  constructor(e = "*") {
    p(this, "listeners", /* @__PURE__ */ new Map());
    p(this, "targetOrigin");
    this.targetOrigin = e;
  }
  static getInstance(e) {
    return d.instance || (d.instance = new d(e)), d.instance;
  }
  /** Reset singleton - used in tests */
  static resetInstance() {
    d.instance = void 0;
  }
  emit(e, r) {
    const s = { type: e, payload: r, timestamp: Date.now() }, c = this.listeners.get(e);
    c && c.forEach((i) => {
      try {
        i(r);
      } catch (n) {
        console.error(`[RampOS] Error in event handler for ${e}:`, n);
      }
    }), typeof window < "u" && window.parent && window.parent !== window && window.parent.postMessage({ source: "rampos-widget", event: s }, this.targetOrigin), typeof window < "u" && window.dispatchEvent(
      new CustomEvent(`rampos:${e.toLowerCase()}`, {
        detail: { ...s },
        bubbles: !0,
        composed: !0
      })
    );
  }
  on(e, r) {
    return this.listeners.has(e) || this.listeners.set(e, /* @__PURE__ */ new Set()), this.listeners.get(e).add(r), () => {
      var s;
      (s = this.listeners.get(e)) == null || s.delete(r);
    };
  }
  off(e, r) {
    var s;
    (s = this.listeners.get(e)) == null || s.delete(r);
  }
  removeAllListeners(e) {
    e ? this.listeners.delete(e) : this.listeners.clear();
  }
};
p(d, "instance");
let m = d;
const v = {
  sandbox: "https://sandbox-api.rampos.io/v1",
  production: "https://api.rampos.io/v1"
};
class f {
  constructor(e) {
    p(this, "apiKey");
    p(this, "baseUrl");
    this.apiKey = e.apiKey, this.baseUrl = e.baseUrl ?? v[e.environment ?? "sandbox"];
  }
  async request(e, r = {}) {
    const s = `${this.baseUrl}${e}`, c = {
      "Content-Type": "application/json",
      "X-API-Key": this.apiKey,
      ...r.headers || {}
    }, i = await fetch(s, {
      ...r,
      headers: c
    });
    if (!i.ok) {
      const n = await i.text();
      throw new Error(`RampOS API error (${i.status}): ${n}`);
    }
    return i.json();
  }
  // ----- Checkout -----
  async createCheckout(e) {
    return this.request("/checkout", {
      method: "POST",
      body: JSON.stringify(e)
    });
  }
  async confirmCheckout(e) {
    return this.request(`/checkout/${encodeURIComponent(e)}/confirm`, {
      method: "POST"
    });
  }
  async getCheckoutStatus(e) {
    return this.request(`/checkout/${encodeURIComponent(e)}`);
  }
  // ----- KYC -----
  async submitKYC(e) {
    return this.request("/kyc/submit", {
      method: "POST",
      body: JSON.stringify(e)
    });
  }
  async getKYCStatus(e) {
    return this.request(`/kyc/status/${encodeURIComponent(e)}`);
  }
  // ----- Wallet -----
  async getWallet(e, r) {
    const s = r ? `?network=${encodeURIComponent(r)}` : "";
    return this.request(`/wallet/${encodeURIComponent(e)}${s}`);
  }
  async getBalances(e, r) {
    return this.request(`/wallet/${encodeURIComponent(e)}/balances?network=${encodeURIComponent(r)}`);
  }
  async sendTransaction(e) {
    return this.request("/wallet/send", {
      method: "POST",
      body: JSON.stringify(e)
    });
  }
  async getTransactionHistory(e, r) {
    return this.request(
      `/wallet/${encodeURIComponent(e)}/transactions?network=${encodeURIComponent(r)}`
    );
  }
}
const b = {
  primaryColor: "#2563eb",
  backgroundColor: "#ffffff",
  textColor: "#1f2937",
  borderRadius: "8px",
  fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  errorColor: "#ef4444",
  successColor: "#10b981"
}, l = /* @__PURE__ */ new Map();
let R = 0;
function S() {
  return `rampos-widget-${++R}`;
}
function x(t) {
  if (typeof t == "string") {
    const e = document.querySelector(t);
    if (!e)
      throw new Error(`[RampOS] Container not found: ${t}`);
    return e;
  }
  if (t instanceof HTMLElement)
    return t;
  throw new Error("[RampOS] Invalid container: must be a CSS selector string or HTMLElement");
}
function h(t, e) {
  const r = { ...b, ...e };
  t.style.setProperty("--rampos-primary", r.primaryColor || ""), t.style.setProperty("--rampos-bg", r.backgroundColor || ""), t.style.setProperty("--rampos-text", r.textColor || ""), t.style.setProperty("--rampos-radius", r.borderRadius || ""), t.style.setProperty("--rampos-font", r.fontFamily || ""), t.style.setProperty("--rampos-error", r.errorColor || ""), t.style.setProperty("--rampos-success", r.successColor || "");
}
function T(t) {
  const e = t.type || "checkout", r = t.environment || "sandbox";
  return `<div class="rampos-widget-inner" data-type="${e}" data-env="${r}">
    <div class="rampos-widget-header">
      <span class="rampos-widget-title">RampOS ${e.charAt(0).toUpperCase() + e.slice(1)}</span>
      <button class="rampos-widget-close" aria-label="Close">&times;</button>
    </div>
    <div class="rampos-widget-body">
      <div class="rampos-widget-content"></div>
    </div>
    <div class="rampos-widget-footer">
      <span class="rampos-powered">Powered by RampOS</span>
    </div>
  </div>`;
}
function O(t) {
  const e = "rampos-embed-styles";
  if (t.querySelector(`#${e}`)) return;
  const r = document.createElement("style");
  r.id = e, r.textContent = `
    .rampos-widget-root {
      font-family: var(--rampos-font, 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
      color: var(--rampos-text, #1f2937);
      background: var(--rampos-bg, #ffffff);
      border-radius: var(--rampos-radius, 8px);
      border: 1px solid #e5e7eb;
      overflow: hidden;
      box-sizing: border-box;
      max-width: 480px;
      width: 100%;
    }
    .rampos-widget-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 12px 16px;
      border-bottom: 1px solid #e5e7eb;
      background: var(--rampos-primary, #2563eb);
      color: #ffffff;
    }
    .rampos-widget-title {
      font-weight: 600;
      font-size: 14px;
    }
    .rampos-widget-close {
      background: none;
      border: none;
      color: #ffffff;
      font-size: 20px;
      cursor: pointer;
      padding: 0 4px;
      line-height: 1;
    }
    .rampos-widget-body {
      padding: 16px;
      min-height: 120px;
    }
    .rampos-widget-content {
      font-size: 14px;
    }
    .rampos-widget-footer {
      padding: 8px 16px;
      border-top: 1px solid #e5e7eb;
      text-align: center;
    }
    .rampos-powered {
      font-size: 11px;
      color: #9ca3af;
    }
  `, t.prepend(r);
}
function k(t) {
  if (!t.apiKey)
    throw new Error("[RampOS] apiKey is required");
  const e = x(t.container), r = S(), s = document.createElement("div");
  s.className = "rampos-widget-root", s.id = r, s.setAttribute("data-rampos-widget", "true");
  const c = { ...b, ...t.theme };
  h(s, c), O(s), s.insertAdjacentHTML("beforeend", T(t)), e.appendChild(s);
  const i = new f({
    apiKey: t.apiKey,
    environment: t.environment
  }), n = m.getInstance(), y = s.querySelector(".rampos-widget-close");
  y && y.addEventListener("click", () => {
    var o;
    (o = t.onClose) == null || o.call(t), n.emit("CHECKOUT_CLOSE");
  });
  const u = [];
  if (t.onSuccess) {
    const o = t.type === "kyc" ? "KYC_APPROVED" : t.type === "wallet" ? "WALLET_CONNECTED" : "CHECKOUT_SUCCESS";
    u.push(
      n.on(o, (a) => {
        a !== void 0 && t.onSuccess(a);
      })
    );
  }
  if (t.onError) {
    const o = t.type === "kyc" ? "KYC_ERROR" : t.type === "wallet" ? "WALLET_ERROR" : "CHECKOUT_ERROR";
    u.push(n.on(o, (a) => {
      t.onError(a instanceof Error ? a : new Error(String(a || "Unknown error")));
    }));
  }
  if (t.onReady) {
    const o = t.type === "kyc" ? "KYC_READY" : t.type === "wallet" ? "WALLET_READY" : "CHECKOUT_READY";
    u.push(n.on(o, t.onReady));
  }
  const C = t.type === "kyc" ? "KYC_READY" : t.type === "wallet" ? "WALLET_READY" : "CHECKOUT_READY";
  setTimeout(() => n.emit(C), 0);
  const w = {
    id: r,
    container: s,
    destroy() {
      u.forEach((o) => o()), u.length = 0, s.remove(), l.delete(r);
    },
    update(o) {
      if (o.theme) {
        const a = { ...c, ...o.theme };
        h(s, a);
      }
    },
    getApiClient() {
      return i;
    },
    getEventEmitter() {
      return n;
    }
  };
  return l.set(r, w), w;
}
const U = {
  version: "1.0.0",
  init(t) {
    return k(t);
  },
  destroy(t) {
    if (!t) {
      l.forEach((s) => s.destroy());
      return;
    }
    const e = typeof t == "string" ? t : t.id, r = l.get(e);
    r && r.destroy();
  },
  destroyAll() {
    l.forEach((t) => t.destroy());
  },
  getInstances() {
    return Array.from(l.values());
  },
  EventEmitter: m,
  ApiClient: f
};
typeof window < "u" && (window.RampOSWidget = U);
export {
  U as RampOSWidget,
  U as default
};
