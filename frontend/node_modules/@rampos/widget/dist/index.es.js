var ue = Object.defineProperty;
var he = (n, o, a) => o in n ? ue(n, o, { enumerable: !0, configurable: !0, writable: !0, value: a }) : n[o] = a;
var X = (n, o, a) => he(n, typeof o != "symbol" ? o + "" : o, a);
import { jsxs as t, jsx as e } from "react/jsx-runtime";
import { useState as y, useEffect as se, useCallback as de, useRef as fe } from "react";
const $ = class $ {
  constructor(o = "*") {
    X(this, "listeners", /* @__PURE__ */ new Map());
    X(this, "targetOrigin");
    this.targetOrigin = o;
  }
  static getInstance(o) {
    return $.instance || ($.instance = new $(o)), $.instance;
  }
  /** Reset singleton - used in tests */
  static resetInstance() {
    $.instance = void 0;
  }
  emit(o, a) {
    const m = { type: o, payload: a, timestamp: Date.now() }, h = this.listeners.get(o);
    h && h.forEach((x) => {
      try {
        x(a);
      } catch (v) {
        console.error(`[RampOS] Error in event handler for ${o}:`, v);
      }
    }), typeof window < "u" && window.parent && window.parent !== window && window.parent.postMessage({ source: "rampos-widget", event: m }, this.targetOrigin), typeof window < "u" && window.dispatchEvent(
      new CustomEvent(`rampos:${o.toLowerCase()}`, {
        detail: { ...m },
        bubbles: !0,
        composed: !0
      })
    );
  }
  on(o, a) {
    return this.listeners.has(o) || this.listeners.set(o, /* @__PURE__ */ new Set()), this.listeners.get(o).add(a), () => {
      var m;
      (m = this.listeners.get(o)) == null || m.delete(a);
    };
  }
  off(o, a) {
    var m;
    (m = this.listeners.get(o)) == null || m.delete(a);
  }
  removeAllListeners(o) {
    o ? this.listeners.delete(o) : this.listeners.clear();
  }
};
X($, "instance");
let le = $;
function ke(n, o) {
  const a = (m) => {
    if (o != null && o.origin && m.origin !== o.origin) return;
    const h = m.data;
    (h == null ? void 0 : h.source) === "rampos-widget" && h.event && n(h.event);
  };
  return window.addEventListener("message", a), () => window.removeEventListener("message", a);
}
const j = {
  primaryColor: "#2563eb",
  backgroundColor: "#ffffff",
  textColor: "#1f2937",
  borderRadius: "8px",
  fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  errorColor: "#ef4444",
  successColor: "#10b981"
};
function ce(n) {
  return {
    primaryColor: (n == null ? void 0 : n.primaryColor) ?? j.primaryColor,
    backgroundColor: (n == null ? void 0 : n.backgroundColor) ?? j.backgroundColor,
    textColor: (n == null ? void 0 : n.textColor) ?? j.textColor,
    borderRadius: (n == null ? void 0 : n.borderRadius) ?? j.borderRadius,
    fontFamily: (n == null ? void 0 : n.fontFamily) ?? j.fontFamily,
    errorColor: (n == null ? void 0 : n.errorColor) ?? j.errorColor,
    successColor: (n == null ? void 0 : n.successColor) ?? j.successColor
  };
}
function we(n) {
  return {
    fontFamily: n.fontFamily,
    color: n.textColor,
    backgroundColor: n.backgroundColor,
    borderRadius: n.borderRadius
  };
}
function Be(n) {
  return {
    "--rampos-primary-color": n.primaryColor,
    "--rampos-background": n.backgroundColor,
    "--rampos-text": n.textColor,
    "--rampos-border-radius": n.borderRadius,
    "--rampos-font-family": n.fontFamily,
    "--rampos-error-color": n.errorColor,
    "--rampos-success-color": n.successColor
  };
}
const u = ({
  variant: n = "primary",
  fullWidth: o = !0,
  loading: a = !1,
  primaryColor: m = "#2563eb",
  children: h,
  disabled: x,
  style: v,
  ...b
}) => /* @__PURE__ */ t(
  "button",
  {
    style: { ...{
      border: "none",
      borderRadius: "6px",
      padding: "10px 16px",
      fontSize: "14px",
      fontWeight: 500,
      cursor: x || a ? "not-allowed" : "pointer",
      width: o ? "100%" : "auto",
      transition: "background-color 0.2s, opacity 0.2s",
      opacity: x || a ? 0.6 : 1,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      gap: "8px"
    }, ...{
      primary: {
        backgroundColor: m,
        color: "#ffffff"
      },
      secondary: {
        backgroundColor: "transparent",
        color: "#6b7280",
        border: "1px solid #d1d5db"
      },
      ghost: {
        backgroundColor: "transparent",
        color: m
      }
    }[n], ...v },
    disabled: x || a,
    ...b,
    children: [
      a && /* @__PURE__ */ e("span", { style: {
        display: "inline-block",
        width: "14px",
        height: "14px",
        border: "2px solid currentColor",
        borderTopColor: "transparent",
        borderRadius: "50%",
        animation: "rampos-spin 0.6s linear infinite"
      } }),
      h
    ]
  }
), M = ({
  label: n,
  error: o,
  helpText: a,
  style: m,
  id: h,
  ...x
}) => {
  const v = h || `rampos-input-${n == null ? void 0 : n.toLowerCase().replace(/\s+/g, "-")}`;
  return /* @__PURE__ */ t("div", { style: { marginBottom: "12px" }, children: [
    n && /* @__PURE__ */ e(
      "label",
      {
        htmlFor: v,
        style: {
          display: "block",
          fontSize: "14px",
          fontWeight: 500,
          marginBottom: "4px",
          color: "#374151"
        },
        children: n
      }
    ),
    /* @__PURE__ */ e(
      "input",
      {
        id: v,
        style: {
          width: "100%",
          padding: "8px 12px",
          border: `1px solid ${o ? "#ef4444" : "#d1d5db"}`,
          borderRadius: "6px",
          fontSize: "14px",
          outline: "none",
          boxSizing: "border-box",
          transition: "border-color 0.2s",
          ...m
        },
        ...x
      }
    ),
    o && /* @__PURE__ */ e("div", { style: { color: "#ef4444", fontSize: "12px", marginTop: "4px" }, children: o }),
    a && !o && /* @__PURE__ */ e("div", { style: { color: "#9ca3af", fontSize: "12px", marginTop: "4px" }, children: a })
  ] });
}, pe = [
  { value: "USDC", label: "USDC", network: "polygon" },
  { value: "USDT", label: "USDT", network: "polygon" },
  { value: "ETH", label: "Ethereum", network: "arbitrum" },
  { value: "MATIC", label: "MATIC", network: "polygon" },
  { value: "VND_TOKEN", label: "VND Token", network: "polygon" }
], me = [
  { value: "bank_transfer", label: "Bank Transfer" },
  { value: "card", label: "Credit / Debit Card" },
  { value: "mobile_money", label: "Mobile Money (MoMo, ZaloPay)" }
], Te = ({
  apiKey: n,
  amount: o,
  asset: a,
  network: m,
  walletAddress: h,
  theme: x,
  onSuccess: v,
  onError: b,
  onClose: S,
  onReady: z
}) => {
  const i = ce(x), g = le.getInstance(), [f, s] = y(() => a && o ? "payment-method" : a ? "enter-amount" : "select-asset"), [c, R] = y(a || ""), [k, w] = y(m || ""), [C, Z] = y(o || 0), [B, V] = y(h || ""), [D, H] = y(""), [A, W] = y(null), [I, q] = y(!1);
  se(() => {
    g.emit("CHECKOUT_READY"), z == null || z();
  }, []);
  const E = de(() => {
    g.emit("CHECKOUT_CLOSE"), S == null || S();
  }, [g, S]), O = (l) => {
    R(l);
    const r = pe.find((p) => p.value === l);
    r && w(r.network), s("enter-amount");
  }, U = () => {
    if (C <= 0) {
      W("Please enter a valid amount");
      return;
    }
    W(null), C > 1e3 && !I ? s("kyc-check") : s("payment-method");
  }, Q = () => {
    q(!0), s("payment-method");
  }, F = (l) => {
    H(l), s("summary");
  }, Y = async () => {
    s("processing"), W(null);
    try {
      await new Promise((r) => setTimeout(r, 2e3));
      const l = {
        transactionId: `tx_${Date.now().toString(36)}_${Math.random().toString(36).substring(2, 8)}`,
        status: "success",
        amount: C,
        asset: c,
        network: k,
        walletAddress: B,
        timestamp: Date.now()
      };
      s("success"), g.emit("CHECKOUT_SUCCESS", l), v == null || v(l);
    } catch (l) {
      const r = l instanceof Error ? l.message : "Transaction failed";
      W(r), s("failed"), g.emit("CHECKOUT_ERROR", { message: r }), b == null || b(l instanceof Error ? l : new Error(r));
    }
  }, K = {
    fontFamily: i.fontFamily,
    padding: "24px",
    borderRadius: i.borderRadius,
    backgroundColor: i.backgroundColor,
    color: i.textColor,
    boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)",
    maxWidth: "420px",
    width: "100%",
    position: "relative"
  }, L = {
    fontSize: "18px",
    fontWeight: 600,
    marginBottom: "20px",
    borderBottom: "1px solid #e5e7eb",
    paddingBottom: "12px",
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center"
  }, J = (l) => ({
    padding: "12px 16px",
    border: `2px solid ${l ? i.primaryColor : "#e5e7eb"}`,
    borderRadius: "8px",
    marginBottom: "8px",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    backgroundColor: l ? `${i.primaryColor}08` : "#fff",
    transition: "all 0.15s"
  }), ee = (l) => ({
    ...J(l)
  }), P = {
    display: "flex",
    justifyContent: "space-between",
    marginBottom: "8px",
    fontSize: "14px",
    color: "#4b5563"
  }, te = {
    color: i.errorColor,
    fontSize: "13px",
    padding: "8px 12px",
    backgroundColor: "#fee2e2",
    borderRadius: "6px",
    marginBottom: "12px"
  }, G = () => /* @__PURE__ */ t("div", { children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Select an asset to purchase" }),
    pe.map((l) => /* @__PURE__ */ t(
      "div",
      {
        style: J(c === l.value),
        onClick: () => O(l.value),
        role: "button",
        tabIndex: 0,
        onKeyDown: (r) => {
          r.key === "Enter" && O(l.value);
        },
        children: [
          /* @__PURE__ */ t("div", { children: [
            /* @__PURE__ */ e("div", { style: { fontWeight: 600 }, children: l.label }),
            /* @__PURE__ */ e("div", { style: { fontSize: "12px", color: "#9ca3af" }, children: l.network })
          ] }),
          c === l.value && /* @__PURE__ */ e("span", { style: { color: i.primaryColor, fontWeight: 700 }, children: "✓" })
        ]
      },
      l.value
    ))
  ] }), N = () => /* @__PURE__ */ t("div", { children: [
    /* @__PURE__ */ e(
      M,
      {
        label: `Amount (${c})`,
        type: "number",
        value: C || "",
        onChange: (l) => Z(parseFloat(l.target.value) || 0),
        placeholder: "0.00",
        min: "0",
        error: A || void 0
      }
    ),
    /* @__PURE__ */ e(
      M,
      {
        label: "Wallet Address",
        type: "text",
        value: B,
        onChange: (l) => V(l.target.value),
        placeholder: "0x...",
        helpText: "Your receiving wallet address"
      }
    ),
    /* @__PURE__ */ t("div", { style: { display: "flex", gap: "8px", marginTop: "8px" }, children: [
      /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("select-asset"), primaryColor: i.primaryColor, children: "Back" }),
      /* @__PURE__ */ e(u, { onClick: U, primaryColor: i.primaryColor, children: "Continue" })
    ] })
  ] }), _ = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "24px", marginBottom: "12px" }, children: "ID" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Identity Verification Required" }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "Transactions over $1,000 require KYC verification. This is a quick process." }),
    /* @__PURE__ */ e(u, { onClick: Q, primaryColor: i.primaryColor, children: "Complete Verification" }),
    /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "ghost", onClick: () => s("enter-amount"), primaryColor: i.primaryColor, children: "Go Back" }) })
  ] }), oe = () => /* @__PURE__ */ t("div", { children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Select payment method" }),
    me.map((l) => /* @__PURE__ */ t(
      "div",
      {
        style: ee(D === l.value),
        onClick: () => F(l.value),
        role: "button",
        tabIndex: 0,
        onKeyDown: (r) => {
          r.key === "Enter" && F(l.value);
        },
        children: [
          /* @__PURE__ */ e("span", { style: { fontWeight: 500 }, children: l.label }),
          D === l.value && /* @__PURE__ */ e("span", { style: { color: i.primaryColor, fontWeight: 700 }, children: "✓" })
        ]
      },
      l.value
    )),
    /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("enter-amount"), primaryColor: i.primaryColor, children: "Back" }) })
  ] }), re = () => {
    var l;
    return /* @__PURE__ */ t("div", { children: [
      /* @__PURE__ */ t("div", { style: { marginBottom: "16px" }, children: [
        /* @__PURE__ */ t("div", { style: P, children: [
          /* @__PURE__ */ e("span", { children: "Asset" }),
          /* @__PURE__ */ e("span", { style: { fontWeight: 600 }, children: c })
        ] }),
        /* @__PURE__ */ t("div", { style: P, children: [
          /* @__PURE__ */ e("span", { children: "Network" }),
          /* @__PURE__ */ e("span", { style: { fontWeight: 600 }, children: k })
        ] }),
        /* @__PURE__ */ t("div", { style: P, children: [
          /* @__PURE__ */ e("span", { children: "Amount" }),
          /* @__PURE__ */ t("span", { style: { fontWeight: 600 }, children: [
            C,
            " ",
            c
          ] })
        ] }),
        /* @__PURE__ */ t("div", { style: P, children: [
          /* @__PURE__ */ e("span", { children: "Payment" }),
          /* @__PURE__ */ e("span", { style: { fontWeight: 600 }, children: (l = me.find((r) => r.value === D)) == null ? void 0 : l.label })
        ] }),
        B && /* @__PURE__ */ t("div", { style: P, children: [
          /* @__PURE__ */ e("span", { children: "Wallet" }),
          /* @__PURE__ */ t("span", { style: { fontWeight: 600, fontSize: "12px", wordBreak: "break-all" }, children: [
            B.substring(0, 6),
            "...",
            B.substring(B.length - 4)
          ] })
        ] }),
        /* @__PURE__ */ t("div", { style: { ...P, borderTop: "1px solid #e5e7eb", paddingTop: "8px", marginTop: "8px", fontWeight: 600 }, children: [
          /* @__PURE__ */ e("span", { children: "Total" }),
          /* @__PURE__ */ t("span", { children: [
            C,
            " ",
            c
          ] })
        ] })
      ] }),
      /* @__PURE__ */ e(u, { onClick: Y, primaryColor: i.primaryColor, children: "Confirm Payment" }),
      /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("payment-method"), primaryColor: i.primaryColor, children: "Back" }) })
    ] });
  }, ne = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "24px 0" }, children: [
    /* @__PURE__ */ e("div", { style: {
      width: "44px",
      height: "44px",
      border: `3px solid ${i.primaryColor}`,
      borderTopColor: "transparent",
      borderRadius: "50%",
      margin: "0 auto 16px",
      animation: "rampos-spin 0.8s linear infinite"
    } }),
    /* @__PURE__ */ e("div", { style: { fontWeight: 500, color: "#374151" }, children: "Processing your transaction..." }),
    /* @__PURE__ */ e("div", { style: { fontSize: "13px", color: "#9ca3af", marginTop: "4px" }, children: "This may take a moment" }),
    /* @__PURE__ */ e("style", { children: "@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }" })
  ] }), ie = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { color: i.successColor, fontSize: "48px", marginBottom: "8px" }, children: "✓" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Payment Successful!" }),
    /* @__PURE__ */ t("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: [
      "Your ",
      C,
      " ",
      c,
      " purchase has been processed."
    ] }),
    /* @__PURE__ */ e(u, { onClick: E, primaryColor: i.primaryColor, children: "Done" })
  ] }), ae = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { color: i.errorColor, fontSize: "48px", marginBottom: "8px" }, children: "✗" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Payment Failed" }),
    A && /* @__PURE__ */ e("div", { style: te, children: A }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "Something went wrong. Please try again." }),
    /* @__PURE__ */ e(u, { onClick: () => s("summary"), primaryColor: i.primaryColor, children: "Try Again" }),
    /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "ghost", onClick: E, primaryColor: i.primaryColor, children: "Cancel" }) })
  ] });
  return /* @__PURE__ */ t("div", { style: K, "data-testid": "rampos-checkout", children: [
    /* @__PURE__ */ t("div", { style: L, children: [
      /* @__PURE__ */ e("span", { children: "RampOS Checkout" }),
      /* @__PURE__ */ e(
        "button",
        {
          onClick: E,
          style: { background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#9ca3af" },
          "aria-label": "Close",
          children: "x"
        }
      )
    ] }),
    f === "select-asset" && G(),
    f === "enter-amount" && N(),
    f === "kyc-check" && _(),
    f === "payment-method" && oe(),
    f === "summary" && re(),
    f === "processing" && ne(),
    f === "success" && ie(),
    f === "failed" && ae(),
    /* @__PURE__ */ e("div", { style: { marginTop: "20px", textAlign: "center", fontSize: "11px", color: "#9ca3af" }, children: "Powered by RampOS" })
  ] });
}, Re = ({
  apiKey: n,
  userId: o,
  level: a = "basic",
  theme: m,
  onSubmitted: h,
  onApproved: x,
  onRejected: v,
  onError: b,
  onClose: S,
  onReady: z
}) => {
  const i = ce(m), g = le.getInstance(), [f, s] = y("intro"), [c, R] = y(""), [k, w] = y(""), [C, Z] = y(""), [B, V] = y(""), [D, H] = y("national_id"), [A, W] = y(!1), [I, q] = y(!1), [E, O] = y(null);
  se(() => {
    g.emit("KYC_READY"), z == null || z();
  }, []);
  const U = de(() => {
    g.emit("KYC_CLOSE"), S == null || S();
  }, [g, S]), Q = () => {
    if (!c || !k || !C) {
      O("Please fill in all required fields");
      return;
    }
    O(null), s("document-upload");
  }, F = () => {
    W(!0), s(a === "basic" ? "review" : "selfie");
  }, Y = () => {
    q(!0), s("review");
  }, K = async () => {
    s("submitting"), O(null);
    try {
      await new Promise((T) => setTimeout(T, 2e3));
      const d = {
        userId: o || `user_${Date.now().toString(36)}`,
        status: "pending",
        level: a,
        verifiedAt: void 0
      };
      s("submitted"), g.emit("KYC_SUBMITTED", d), h == null || h(d), setTimeout(() => {
        const T = {
          ...d,
          status: "approved",
          verifiedAt: Date.now(),
          expiresAt: Date.now() + 31536e6
        };
        s("approved"), g.emit("KYC_APPROVED", T), x == null || x(T);
      }, 3e3);
    } catch (d) {
      const T = d instanceof Error ? d.message : "KYC submission failed";
      O(T), s("intro"), g.emit("KYC_ERROR", { message: T }), b == null || b(d instanceof Error ? d : new Error(T));
    }
  }, L = {
    fontFamily: i.fontFamily,
    padding: "24px",
    borderRadius: i.borderRadius,
    backgroundColor: i.backgroundColor,
    color: i.textColor,
    boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
    maxWidth: "420px",
    width: "100%"
  }, J = {
    fontSize: "18px",
    fontWeight: 600,
    marginBottom: "20px",
    borderBottom: "1px solid #e5e7eb",
    paddingBottom: "12px",
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center"
  }, ee = {
    color: i.errorColor,
    fontSize: "13px",
    padding: "8px 12px",
    backgroundColor: "#fee2e2",
    borderRadius: "6px",
    marginBottom: "12px"
  }, P = {
    display: "flex",
    gap: "4px",
    marginBottom: "20px"
  }, te = a === "basic" ? ["Info", "Document", "Review"] : ["Info", "Document", "Selfie", "Review"], G = (() => {
    switch (f) {
      case "intro":
        return -1;
      case "personal-info":
        return 0;
      case "document-upload":
        return 1;
      case "selfie":
        return 2;
      case "review":
        return a === "basic" ? 2 : 3;
      default:
        return -1;
    }
  })(), N = () => /* @__PURE__ */ e("div", { style: P, children: te.map((d, T) => /* @__PURE__ */ t("div", { style: { flex: 1, textAlign: "center" }, children: [
    /* @__PURE__ */ e("div", { style: {
      height: "4px",
      borderRadius: "2px",
      backgroundColor: T <= G ? i.primaryColor : "#e5e7eb",
      marginBottom: "4px",
      transition: "background-color 0.2s"
    } }),
    /* @__PURE__ */ e("span", { style: { fontSize: "11px", color: T <= G ? i.primaryColor : "#9ca3af" }, children: d })
  ] }, d)) }), _ = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "32px", marginBottom: "12px", color: i.primaryColor }, children: "ID" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Identity Verification" }),
    /* @__PURE__ */ t("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "8px" }, children: [
      "Level: ",
      /* @__PURE__ */ e("strong", { children: a.charAt(0).toUpperCase() + a.slice(1) })
    ] }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "13px", marginBottom: "24px", lineHeight: "1.5" }, children: "We need to verify your identity to comply with regulations. This usually takes a few minutes." }),
    /* @__PURE__ */ e(u, { onClick: () => s("personal-info"), primaryColor: i.primaryColor, children: "Start Verification" })
  ] }), oe = () => /* @__PURE__ */ t("div", { children: [
    N(),
    /* @__PURE__ */ e(M, { label: "First Name *", value: c, onChange: (d) => R(d.target.value), placeholder: "John" }),
    /* @__PURE__ */ e(M, { label: "Last Name *", value: k, onChange: (d) => w(d.target.value), placeholder: "Doe" }),
    /* @__PURE__ */ e(M, { label: "Date of Birth *", type: "date", value: C, onChange: (d) => Z(d.target.value) }),
    /* @__PURE__ */ e(M, { label: "Nationality", value: B, onChange: (d) => V(d.target.value), placeholder: "Vietnamese" }),
    E && /* @__PURE__ */ e("div", { style: ee, children: E }),
    /* @__PURE__ */ t("div", { style: { display: "flex", gap: "8px", marginTop: "8px" }, children: [
      /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("intro"), primaryColor: i.primaryColor, children: "Back" }),
      /* @__PURE__ */ e(u, { onClick: Q, primaryColor: i.primaryColor, children: "Next" })
    ] })
  ] }), re = () => /* @__PURE__ */ t("div", { children: [
    N(),
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Upload Identity Document" }),
    /* @__PURE__ */ t("div", { style: { marginBottom: "16px" }, children: [
      /* @__PURE__ */ e("label", { style: { fontSize: "13px", fontWeight: 500, color: "#374151", display: "block", marginBottom: "8px" }, children: "Document Type" }),
      /* @__PURE__ */ t(
        "select",
        {
          value: D,
          onChange: (d) => H(d.target.value),
          style: {
            width: "100%",
            padding: "8px 12px",
            border: "1px solid #d1d5db",
            borderRadius: "6px",
            fontSize: "14px",
            backgroundColor: "#fff"
          },
          children: [
            /* @__PURE__ */ e("option", { value: "national_id", children: "National ID Card" }),
            /* @__PURE__ */ e("option", { value: "passport", children: "Passport" }),
            /* @__PURE__ */ e("option", { value: "drivers_license", children: "Driver's License" })
          ]
        }
      )
    ] }),
    /* @__PURE__ */ t(
      "div",
      {
        onClick: F,
        style: {
          border: `2px dashed ${A ? i.successColor : "#d1d5db"}`,
          borderRadius: "8px",
          padding: "32px",
          textAlign: "center",
          cursor: "pointer",
          backgroundColor: A ? "#f0fdf4" : "#fafafa",
          transition: "all 0.2s",
          marginBottom: "16px"
        },
        role: "button",
        tabIndex: 0,
        onKeyDown: (d) => {
          d.key === "Enter" && F();
        },
        children: [
          /* @__PURE__ */ e("div", { style: { fontSize: "24px", marginBottom: "8px" }, children: A ? "&#10003;" : "+" }),
          /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, color: A ? i.successColor : "#6b7280" }, children: A ? "Document uploaded" : "Click to upload front of document" }),
          /* @__PURE__ */ e("div", { style: { fontSize: "12px", color: "#9ca3af", marginTop: "4px" }, children: "PNG, JPG up to 10MB" })
        ]
      }
    ),
    /* @__PURE__ */ e("div", { style: { display: "flex", gap: "8px" }, children: /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("personal-info"), primaryColor: i.primaryColor, children: "Back" }) })
  ] }), ne = () => /* @__PURE__ */ t("div", { children: [
    N(),
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Take a Selfie" }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "13px", marginBottom: "16px" }, children: "Please take a clear photo of your face. Make sure your face is well-lit and fully visible." }),
    /* @__PURE__ */ t(
      "div",
      {
        onClick: Y,
        style: {
          border: `2px dashed ${I ? i.successColor : "#d1d5db"}`,
          borderRadius: "8px",
          padding: "32px",
          textAlign: "center",
          cursor: "pointer",
          backgroundColor: I ? "#f0fdf4" : "#fafafa",
          marginBottom: "16px"
        },
        role: "button",
        tabIndex: 0,
        onKeyDown: (d) => {
          d.key === "Enter" && Y();
        },
        children: [
          /* @__PURE__ */ e("div", { style: { fontSize: "24px", marginBottom: "8px" }, children: I ? "&#10003;" : "+" }),
          /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, color: I ? i.successColor : "#6b7280" }, children: I ? "Selfie captured" : "Click to take selfie" })
        ]
      }
    ),
    /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("document-upload"), primaryColor: i.primaryColor, children: "Back" })
  ] }), ie = () => /* @__PURE__ */ t("div", { children: [
    N(),
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "16px", color: "#374151" }, children: "Review Your Information" }),
    /* @__PURE__ */ t("div", { style: { backgroundColor: "#f9fafb", borderRadius: "8px", padding: "16px", marginBottom: "16px", fontSize: "14px" }, children: [
      /* @__PURE__ */ t("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "8px" }, children: [
        /* @__PURE__ */ e("span", { style: { color: "#6b7280" }, children: "Name" }),
        /* @__PURE__ */ t("span", { style: { fontWeight: 500 }, children: [
          c,
          " ",
          k
        ] })
      ] }),
      /* @__PURE__ */ t("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "8px" }, children: [
        /* @__PURE__ */ e("span", { style: { color: "#6b7280" }, children: "Date of Birth" }),
        /* @__PURE__ */ e("span", { style: { fontWeight: 500 }, children: C })
      ] }),
      /* @__PURE__ */ t("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "8px" }, children: [
        /* @__PURE__ */ e("span", { style: { color: "#6b7280" }, children: "Document" }),
        /* @__PURE__ */ e("span", { style: { fontWeight: 500 }, children: D.replace("_", " ") })
      ] }),
      /* @__PURE__ */ t("div", { style: { display: "flex", justifyContent: "space-between" }, children: [
        /* @__PURE__ */ e("span", { style: { color: "#6b7280" }, children: "Level" }),
        /* @__PURE__ */ e("span", { style: { fontWeight: 500 }, children: a })
      ] })
    ] }),
    /* @__PURE__ */ e(u, { onClick: K, primaryColor: i.primaryColor, children: "Submit for Verification" }),
    /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => s("document-upload"), primaryColor: i.primaryColor, children: "Back" }) })
  ] }), ae = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "24px 0" }, children: [
    /* @__PURE__ */ e("div", { style: {
      width: "44px",
      height: "44px",
      border: `3px solid ${i.primaryColor}`,
      borderTopColor: "transparent",
      borderRadius: "50%",
      margin: "0 auto 16px",
      animation: "rampos-spin 0.8s linear infinite"
    } }),
    /* @__PURE__ */ e("div", { style: { fontWeight: 500, color: "#374151" }, children: "Submitting your documents..." }),
    /* @__PURE__ */ e("style", { children: "@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }" })
  ] }), l = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: {
      width: "44px",
      height: "44px",
      border: `3px solid ${i.primaryColor}`,
      borderTopColor: "transparent",
      borderRadius: "50%",
      margin: "0 auto 16px",
      animation: "rampos-spin 0.8s linear infinite"
    } }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Verification In Progress" }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "14px" }, children: "Your documents are being reviewed. This usually takes a few minutes." }),
    /* @__PURE__ */ e("style", { children: "@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }" })
  ] }), r = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { color: i.successColor, fontSize: "48px", marginBottom: "8px" }, children: "✓" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Verification Complete" }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "Your identity has been verified successfully." }),
    /* @__PURE__ */ e(u, { onClick: U, primaryColor: i.primaryColor, children: "Done" })
  ] }), p = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { color: i.errorColor, fontSize: "48px", marginBottom: "8px" }, children: "✗" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Verification Failed" }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "We were unable to verify your identity. Please try again with clearer documents." }),
    /* @__PURE__ */ e(u, { onClick: () => s("intro"), primaryColor: i.primaryColor, children: "Try Again" }),
    /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "ghost", onClick: U, primaryColor: i.primaryColor, children: "Close" }) })
  ] });
  return /* @__PURE__ */ t("div", { style: L, "data-testid": "rampos-kyc", children: [
    /* @__PURE__ */ t("div", { style: J, children: [
      /* @__PURE__ */ e("span", { children: "RampOS KYC" }),
      /* @__PURE__ */ e(
        "button",
        {
          onClick: U,
          style: { background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#9ca3af" },
          "aria-label": "Close",
          children: "x"
        }
      )
    ] }),
    f === "intro" && _(),
    f === "personal-info" && oe(),
    f === "document-upload" && re(),
    f === "selfie" && ne(),
    f === "review" && ie(),
    f === "submitting" && ae(),
    f === "submitted" && l(),
    f === "approved" && r(),
    f === "rejected" && p(),
    /* @__PURE__ */ e("div", { style: { marginTop: "20px", textAlign: "center", fontSize: "11px", color: "#9ca3af" }, children: "Powered by RampOS" })
  ] });
}, ye = [
  { asset: "USDC", balance: "1,250.00", decimals: 6, usdValue: 1250 },
  { asset: "ETH", balance: "0.5432", decimals: 18, usdValue: 1358 },
  { asset: "MATIC", balance: "500.00", decimals: 18, usdValue: 450 },
  { asset: "VND_TOKEN", balance: "25,000,000", decimals: 18, usdValue: 1e3 }
], xe = [
  { id: "tx1", type: "receive", asset: "USDC", amount: "500", from: "0xabc...def", to: "0x123...456", status: "confirmed", timestamp: Date.now() - 864e5, txHash: "0xfeed..." },
  { id: "tx2", type: "send", asset: "ETH", amount: "0.1", from: "0x123...456", to: "0xdef...abc", status: "confirmed", timestamp: Date.now() - 1728e5, txHash: "0xbeef..." },
  { id: "tx3", type: "receive", asset: "MATIC", amount: "200", from: "0xabc...789", to: "0x123...456", status: "pending", timestamp: Date.now() - 36e5 }
], ge = [
  { value: "polygon", label: "Polygon" },
  { value: "arbitrum", label: "Arbitrum" },
  { value: "optimism", label: "Optimism" },
  { value: "ethereum", label: "Ethereum" },
  { value: "base", label: "Base" }
], ze = ({
  apiKey: n,
  userId: o,
  defaultNetwork: a = "polygon",
  theme: m,
  showBalance: h = !0,
  allowSend: x = !0,
  allowReceive: v = !0,
  onConnected: b,
  onDisconnected: S,
  onTransactionSent: z,
  onTransactionConfirmed: i,
  onError: g,
  onClose: f,
  onReady: s
}) => {
  const c = ce(m), R = le.getInstance(), [k, w] = y("connect"), [C, Z] = y(a), [B, V] = y(""), [D, H] = y([]), [A, W] = y([]), [I, q] = y(""), [E, O] = y(""), [U, Q] = y("USDC"), [F, Y] = y(!1), [K, L] = y(null);
  se(() => {
    R.emit("WALLET_READY"), s == null || s();
  }, []);
  const J = de(() => {
    R.emit("WALLET_CLOSE"), f == null || f();
  }, [R, f]), ee = async () => {
    try {
      await new Promise((d) => setTimeout(d, 1e3));
      const r = "0x" + Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join("");
      V(r), H(ye), W(xe), w("dashboard");
      const p = {
        address: r,
        network: C,
        balances: ye
      };
      R.emit("WALLET_CONNECTED", p), b == null || b(p);
    } catch (r) {
      const p = r instanceof Error ? r.message : "Connection failed";
      L(p), R.emit("WALLET_ERROR", { message: p }), g == null || g(r instanceof Error ? r : new Error(p));
    }
  }, P = () => {
    V(""), H([]), W([]), w("connect"), R.emit("WALLET_DISCONNECTED"), S == null || S();
  }, te = async () => {
    if (!I || !E) {
      L("Please fill in all fields");
      return;
    }
    Y(!0), L(null);
    try {
      await new Promise((p) => setTimeout(p, 2e3));
      const r = {
        id: `tx_${Date.now().toString(36)}`,
        type: "send",
        asset: U,
        amount: E,
        from: B,
        to: I,
        status: "pending",
        timestamp: Date.now(),
        txHash: "0x" + Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join("")
      };
      W((p) => [r, ...p]), R.emit("WALLET_TX_SENT", r), z == null || z(r), setTimeout(() => {
        const p = { ...r, status: "confirmed" };
        W((d) => d.map((T) => T.id === r.id ? p : T)), R.emit("WALLET_TX_CONFIRMED", p), i == null || i(p);
      }, 3e3), q(""), O(""), w("dashboard");
    } catch (r) {
      const p = r instanceof Error ? r.message : "Transaction failed";
      L(p), R.emit("WALLET_ERROR", { message: p }), g == null || g(r instanceof Error ? r : new Error(p));
    } finally {
      Y(!1);
    }
  }, G = {
    fontFamily: c.fontFamily,
    padding: "24px",
    borderRadius: c.borderRadius,
    backgroundColor: c.backgroundColor,
    color: c.textColor,
    boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
    maxWidth: "420px",
    width: "100%"
  }, N = {
    fontSize: "18px",
    fontWeight: 600,
    marginBottom: "20px",
    borderBottom: "1px solid #e5e7eb",
    paddingBottom: "12px",
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center"
  }, _ = (r) => ({
    padding: "8px 16px",
    fontSize: "13px",
    fontWeight: r ? 600 : 400,
    color: r ? c.primaryColor : "#6b7280",
    borderBottom: r ? `2px solid ${c.primaryColor}` : "2px solid transparent",
    cursor: "pointer",
    background: "none",
    border: "none",
    transition: "all 0.15s"
  }), oe = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "12px 0",
    borderBottom: "1px solid #f3f4f6"
  }, re = () => /* @__PURE__ */ t("div", { style: { textAlign: "center", padding: "24px 0" }, children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "32px", marginBottom: "12px", color: c.primaryColor }, children: "W" }),
    /* @__PURE__ */ e("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Connect Wallet" }),
    /* @__PURE__ */ e("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "16px" }, children: "Connect your RampOS wallet to view balances and send transactions." }),
    /* @__PURE__ */ t("div", { style: { marginBottom: "16px" }, children: [
      /* @__PURE__ */ e("label", { style: { fontSize: "13px", fontWeight: 500, color: "#374151", display: "block", marginBottom: "6px" }, children: "Network" }),
      /* @__PURE__ */ e(
        "select",
        {
          value: C,
          onChange: (r) => Z(r.target.value),
          style: {
            width: "100%",
            padding: "8px 12px",
            border: "1px solid #d1d5db",
            borderRadius: "6px",
            fontSize: "14px",
            backgroundColor: "#fff"
          },
          children: ge.map((r) => /* @__PURE__ */ e("option", { value: r.value, children: r.label }, r.value))
        }
      )
    ] }),
    K && /* @__PURE__ */ e("div", { style: { color: c.errorColor, fontSize: "13px", padding: "8px 12px", backgroundColor: "#fee2e2", borderRadius: "6px", marginBottom: "12px" }, children: K }),
    /* @__PURE__ */ e(u, { onClick: ee, primaryColor: c.primaryColor, children: "Connect Wallet" })
  ] }), ne = () => {
    const r = D.reduce((p, d) => p + (d.usdValue || 0), 0);
    return /* @__PURE__ */ t("div", { children: [
      /* @__PURE__ */ t("div", { style: { backgroundColor: "#f9fafb", borderRadius: "8px", padding: "12px", marginBottom: "16px", fontSize: "13px" }, children: [
        /* @__PURE__ */ e("div", { style: { color: "#6b7280", marginBottom: "4px" }, children: "Wallet Address" }),
        /* @__PURE__ */ t("div", { style: { fontWeight: 500, wordBreak: "break-all" }, children: [
          B.substring(0, 10),
          "...",
          B.substring(B.length - 8)
        ] }),
        /* @__PURE__ */ t("div", { style: { color: "#9ca3af", fontSize: "12px", marginTop: "4px" }, children: [
          "Network: ",
          C
        ] })
      ] }),
      /* @__PURE__ */ t("div", { style: { display: "flex", borderBottom: "1px solid #e5e7eb", marginBottom: "16px" }, children: [
        /* @__PURE__ */ e("button", { style: _(k === "dashboard"), onClick: () => w("dashboard"), children: "Balances" }),
        /* @__PURE__ */ e("button", { style: _(k === "history"), onClick: () => w("history"), children: "History" })
      ] }),
      h && /* @__PURE__ */ t("div", { style: { textAlign: "center", marginBottom: "16px" }, children: [
        /* @__PURE__ */ e("div", { style: { color: "#6b7280", fontSize: "13px" }, children: "Total Balance" }),
        /* @__PURE__ */ t("div", { style: { fontSize: "28px", fontWeight: 700, color: "#111827" }, children: [
          "$",
          r.toLocaleString("en-US", { minimumFractionDigits: 2 })
        ] })
      ] }),
      /* @__PURE__ */ e("div", { style: { marginBottom: "16px" }, children: D.map((p) => /* @__PURE__ */ t("div", { style: oe, children: [
        /* @__PURE__ */ t("div", { children: [
          /* @__PURE__ */ e("div", { style: { fontWeight: 600 }, children: p.asset }),
          /* @__PURE__ */ e("div", { style: { fontSize: "12px", color: "#9ca3af" }, children: p.balance })
        ] }),
        p.usdValue !== void 0 && /* @__PURE__ */ t("div", { style: { fontWeight: 500, color: "#374151" }, children: [
          "$",
          p.usdValue.toLocaleString("en-US", { minimumFractionDigits: 2 })
        ] })
      ] }, p.asset)) }),
      /* @__PURE__ */ t("div", { style: { display: "flex", gap: "8px" }, children: [
        x && /* @__PURE__ */ e(u, { onClick: () => w("send"), primaryColor: c.primaryColor, children: "Send" }),
        v && /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => w("receive"), primaryColor: c.primaryColor, children: "Receive" })
      ] }),
      /* @__PURE__ */ e("div", { style: { marginTop: "12px", textAlign: "center" }, children: /* @__PURE__ */ e(
        "button",
        {
          onClick: P,
          style: { background: "none", border: "none", fontSize: "13px", color: "#ef4444", cursor: "pointer" },
          children: "Disconnect"
        }
      ) })
    ] });
  }, ie = () => /* @__PURE__ */ t("div", { children: [
    /* @__PURE__ */ t("div", { style: { display: "flex", borderBottom: "1px solid #e5e7eb", marginBottom: "16px" }, children: [
      /* @__PURE__ */ e("button", { style: _(k === "dashboard"), onClick: () => w("dashboard"), children: "Balances" }),
      /* @__PURE__ */ e("button", { style: _(k === "history"), onClick: () => w("history"), children: "History" })
    ] }),
    A.length === 0 ? /* @__PURE__ */ e("div", { style: { textAlign: "center", padding: "24px 0", color: "#9ca3af", fontSize: "14px" }, children: "No transactions yet" }) : A.map((r) => /* @__PURE__ */ t("div", { style: { padding: "12px 0", borderBottom: "1px solid #f3f4f6", fontSize: "14px" }, children: [
      /* @__PURE__ */ t("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "4px" }, children: [
        /* @__PURE__ */ e("span", { style: { fontWeight: 500, textTransform: "capitalize" }, children: r.type }),
        /* @__PURE__ */ t("span", { style: { fontWeight: 600, color: r.type === "receive" ? c.successColor : "#374151" }, children: [
          r.type === "receive" ? "+" : "-",
          r.amount,
          " ",
          r.asset
        ] })
      ] }),
      /* @__PURE__ */ t("div", { style: { display: "flex", justifyContent: "space-between", fontSize: "12px", color: "#9ca3af" }, children: [
        /* @__PURE__ */ e("span", { children: new Date(r.timestamp).toLocaleDateString() }),
        /* @__PURE__ */ e("span", { style: {
          padding: "2px 6px",
          borderRadius: "4px",
          fontSize: "11px",
          backgroundColor: r.status === "confirmed" ? "#f0fdf4" : r.status === "pending" ? "#fefce8" : "#fee2e2",
          color: r.status === "confirmed" ? "#16a34a" : r.status === "pending" ? "#ca8a04" : "#dc2626"
        }, children: r.status })
      ] })
    ] }, r.id))
  ] }), ae = () => /* @__PURE__ */ t("div", { children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Send Tokens" }),
    /* @__PURE__ */ t("div", { style: { marginBottom: "12px" }, children: [
      /* @__PURE__ */ e("label", { style: { fontSize: "13px", fontWeight: 500, color: "#374151", display: "block", marginBottom: "6px" }, children: "Asset" }),
      /* @__PURE__ */ e(
        "select",
        {
          value: U,
          onChange: (r) => Q(r.target.value),
          style: {
            width: "100%",
            padding: "8px 12px",
            border: "1px solid #d1d5db",
            borderRadius: "6px",
            fontSize: "14px",
            backgroundColor: "#fff"
          },
          children: D.map((r) => /* @__PURE__ */ t("option", { value: r.asset, children: [
            r.asset,
            " (",
            r.balance,
            ")"
          ] }, r.asset))
        }
      )
    ] }),
    /* @__PURE__ */ e(M, { label: "Recipient Address", value: I, onChange: (r) => q(r.target.value), placeholder: "0x..." }),
    /* @__PURE__ */ e(M, { label: "Amount", type: "number", value: E, onChange: (r) => O(r.target.value), placeholder: "0.00", min: "0" }),
    K && /* @__PURE__ */ e("div", { style: { color: c.errorColor, fontSize: "13px", padding: "8px 12px", backgroundColor: "#fee2e2", borderRadius: "6px", marginBottom: "12px" }, children: K }),
    /* @__PURE__ */ t("div", { style: { display: "flex", gap: "8px" }, children: [
      /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => w("dashboard"), primaryColor: c.primaryColor, children: "Cancel" }),
      /* @__PURE__ */ e(u, { onClick: te, loading: F, primaryColor: c.primaryColor, children: "Send" })
    ] })
  ] }), l = () => /* @__PURE__ */ t("div", { style: { textAlign: "center" }, children: [
    /* @__PURE__ */ e("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "16px", color: "#374151" }, children: "Receive Tokens" }),
    /* @__PURE__ */ t("div", { style: {
      backgroundColor: "#f9fafb",
      borderRadius: "8px",
      padding: "20px",
      marginBottom: "16px",
      wordBreak: "break-all"
    }, children: [
      /* @__PURE__ */ t("div", { style: { fontSize: "12px", color: "#6b7280", marginBottom: "8px" }, children: [
        "Your Address (",
        C,
        ")"
      ] }),
      /* @__PURE__ */ e("div", { style: { fontWeight: 500, fontSize: "14px", color: "#111827", fontFamily: "monospace" }, children: B })
    ] }),
    /* @__PURE__ */ t("p", { style: { color: "#6b7280", fontSize: "13px", marginBottom: "16px" }, children: [
      "Send tokens to the address above on the ",
      C,
      " network."
    ] }),
    /* @__PURE__ */ e(
      u,
      {
        onClick: () => {
          typeof navigator < "u" && navigator.clipboard && navigator.clipboard.writeText(B);
        },
        primaryColor: c.primaryColor,
        children: "Copy Address"
      }
    ),
    /* @__PURE__ */ e("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ e(u, { variant: "secondary", onClick: () => w("dashboard"), primaryColor: c.primaryColor, children: "Back" }) })
  ] });
  return /* @__PURE__ */ t("div", { style: G, "data-testid": "rampos-wallet", children: [
    /* @__PURE__ */ t("div", { style: N, children: [
      /* @__PURE__ */ e("span", { children: "RampOS Wallet" }),
      /* @__PURE__ */ e(
        "button",
        {
          onClick: J,
          style: { background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#9ca3af" },
          "aria-label": "Close",
          children: "x"
        }
      )
    ] }),
    k === "connect" && re(),
    k === "dashboard" && ne(),
    k === "history" && ie(),
    k === "send" && ae(),
    k === "receive" && l(),
    /* @__PURE__ */ e("div", { style: { marginTop: "20px", textAlign: "center", fontSize: "11px", color: "#9ca3af" }, children: "Powered by RampOS" })
  ] });
}, Ae = ({ open: n, onClose: o, title: a, children: m, width: h = "420px" }) => {
  const x = fe(null);
  return se(() => {
    const b = (S) => {
      S.key === "Escape" && o();
    };
    return n && document.addEventListener("keydown", b), () => document.removeEventListener("keydown", b);
  }, [n, o]), n ? /* @__PURE__ */ t(
    "div",
    {
      ref: x,
      onClick: (b) => {
        b.target === x.current && o();
      },
      style: {
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: "rgba(0, 0, 0, 0.5)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 99999,
        animation: "rampos-fade-in 0.2s ease-out"
      },
      children: [
        /* @__PURE__ */ t(
          "div",
          {
            style: {
              backgroundColor: "#ffffff",
              borderRadius: "12px",
              boxShadow: "0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)",
              maxWidth: h,
              width: "100%",
              maxHeight: "90vh",
              overflow: "auto",
              animation: "rampos-slide-up 0.2s ease-out"
            },
            children: [
              a && /* @__PURE__ */ t("div", { style: {
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "16px 20px",
                borderBottom: "1px solid #e5e7eb"
              }, children: [
                /* @__PURE__ */ e("span", { style: { fontSize: "16px", fontWeight: 600, color: "#111827" }, children: a }),
                /* @__PURE__ */ e(
                  "button",
                  {
                    onClick: o,
                    style: {
                      background: "none",
                      border: "none",
                      fontSize: "20px",
                      cursor: "pointer",
                      color: "#9ca3af",
                      padding: "4px",
                      lineHeight: 1
                    },
                    "aria-label": "Close",
                    children: "x"
                  }
                )
              ] }),
              /* @__PURE__ */ e("div", { style: { padding: "20px" }, children: m })
            ]
          }
        ),
        /* @__PURE__ */ e("style", { children: `
        @keyframes rampos-fade-in {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes rampos-slide-up {
          from { transform: translateY(10px); opacity: 0; }
          to { transform: translateY(0); opacity: 1; }
        }
        @keyframes rampos-spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      ` })
      ]
    }
  ) : null;
}, be = {
  sandbox: "https://sandbox-api.rampos.io/v1",
  production: "https://api.rampos.io/v1"
};
class We {
  constructor(o) {
    X(this, "apiKey");
    X(this, "baseUrl");
    this.apiKey = o.apiKey, this.baseUrl = o.baseUrl ?? be[o.environment ?? "sandbox"];
  }
  async request(o, a = {}) {
    const m = `${this.baseUrl}${o}`, h = {
      "Content-Type": "application/json",
      "X-API-Key": this.apiKey,
      ...a.headers || {}
    }, x = await fetch(m, {
      ...a,
      headers: h
    });
    if (!x.ok) {
      const v = await x.text();
      throw new Error(`RampOS API error (${x.status}): ${v}`);
    }
    return x.json();
  }
  // ----- Checkout -----
  async createCheckout(o) {
    return this.request("/checkout", {
      method: "POST",
      body: JSON.stringify(o)
    });
  }
  async confirmCheckout(o) {
    return this.request(`/checkout/${encodeURIComponent(o)}/confirm`, {
      method: "POST"
    });
  }
  async getCheckoutStatus(o) {
    return this.request(`/checkout/${encodeURIComponent(o)}`);
  }
  // ----- KYC -----
  async submitKYC(o) {
    return this.request("/kyc/submit", {
      method: "POST",
      body: JSON.stringify(o)
    });
  }
  async getKYCStatus(o) {
    return this.request(`/kyc/status/${encodeURIComponent(o)}`);
  }
  // ----- Wallet -----
  async getWallet(o, a) {
    const m = a ? `?network=${encodeURIComponent(a)}` : "";
    return this.request(`/wallet/${encodeURIComponent(o)}${m}`);
  }
  async getBalances(o, a) {
    return this.request(`/wallet/${encodeURIComponent(o)}/balances?network=${encodeURIComponent(a)}`);
  }
  async sendTransaction(o) {
    return this.request("/wallet/send", {
      method: "POST",
      body: JSON.stringify(o)
    });
  }
  async getTransactionHistory(o, a) {
    return this.request(
      `/wallet/${encodeURIComponent(o)}/transactions?network=${encodeURIComponent(a)}`
    );
  }
}
export {
  u as Button,
  Te as Checkout,
  j as DEFAULT_THEME,
  M as Input,
  Ae as Modal,
  We as RampOSApiClient,
  Te as RampOSCheckout,
  le as RampOSEventEmitter,
  Re as RampOSKYC,
  ze as RampOSWallet,
  ke as onRampOSMessage,
  ce as resolveTheme,
  we as themeToCSS,
  Be as themeToCSSVars
};
