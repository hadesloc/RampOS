var RampOSWidget=function(i){"use strict";var U=Object.defineProperty;var A=(i,n,u)=>n in i?U(i,n,{enumerable:!0,configurable:!0,writable:!0,value:u}):i[n]=u;var y=(i,n,u)=>A(i,typeof n!="symbol"?n+"":n,u);const d=class d{constructor(e="*"){y(this,"listeners",new Map);y(this,"targetOrigin");this.targetOrigin=e}static getInstance(e){return d.instance||(d.instance=new d(e)),d.instance}static resetInstance(){d.instance=void 0}emit(e,r){const s={type:e,payload:r,timestamp:Date.now()},l=this.listeners.get(e);l&&l.forEach(c=>{try{c(r)}catch(a){console.error(`[RampOS] Error in event handler for ${e}:`,a)}}),typeof window<"u"&&window.parent&&window.parent!==window&&window.parent.postMessage({source:"rampos-widget",event:s},this.targetOrigin),typeof window<"u"&&window.dispatchEvent(new CustomEvent(`rampos:${e.toLowerCase()}`,{detail:{...s},bubbles:!0,composed:!0}))}on(e,r){return this.listeners.has(e)||this.listeners.set(e,new Set),this.listeners.get(e).add(r),()=>{var s;(s=this.listeners.get(e))==null||s.delete(r)}}off(e,r){var s;(s=this.listeners.get(e))==null||s.delete(r)}removeAllListeners(e){e?this.listeners.delete(e):this.listeners.clear()}};y(d,"instance");let n=d;const u={sandbox:"https://sandbox-api.rampos.io/v1",production:"https://api.rampos.io/v1"};class h{constructor(e){y(this,"apiKey");y(this,"baseUrl");this.apiKey=e.apiKey,this.baseUrl=e.baseUrl??u[e.environment??"sandbox"]}async request(e,r={}){const s=`${this.baseUrl}${e}`,l={"Content-Type":"application/json","X-API-Key":this.apiKey,...r.headers||{}},c=await fetch(s,{...r,headers:l});if(!c.ok){const a=await c.text();throw new Error(`RampOS API error (${c.status}): ${a}`)}return c.json()}async createCheckout(e){return this.request("/checkout",{method:"POST",body:JSON.stringify(e)})}async confirmCheckout(e){return this.request(`/checkout/${encodeURIComponent(e)}/confirm`,{method:"POST"})}async getCheckoutStatus(e){return this.request(`/checkout/${encodeURIComponent(e)}`)}async submitKYC(e){return this.request("/kyc/submit",{method:"POST",body:JSON.stringify(e)})}async getKYCStatus(e){return this.request(`/kyc/status/${encodeURIComponent(e)}`)}async getWallet(e,r){const s=r?`?network=${encodeURIComponent(r)}`:"";return this.request(`/wallet/${encodeURIComponent(e)}${s}`)}async getBalances(e,r){return this.request(`/wallet/${encodeURIComponent(e)}/balances?network=${encodeURIComponent(r)}`)}async sendTransaction(e){return this.request("/wallet/send",{method:"POST",body:JSON.stringify(e)})}async getTransactionHistory(e,r){return this.request(`/wallet/${encodeURIComponent(e)}/transactions?network=${encodeURIComponent(r)}`)}}const b={primaryColor:"#2563eb",backgroundColor:"#ffffff",textColor:"#1f2937",borderRadius:"8px",fontFamily:"'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",errorColor:"#ef4444",successColor:"#10b981"},m=new Map;let v=0;function R(){return`rampos-widget-${++v}`}function S(t){if(typeof t=="string"){const e=document.querySelector(t);if(!e)throw new Error(`[RampOS] Container not found: ${t}`);return e}if(t instanceof HTMLElement)return t;throw new Error("[RampOS] Invalid container: must be a CSS selector string or HTMLElement")}function C(t,e){const r={...b,...e};t.style.setProperty("--rampos-primary",r.primaryColor||""),t.style.setProperty("--rampos-bg",r.backgroundColor||""),t.style.setProperty("--rampos-text",r.textColor||""),t.style.setProperty("--rampos-radius",r.borderRadius||""),t.style.setProperty("--rampos-font",r.fontFamily||""),t.style.setProperty("--rampos-error",r.errorColor||""),t.style.setProperty("--rampos-success",r.successColor||"")}function T(t){const e=t.type||"checkout",r=t.environment||"sandbox";return`<div class="rampos-widget-inner" data-type="${e}" data-env="${r}">
    <div class="rampos-widget-header">
      <span class="rampos-widget-title">RampOS ${e.charAt(0).toUpperCase()+e.slice(1)}</span>
      <button class="rampos-widget-close" aria-label="Close">&times;</button>
    </div>
    <div class="rampos-widget-body">
      <div class="rampos-widget-content"></div>
    </div>
    <div class="rampos-widget-footer">
      <span class="rampos-powered">Powered by RampOS</span>
    </div>
  </div>`}function x(t){const e="rampos-embed-styles";if(t.querySelector(`#${e}`))return;const r=document.createElement("style");r.id=e,r.textContent=`
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
  `,t.prepend(r)}function O(t){if(!t.apiKey)throw new Error("[RampOS] apiKey is required");const e=S(t.container),r=R(),s=document.createElement("div");s.className="rampos-widget-root",s.id=r,s.setAttribute("data-rampos-widget","true");const l={...b,...t.theme};C(s,l),x(s),s.insertAdjacentHTML("beforeend",T(t)),e.appendChild(s);const c=new h({apiKey:t.apiKey,environment:t.environment}),a=n.getInstance(),g=s.querySelector(".rampos-widget-close");g&&g.addEventListener("click",()=>{var o;(o=t.onClose)==null||o.call(t),a.emit("CHECKOUT_CLOSE")});const w=[];if(t.onSuccess){const o=t.type==="kyc"?"KYC_APPROVED":t.type==="wallet"?"WALLET_CONNECTED":"CHECKOUT_SUCCESS";w.push(a.on(o,p=>{p!==void 0&&t.onSuccess(p)}))}if(t.onError){const o=t.type==="kyc"?"KYC_ERROR":t.type==="wallet"?"WALLET_ERROR":"CHECKOUT_ERROR";w.push(a.on(o,p=>{t.onError(p instanceof Error?p:new Error(String(p||"Unknown error")))}))}if(t.onReady){const o=t.type==="kyc"?"KYC_READY":t.type==="wallet"?"WALLET_READY":"CHECKOUT_READY";w.push(a.on(o,t.onReady))}const k=t.type==="kyc"?"KYC_READY":t.type==="wallet"?"WALLET_READY":"CHECKOUT_READY";setTimeout(()=>a.emit(k),0);const E={id:r,container:s,destroy(){w.forEach(o=>o()),w.length=0,s.remove(),m.delete(r)},update(o){if(o.theme){const p={...l,...o.theme};C(s,p)}},getApiClient(){return c},getEventEmitter(){return a}};return m.set(r,E),E}const f={version:"1.0.0",init(t){return O(t)},destroy(t){if(!t){m.forEach(s=>s.destroy());return}const e=typeof t=="string"?t:t.id,r=m.get(e);r&&r.destroy()},destroyAll(){m.forEach(t=>t.destroy())},getInstances(){return Array.from(m.values())},EventEmitter:n,ApiClient:h};return typeof window<"u"&&(window.RampOSWidget=f),i.RampOSWidget=f,i.default=f,Object.defineProperties(i,{__esModule:{value:!0},[Symbol.toStringTag]:{value:"Module"}}),i}({});
