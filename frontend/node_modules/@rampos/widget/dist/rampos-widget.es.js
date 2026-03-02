var Cc = Object.defineProperty;
var Ec = (e, t, n) => t in e ? Cc(e, t, { enumerable: !0, configurable: !0, writable: !0, value: n }) : e[t] = n;
var Oe = (e, t, n) => Ec(e, typeof t != "symbol" ? t + "" : t, n);
function jc(e) {
  return e && e.__esModule && Object.prototype.hasOwnProperty.call(e, "default") ? e.default : e;
}
var du = { exports: {} }, D = {};
/**
 * @license React
 * react.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
var vr = Symbol.for("react.element"), _c = Symbol.for("react.portal"), zc = Symbol.for("react.fragment"), Tc = Symbol.for("react.strict_mode"), Pc = Symbol.for("react.profiler"), Nc = Symbol.for("react.provider"), Rc = Symbol.for("react.context"), Lc = Symbol.for("react.forward_ref"), Oc = Symbol.for("react.suspense"), Dc = Symbol.for("react.memo"), Ic = Symbol.for("react.lazy"), qi = Symbol.iterator;
function Ac(e) {
  return e === null || typeof e != "object" ? null : (e = qi && e[qi] || e["@@iterator"], typeof e == "function" ? e : null);
}
var fu = { isMounted: function() {
  return !1;
}, enqueueForceUpdate: function() {
}, enqueueReplaceState: function() {
}, enqueueSetState: function() {
} }, pu = Object.assign, mu = {};
function jn(e, t, n) {
  this.props = e, this.context = t, this.refs = mu, this.updater = n || fu;
}
jn.prototype.isReactComponent = {};
jn.prototype.setState = function(e, t) {
  if (typeof e != "object" && typeof e != "function" && e != null) throw Error("setState(...): takes an object of state variables to update or a function which returns an object of state variables.");
  this.updater.enqueueSetState(this, e, t, "setState");
};
jn.prototype.forceUpdate = function(e) {
  this.updater.enqueueForceUpdate(this, e, "forceUpdate");
};
function hu() {
}
hu.prototype = jn.prototype;
function ni(e, t, n) {
  this.props = e, this.context = t, this.refs = mu, this.updater = n || fu;
}
var ri = ni.prototype = new hu();
ri.constructor = ni;
pu(ri, jn.prototype);
ri.isPureReactComponent = !0;
var bi = Array.isArray, yu = Object.prototype.hasOwnProperty, li = { current: null }, vu = { key: !0, ref: !0, __self: !0, __source: !0 };
function gu(e, t, n) {
  var r, l = {}, o = null, i = null;
  if (t != null) for (r in t.ref !== void 0 && (i = t.ref), t.key !== void 0 && (o = "" + t.key), t) yu.call(t, r) && !vu.hasOwnProperty(r) && (l[r] = t[r]);
  var s = arguments.length - 2;
  if (s === 1) l.children = n;
  else if (1 < s) {
    for (var u = Array(s), f = 0; f < s; f++) u[f] = arguments[f + 2];
    l.children = u;
  }
  if (e && e.defaultProps) for (r in s = e.defaultProps, s) l[r] === void 0 && (l[r] = s[r]);
  return { $$typeof: vr, type: e, key: o, ref: i, props: l, _owner: li.current };
}
function Mc(e, t) {
  return { $$typeof: vr, type: e.type, key: t, ref: e.ref, props: e.props, _owner: e._owner };
}
function oi(e) {
  return typeof e == "object" && e !== null && e.$$typeof === vr;
}
function Bc(e) {
  var t = { "=": "=0", ":": "=2" };
  return "$" + e.replace(/[=:]/g, function(n) {
    return t[n];
  });
}
var es = /\/+/g;
function Ml(e, t) {
  return typeof e == "object" && e !== null && e.key != null ? Bc("" + e.key) : t.toString(36);
}
function Fr(e, t, n, r, l) {
  var o = typeof e;
  (o === "undefined" || o === "boolean") && (e = null);
  var i = !1;
  if (e === null) i = !0;
  else switch (o) {
    case "string":
    case "number":
      i = !0;
      break;
    case "object":
      switch (e.$$typeof) {
        case vr:
        case _c:
          i = !0;
      }
  }
  if (i) return i = e, l = l(i), e = r === "" ? "." + Ml(i, 0) : r, bi(l) ? (n = "", e != null && (n = e.replace(es, "$&/") + "/"), Fr(l, t, n, "", function(f) {
    return f;
  })) : l != null && (oi(l) && (l = Mc(l, n + (!l.key || i && i.key === l.key ? "" : ("" + l.key).replace(es, "$&/") + "/") + e)), t.push(l)), 1;
  if (i = 0, r = r === "" ? "." : r + ":", bi(e)) for (var s = 0; s < e.length; s++) {
    o = e[s];
    var u = r + Ml(o, s);
    i += Fr(o, t, n, u, l);
  }
  else if (u = Ac(e), typeof u == "function") for (e = u.call(e), s = 0; !(o = e.next()).done; ) o = o.value, u = r + Ml(o, s++), i += Fr(o, t, n, u, l);
  else if (o === "object") throw t = String(e), Error("Objects are not valid as a React child (found: " + (t === "[object Object]" ? "object with keys {" + Object.keys(e).join(", ") + "}" : t) + "). If you meant to render a collection of children, use an array instead.");
  return i;
}
function kr(e, t, n) {
  if (e == null) return e;
  var r = [], l = 0;
  return Fr(e, r, "", "", function(o) {
    return t.call(n, o, l++);
  }), r;
}
function Fc(e) {
  if (e._status === -1) {
    var t = e._result;
    t = t(), t.then(function(n) {
      (e._status === 0 || e._status === -1) && (e._status = 1, e._result = n);
    }, function(n) {
      (e._status === 0 || e._status === -1) && (e._status = 2, e._result = n);
    }), e._status === -1 && (e._status = 0, e._result = t);
  }
  if (e._status === 1) return e._result.default;
  throw e._result;
}
var ve = { current: null }, Ur = { transition: null }, Uc = { ReactCurrentDispatcher: ve, ReactCurrentBatchConfig: Ur, ReactCurrentOwner: li };
function xu() {
  throw Error("act(...) is not supported in production builds of React.");
}
D.Children = { map: kr, forEach: function(e, t, n) {
  kr(e, function() {
    t.apply(this, arguments);
  }, n);
}, count: function(e) {
  var t = 0;
  return kr(e, function() {
    t++;
  }), t;
}, toArray: function(e) {
  return kr(e, function(t) {
    return t;
  }) || [];
}, only: function(e) {
  if (!oi(e)) throw Error("React.Children.only expected to receive a single React element child.");
  return e;
} };
D.Component = jn;
D.Fragment = zc;
D.Profiler = Pc;
D.PureComponent = ni;
D.StrictMode = Tc;
D.Suspense = Oc;
D.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED = Uc;
D.act = xu;
D.cloneElement = function(e, t, n) {
  if (e == null) throw Error("React.cloneElement(...): The argument must be a React element, but you passed " + e + ".");
  var r = pu({}, e.props), l = e.key, o = e.ref, i = e._owner;
  if (t != null) {
    if (t.ref !== void 0 && (o = t.ref, i = li.current), t.key !== void 0 && (l = "" + t.key), e.type && e.type.defaultProps) var s = e.type.defaultProps;
    for (u in t) yu.call(t, u) && !vu.hasOwnProperty(u) && (r[u] = t[u] === void 0 && s !== void 0 ? s[u] : t[u]);
  }
  var u = arguments.length - 2;
  if (u === 1) r.children = n;
  else if (1 < u) {
    s = Array(u);
    for (var f = 0; f < u; f++) s[f] = arguments[f + 2];
    r.children = s;
  }
  return { $$typeof: vr, type: e.type, key: l, ref: o, props: r, _owner: i };
};
D.createContext = function(e) {
  return e = { $$typeof: Rc, _currentValue: e, _currentValue2: e, _threadCount: 0, Provider: null, Consumer: null, _defaultValue: null, _globalName: null }, e.Provider = { $$typeof: Nc, _context: e }, e.Consumer = e;
};
D.createElement = gu;
D.createFactory = function(e) {
  var t = gu.bind(null, e);
  return t.type = e, t;
};
D.createRef = function() {
  return { current: null };
};
D.forwardRef = function(e) {
  return { $$typeof: Lc, render: e };
};
D.isValidElement = oi;
D.lazy = function(e) {
  return { $$typeof: Ic, _payload: { _status: -1, _result: e }, _init: Fc };
};
D.memo = function(e, t) {
  return { $$typeof: Dc, type: e, compare: t === void 0 ? null : t };
};
D.startTransition = function(e) {
  var t = Ur.transition;
  Ur.transition = {};
  try {
    e();
  } finally {
    Ur.transition = t;
  }
};
D.unstable_act = xu;
D.useCallback = function(e, t) {
  return ve.current.useCallback(e, t);
};
D.useContext = function(e) {
  return ve.current.useContext(e);
};
D.useDebugValue = function() {
};
D.useDeferredValue = function(e) {
  return ve.current.useDeferredValue(e);
};
D.useEffect = function(e, t) {
  return ve.current.useEffect(e, t);
};
D.useId = function() {
  return ve.current.useId();
};
D.useImperativeHandle = function(e, t, n) {
  return ve.current.useImperativeHandle(e, t, n);
};
D.useInsertionEffect = function(e, t) {
  return ve.current.useInsertionEffect(e, t);
};
D.useLayoutEffect = function(e, t) {
  return ve.current.useLayoutEffect(e, t);
};
D.useMemo = function(e, t) {
  return ve.current.useMemo(e, t);
};
D.useReducer = function(e, t, n) {
  return ve.current.useReducer(e, t, n);
};
D.useRef = function(e) {
  return ve.current.useRef(e);
};
D.useState = function(e) {
  return ve.current.useState(e);
};
D.useSyncExternalStore = function(e, t, n) {
  return ve.current.useSyncExternalStore(e, t, n);
};
D.useTransition = function() {
  return ve.current.useTransition();
};
D.version = "18.3.1";
du.exports = D;
var I = du.exports;
const ii = /* @__PURE__ */ jc(I);
var bn = {}, wu = { exports: {} }, Ne = {}, Su = { exports: {} }, ku = {};
/**
 * @license React
 * scheduler.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
(function(e) {
  function t(k, P) {
    var R = k.length;
    k.push(P);
    e: for (; 0 < R; ) {
      var $ = R - 1 >>> 1, U = k[$];
      if (0 < l(U, P)) k[$] = P, k[R] = U, R = $;
      else break e;
    }
  }
  function n(k) {
    return k.length === 0 ? null : k[0];
  }
  function r(k) {
    if (k.length === 0) return null;
    var P = k[0], R = k.pop();
    if (R !== P) {
      k[0] = R;
      e: for (var $ = 0, U = k.length, Ze = U >>> 1; $ < Ze; ) {
        var xe = 2 * ($ + 1) - 1, Le = k[xe], ce = xe + 1, Je = k[ce];
        if (0 > l(Le, R)) ce < U && 0 > l(Je, Le) ? (k[$] = Je, k[ce] = R, $ = ce) : (k[$] = Le, k[xe] = R, $ = xe);
        else if (ce < U && 0 > l(Je, R)) k[$] = Je, k[ce] = R, $ = ce;
        else break e;
      }
    }
    return P;
  }
  function l(k, P) {
    var R = k.sortIndex - P.sortIndex;
    return R !== 0 ? R : k.id - P.id;
  }
  if (typeof performance == "object" && typeof performance.now == "function") {
    var o = performance;
    e.unstable_now = function() {
      return o.now();
    };
  } else {
    var i = Date, s = i.now();
    e.unstable_now = function() {
      return i.now() - s;
    };
  }
  var u = [], f = [], m = 1, y = null, h = 3, g = !1, x = !1, S = !1, O = typeof setTimeout == "function" ? setTimeout : null, d = typeof clearTimeout == "function" ? clearTimeout : null, c = typeof setImmediate < "u" ? setImmediate : null;
  typeof navigator < "u" && navigator.scheduling !== void 0 && navigator.scheduling.isInputPending !== void 0 && navigator.scheduling.isInputPending.bind(navigator.scheduling);
  function p(k) {
    for (var P = n(f); P !== null; ) {
      if (P.callback === null) r(f);
      else if (P.startTime <= k) r(f), P.sortIndex = P.expirationTime, t(u, P);
      else break;
      P = n(f);
    }
  }
  function v(k) {
    if (S = !1, p(k), !x) if (n(u) !== null) x = !0, We(C);
    else {
      var P = n(f);
      P !== null && $e(v, P.startTime - k);
    }
  }
  function C(k, P) {
    x = !1, S && (S = !1, d(_), _ = -1), g = !0;
    var R = h;
    try {
      for (p(P), y = n(u); y !== null && (!(y.expirationTime > P) || k && !ne()); ) {
        var $ = y.callback;
        if (typeof $ == "function") {
          y.callback = null, h = y.priorityLevel;
          var U = $(y.expirationTime <= P);
          P = e.unstable_now(), typeof U == "function" ? y.callback = U : y === n(u) && r(u), p(P);
        } else r(u);
        y = n(u);
      }
      if (y !== null) var Ze = !0;
      else {
        var xe = n(f);
        xe !== null && $e(v, xe.startTime - P), Ze = !1;
      }
      return Ze;
    } finally {
      y = null, h = R, g = !1;
    }
  }
  var j = !1, T = null, _ = -1, A = 5, N = -1;
  function ne() {
    return !(e.unstable_now() - N < A);
  }
  function ue() {
    if (T !== null) {
      var k = e.unstable_now();
      N = k;
      var P = !0;
      try {
        P = T(!0, k);
      } finally {
        P ? ae() : (j = !1, T = null);
      }
    } else j = !1;
  }
  var ae;
  if (typeof c == "function") ae = function() {
    c(ue);
  };
  else if (typeof MessageChannel < "u") {
    var Ue = new MessageChannel(), It = Ue.port2;
    Ue.port1.onmessage = ue, ae = function() {
      It.postMessage(null);
    };
  } else ae = function() {
    O(ue, 0);
  };
  function We(k) {
    T = k, j || (j = !0, ae());
  }
  function $e(k, P) {
    _ = O(function() {
      k(e.unstable_now());
    }, P);
  }
  e.unstable_IdlePriority = 5, e.unstable_ImmediatePriority = 1, e.unstable_LowPriority = 4, e.unstable_NormalPriority = 3, e.unstable_Profiling = null, e.unstable_UserBlockingPriority = 2, e.unstable_cancelCallback = function(k) {
    k.callback = null;
  }, e.unstable_continueExecution = function() {
    x || g || (x = !0, We(C));
  }, e.unstable_forceFrameRate = function(k) {
    0 > k || 125 < k ? console.error("forceFrameRate takes a positive int between 0 and 125, forcing frame rates higher than 125 fps is not supported") : A = 0 < k ? Math.floor(1e3 / k) : 5;
  }, e.unstable_getCurrentPriorityLevel = function() {
    return h;
  }, e.unstable_getFirstCallbackNode = function() {
    return n(u);
  }, e.unstable_next = function(k) {
    switch (h) {
      case 1:
      case 2:
      case 3:
        var P = 3;
        break;
      default:
        P = h;
    }
    var R = h;
    h = P;
    try {
      return k();
    } finally {
      h = R;
    }
  }, e.unstable_pauseExecution = function() {
  }, e.unstable_requestPaint = function() {
  }, e.unstable_runWithPriority = function(k, P) {
    switch (k) {
      case 1:
      case 2:
      case 3:
      case 4:
      case 5:
        break;
      default:
        k = 3;
    }
    var R = h;
    h = k;
    try {
      return P();
    } finally {
      h = R;
    }
  }, e.unstable_scheduleCallback = function(k, P, R) {
    var $ = e.unstable_now();
    switch (typeof R == "object" && R !== null ? (R = R.delay, R = typeof R == "number" && 0 < R ? $ + R : $) : R = $, k) {
      case 1:
        var U = -1;
        break;
      case 2:
        U = 250;
        break;
      case 5:
        U = 1073741823;
        break;
      case 4:
        U = 1e4;
        break;
      default:
        U = 5e3;
    }
    return U = R + U, k = { id: m++, callback: P, priorityLevel: k, startTime: R, expirationTime: U, sortIndex: -1 }, R > $ ? (k.sortIndex = R, t(f, k), n(u) === null && k === n(f) && (S ? (d(_), _ = -1) : S = !0, $e(v, R - $))) : (k.sortIndex = U, t(u, k), x || g || (x = !0, We(C))), k;
  }, e.unstable_shouldYield = ne, e.unstable_wrapCallback = function(k) {
    var P = h;
    return function() {
      var R = h;
      h = P;
      try {
        return k.apply(this, arguments);
      } finally {
        h = R;
      }
    };
  };
})(ku);
Su.exports = ku;
var Wc = Su.exports;
/**
 * @license React
 * react-dom.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
var $c = I, Pe = Wc;
function w(e) {
  for (var t = "https://reactjs.org/docs/error-decoder.html?invariant=" + e, n = 1; n < arguments.length; n++) t += "&args[]=" + encodeURIComponent(arguments[n]);
  return "Minified React error #" + e + "; visit " + t + " for the full message or use the non-minified dev environment for full errors and additional helpful warnings.";
}
var Cu = /* @__PURE__ */ new Set(), er = {};
function Gt(e, t) {
  vn(e, t), vn(e + "Capture", t);
}
function vn(e, t) {
  for (er[e] = t, e = 0; e < t.length; e++) Cu.add(t[e]);
}
var ut = !(typeof window > "u" || typeof window.document > "u" || typeof window.document.createElement > "u"), ao = Object.prototype.hasOwnProperty, Vc = /^[:A-Z_a-z\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u02FF\u0370-\u037D\u037F-\u1FFF\u200C-\u200D\u2070-\u218F\u2C00-\u2FEF\u3001-\uD7FF\uF900-\uFDCF\uFDF0-\uFFFD][:A-Z_a-z\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u02FF\u0370-\u037D\u037F-\u1FFF\u200C-\u200D\u2070-\u218F\u2C00-\u2FEF\u3001-\uD7FF\uF900-\uFDCF\uFDF0-\uFFFD\-.0-9\u00B7\u0300-\u036F\u203F-\u2040]*$/, ts = {}, ns = {};
function Hc(e) {
  return ao.call(ns, e) ? !0 : ao.call(ts, e) ? !1 : Vc.test(e) ? ns[e] = !0 : (ts[e] = !0, !1);
}
function Kc(e, t, n, r) {
  if (n !== null && n.type === 0) return !1;
  switch (typeof t) {
    case "function":
    case "symbol":
      return !0;
    case "boolean":
      return r ? !1 : n !== null ? !n.acceptsBooleans : (e = e.toLowerCase().slice(0, 5), e !== "data-" && e !== "aria-");
    default:
      return !1;
  }
}
function Qc(e, t, n, r) {
  if (t === null || typeof t > "u" || Kc(e, t, n, r)) return !0;
  if (r) return !1;
  if (n !== null) switch (n.type) {
    case 3:
      return !t;
    case 4:
      return t === !1;
    case 5:
      return isNaN(t);
    case 6:
      return isNaN(t) || 1 > t;
  }
  return !1;
}
function ge(e, t, n, r, l, o, i) {
  this.acceptsBooleans = t === 2 || t === 3 || t === 4, this.attributeName = r, this.attributeNamespace = l, this.mustUseProperty = n, this.propertyName = e, this.type = t, this.sanitizeURL = o, this.removeEmptyString = i;
}
var se = {};
"children dangerouslySetInnerHTML defaultValue defaultChecked innerHTML suppressContentEditableWarning suppressHydrationWarning style".split(" ").forEach(function(e) {
  se[e] = new ge(e, 0, !1, e, null, !1, !1);
});
[["acceptCharset", "accept-charset"], ["className", "class"], ["htmlFor", "for"], ["httpEquiv", "http-equiv"]].forEach(function(e) {
  var t = e[0];
  se[t] = new ge(t, 1, !1, e[1], null, !1, !1);
});
["contentEditable", "draggable", "spellCheck", "value"].forEach(function(e) {
  se[e] = new ge(e, 2, !1, e.toLowerCase(), null, !1, !1);
});
["autoReverse", "externalResourcesRequired", "focusable", "preserveAlpha"].forEach(function(e) {
  se[e] = new ge(e, 2, !1, e, null, !1, !1);
});
"allowFullScreen async autoFocus autoPlay controls default defer disabled disablePictureInPicture disableRemotePlayback formNoValidate hidden loop noModule noValidate open playsInline readOnly required reversed scoped seamless itemScope".split(" ").forEach(function(e) {
  se[e] = new ge(e, 3, !1, e.toLowerCase(), null, !1, !1);
});
["checked", "multiple", "muted", "selected"].forEach(function(e) {
  se[e] = new ge(e, 3, !0, e, null, !1, !1);
});
["capture", "download"].forEach(function(e) {
  se[e] = new ge(e, 4, !1, e, null, !1, !1);
});
["cols", "rows", "size", "span"].forEach(function(e) {
  se[e] = new ge(e, 6, !1, e, null, !1, !1);
});
["rowSpan", "start"].forEach(function(e) {
  se[e] = new ge(e, 5, !1, e.toLowerCase(), null, !1, !1);
});
var si = /[\-:]([a-z])/g;
function ui(e) {
  return e[1].toUpperCase();
}
"accent-height alignment-baseline arabic-form baseline-shift cap-height clip-path clip-rule color-interpolation color-interpolation-filters color-profile color-rendering dominant-baseline enable-background fill-opacity fill-rule flood-color flood-opacity font-family font-size font-size-adjust font-stretch font-style font-variant font-weight glyph-name glyph-orientation-horizontal glyph-orientation-vertical horiz-adv-x horiz-origin-x image-rendering letter-spacing lighting-color marker-end marker-mid marker-start overline-position overline-thickness paint-order panose-1 pointer-events rendering-intent shape-rendering stop-color stop-opacity strikethrough-position strikethrough-thickness stroke-dasharray stroke-dashoffset stroke-linecap stroke-linejoin stroke-miterlimit stroke-opacity stroke-width text-anchor text-decoration text-rendering underline-position underline-thickness unicode-bidi unicode-range units-per-em v-alphabetic v-hanging v-ideographic v-mathematical vector-effect vert-adv-y vert-origin-x vert-origin-y word-spacing writing-mode xmlns:xlink x-height".split(" ").forEach(function(e) {
  var t = e.replace(
    si,
    ui
  );
  se[t] = new ge(t, 1, !1, e, null, !1, !1);
});
"xlink:actuate xlink:arcrole xlink:role xlink:show xlink:title xlink:type".split(" ").forEach(function(e) {
  var t = e.replace(si, ui);
  se[t] = new ge(t, 1, !1, e, "http://www.w3.org/1999/xlink", !1, !1);
});
["xml:base", "xml:lang", "xml:space"].forEach(function(e) {
  var t = e.replace(si, ui);
  se[t] = new ge(t, 1, !1, e, "http://www.w3.org/XML/1998/namespace", !1, !1);
});
["tabIndex", "crossOrigin"].forEach(function(e) {
  se[e] = new ge(e, 1, !1, e.toLowerCase(), null, !1, !1);
});
se.xlinkHref = new ge("xlinkHref", 1, !1, "xlink:href", "http://www.w3.org/1999/xlink", !0, !1);
["src", "href", "action", "formAction"].forEach(function(e) {
  se[e] = new ge(e, 1, !1, e.toLowerCase(), null, !0, !0);
});
function ai(e, t, n, r) {
  var l = se.hasOwnProperty(t) ? se[t] : null;
  (l !== null ? l.type !== 0 : r || !(2 < t.length) || t[0] !== "o" && t[0] !== "O" || t[1] !== "n" && t[1] !== "N") && (Qc(t, n, l, r) && (n = null), r || l === null ? Hc(t) && (n === null ? e.removeAttribute(t) : e.setAttribute(t, "" + n)) : l.mustUseProperty ? e[l.propertyName] = n === null ? l.type === 3 ? !1 : "" : n : (t = l.attributeName, r = l.attributeNamespace, n === null ? e.removeAttribute(t) : (l = l.type, n = l === 3 || l === 4 && n === !0 ? "" : "" + n, r ? e.setAttributeNS(r, t, n) : e.setAttribute(t, n))));
}
var ft = $c.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED, Cr = Symbol.for("react.element"), qt = Symbol.for("react.portal"), bt = Symbol.for("react.fragment"), ci = Symbol.for("react.strict_mode"), co = Symbol.for("react.profiler"), Eu = Symbol.for("react.provider"), ju = Symbol.for("react.context"), di = Symbol.for("react.forward_ref"), fo = Symbol.for("react.suspense"), po = Symbol.for("react.suspense_list"), fi = Symbol.for("react.memo"), mt = Symbol.for("react.lazy"), _u = Symbol.for("react.offscreen"), rs = Symbol.iterator;
function Ln(e) {
  return e === null || typeof e != "object" ? null : (e = rs && e[rs] || e["@@iterator"], typeof e == "function" ? e : null);
}
var Z = Object.assign, Bl;
function Un(e) {
  if (Bl === void 0) try {
    throw Error();
  } catch (n) {
    var t = n.stack.trim().match(/\n( *(at )?)/);
    Bl = t && t[1] || "";
  }
  return `
` + Bl + e;
}
var Fl = !1;
function Ul(e, t) {
  if (!e || Fl) return "";
  Fl = !0;
  var n = Error.prepareStackTrace;
  Error.prepareStackTrace = void 0;
  try {
    if (t) if (t = function() {
      throw Error();
    }, Object.defineProperty(t.prototype, "props", { set: function() {
      throw Error();
    } }), typeof Reflect == "object" && Reflect.construct) {
      try {
        Reflect.construct(t, []);
      } catch (f) {
        var r = f;
      }
      Reflect.construct(e, [], t);
    } else {
      try {
        t.call();
      } catch (f) {
        r = f;
      }
      e.call(t.prototype);
    }
    else {
      try {
        throw Error();
      } catch (f) {
        r = f;
      }
      e();
    }
  } catch (f) {
    if (f && r && typeof f.stack == "string") {
      for (var l = f.stack.split(`
`), o = r.stack.split(`
`), i = l.length - 1, s = o.length - 1; 1 <= i && 0 <= s && l[i] !== o[s]; ) s--;
      for (; 1 <= i && 0 <= s; i--, s--) if (l[i] !== o[s]) {
        if (i !== 1 || s !== 1)
          do
            if (i--, s--, 0 > s || l[i] !== o[s]) {
              var u = `
` + l[i].replace(" at new ", " at ");
              return e.displayName && u.includes("<anonymous>") && (u = u.replace("<anonymous>", e.displayName)), u;
            }
          while (1 <= i && 0 <= s);
        break;
      }
    }
  } finally {
    Fl = !1, Error.prepareStackTrace = n;
  }
  return (e = e ? e.displayName || e.name : "") ? Un(e) : "";
}
function Yc(e) {
  switch (e.tag) {
    case 5:
      return Un(e.type);
    case 16:
      return Un("Lazy");
    case 13:
      return Un("Suspense");
    case 19:
      return Un("SuspenseList");
    case 0:
    case 2:
    case 15:
      return e = Ul(e.type, !1), e;
    case 11:
      return e = Ul(e.type.render, !1), e;
    case 1:
      return e = Ul(e.type, !0), e;
    default:
      return "";
  }
}
function mo(e) {
  if (e == null) return null;
  if (typeof e == "function") return e.displayName || e.name || null;
  if (typeof e == "string") return e;
  switch (e) {
    case bt:
      return "Fragment";
    case qt:
      return "Portal";
    case co:
      return "Profiler";
    case ci:
      return "StrictMode";
    case fo:
      return "Suspense";
    case po:
      return "SuspenseList";
  }
  if (typeof e == "object") switch (e.$$typeof) {
    case ju:
      return (e.displayName || "Context") + ".Consumer";
    case Eu:
      return (e._context.displayName || "Context") + ".Provider";
    case di:
      var t = e.render;
      return e = e.displayName, e || (e = t.displayName || t.name || "", e = e !== "" ? "ForwardRef(" + e + ")" : "ForwardRef"), e;
    case fi:
      return t = e.displayName || null, t !== null ? t : mo(e.type) || "Memo";
    case mt:
      t = e._payload, e = e._init;
      try {
        return mo(e(t));
      } catch {
      }
  }
  return null;
}
function Xc(e) {
  var t = e.type;
  switch (e.tag) {
    case 24:
      return "Cache";
    case 9:
      return (t.displayName || "Context") + ".Consumer";
    case 10:
      return (t._context.displayName || "Context") + ".Provider";
    case 18:
      return "DehydratedFragment";
    case 11:
      return e = t.render, e = e.displayName || e.name || "", t.displayName || (e !== "" ? "ForwardRef(" + e + ")" : "ForwardRef");
    case 7:
      return "Fragment";
    case 5:
      return t;
    case 4:
      return "Portal";
    case 3:
      return "Root";
    case 6:
      return "Text";
    case 16:
      return mo(t);
    case 8:
      return t === ci ? "StrictMode" : "Mode";
    case 22:
      return "Offscreen";
    case 12:
      return "Profiler";
    case 21:
      return "Scope";
    case 13:
      return "Suspense";
    case 19:
      return "SuspenseList";
    case 25:
      return "TracingMarker";
    case 1:
    case 0:
    case 17:
    case 2:
    case 14:
    case 15:
      if (typeof t == "function") return t.displayName || t.name || null;
      if (typeof t == "string") return t;
  }
  return null;
}
function Nt(e) {
  switch (typeof e) {
    case "boolean":
    case "number":
    case "string":
    case "undefined":
      return e;
    case "object":
      return e;
    default:
      return "";
  }
}
function zu(e) {
  var t = e.type;
  return (e = e.nodeName) && e.toLowerCase() === "input" && (t === "checkbox" || t === "radio");
}
function Gc(e) {
  var t = zu(e) ? "checked" : "value", n = Object.getOwnPropertyDescriptor(e.constructor.prototype, t), r = "" + e[t];
  if (!e.hasOwnProperty(t) && typeof n < "u" && typeof n.get == "function" && typeof n.set == "function") {
    var l = n.get, o = n.set;
    return Object.defineProperty(e, t, { configurable: !0, get: function() {
      return l.call(this);
    }, set: function(i) {
      r = "" + i, o.call(this, i);
    } }), Object.defineProperty(e, t, { enumerable: n.enumerable }), { getValue: function() {
      return r;
    }, setValue: function(i) {
      r = "" + i;
    }, stopTracking: function() {
      e._valueTracker = null, delete e[t];
    } };
  }
}
function Er(e) {
  e._valueTracker || (e._valueTracker = Gc(e));
}
function Tu(e) {
  if (!e) return !1;
  var t = e._valueTracker;
  if (!t) return !0;
  var n = t.getValue(), r = "";
  return e && (r = zu(e) ? e.checked ? "true" : "false" : e.value), e = r, e !== n ? (t.setValue(e), !0) : !1;
}
function Jr(e) {
  if (e = e || (typeof document < "u" ? document : void 0), typeof e > "u") return null;
  try {
    return e.activeElement || e.body;
  } catch {
    return e.body;
  }
}
function ho(e, t) {
  var n = t.checked;
  return Z({}, t, { defaultChecked: void 0, defaultValue: void 0, value: void 0, checked: n ?? e._wrapperState.initialChecked });
}
function ls(e, t) {
  var n = t.defaultValue == null ? "" : t.defaultValue, r = t.checked != null ? t.checked : t.defaultChecked;
  n = Nt(t.value != null ? t.value : n), e._wrapperState = { initialChecked: r, initialValue: n, controlled: t.type === "checkbox" || t.type === "radio" ? t.checked != null : t.value != null };
}
function Pu(e, t) {
  t = t.checked, t != null && ai(e, "checked", t, !1);
}
function yo(e, t) {
  Pu(e, t);
  var n = Nt(t.value), r = t.type;
  if (n != null) r === "number" ? (n === 0 && e.value === "" || e.value != n) && (e.value = "" + n) : e.value !== "" + n && (e.value = "" + n);
  else if (r === "submit" || r === "reset") {
    e.removeAttribute("value");
    return;
  }
  t.hasOwnProperty("value") ? vo(e, t.type, n) : t.hasOwnProperty("defaultValue") && vo(e, t.type, Nt(t.defaultValue)), t.checked == null && t.defaultChecked != null && (e.defaultChecked = !!t.defaultChecked);
}
function os(e, t, n) {
  if (t.hasOwnProperty("value") || t.hasOwnProperty("defaultValue")) {
    var r = t.type;
    if (!(r !== "submit" && r !== "reset" || t.value !== void 0 && t.value !== null)) return;
    t = "" + e._wrapperState.initialValue, n || t === e.value || (e.value = t), e.defaultValue = t;
  }
  n = e.name, n !== "" && (e.name = ""), e.defaultChecked = !!e._wrapperState.initialChecked, n !== "" && (e.name = n);
}
function vo(e, t, n) {
  (t !== "number" || Jr(e.ownerDocument) !== e) && (n == null ? e.defaultValue = "" + e._wrapperState.initialValue : e.defaultValue !== "" + n && (e.defaultValue = "" + n));
}
var Wn = Array.isArray;
function dn(e, t, n, r) {
  if (e = e.options, t) {
    t = {};
    for (var l = 0; l < n.length; l++) t["$" + n[l]] = !0;
    for (n = 0; n < e.length; n++) l = t.hasOwnProperty("$" + e[n].value), e[n].selected !== l && (e[n].selected = l), l && r && (e[n].defaultSelected = !0);
  } else {
    for (n = "" + Nt(n), t = null, l = 0; l < e.length; l++) {
      if (e[l].value === n) {
        e[l].selected = !0, r && (e[l].defaultSelected = !0);
        return;
      }
      t !== null || e[l].disabled || (t = e[l]);
    }
    t !== null && (t.selected = !0);
  }
}
function go(e, t) {
  if (t.dangerouslySetInnerHTML != null) throw Error(w(91));
  return Z({}, t, { value: void 0, defaultValue: void 0, children: "" + e._wrapperState.initialValue });
}
function is(e, t) {
  var n = t.value;
  if (n == null) {
    if (n = t.children, t = t.defaultValue, n != null) {
      if (t != null) throw Error(w(92));
      if (Wn(n)) {
        if (1 < n.length) throw Error(w(93));
        n = n[0];
      }
      t = n;
    }
    t == null && (t = ""), n = t;
  }
  e._wrapperState = { initialValue: Nt(n) };
}
function Nu(e, t) {
  var n = Nt(t.value), r = Nt(t.defaultValue);
  n != null && (n = "" + n, n !== e.value && (e.value = n), t.defaultValue == null && e.defaultValue !== n && (e.defaultValue = n)), r != null && (e.defaultValue = "" + r);
}
function ss(e) {
  var t = e.textContent;
  t === e._wrapperState.initialValue && t !== "" && t !== null && (e.value = t);
}
function Ru(e) {
  switch (e) {
    case "svg":
      return "http://www.w3.org/2000/svg";
    case "math":
      return "http://www.w3.org/1998/Math/MathML";
    default:
      return "http://www.w3.org/1999/xhtml";
  }
}
function xo(e, t) {
  return e == null || e === "http://www.w3.org/1999/xhtml" ? Ru(t) : e === "http://www.w3.org/2000/svg" && t === "foreignObject" ? "http://www.w3.org/1999/xhtml" : e;
}
var jr, Lu = function(e) {
  return typeof MSApp < "u" && MSApp.execUnsafeLocalFunction ? function(t, n, r, l) {
    MSApp.execUnsafeLocalFunction(function() {
      return e(t, n, r, l);
    });
  } : e;
}(function(e, t) {
  if (e.namespaceURI !== "http://www.w3.org/2000/svg" || "innerHTML" in e) e.innerHTML = t;
  else {
    for (jr = jr || document.createElement("div"), jr.innerHTML = "<svg>" + t.valueOf().toString() + "</svg>", t = jr.firstChild; e.firstChild; ) e.removeChild(e.firstChild);
    for (; t.firstChild; ) e.appendChild(t.firstChild);
  }
});
function tr(e, t) {
  if (t) {
    var n = e.firstChild;
    if (n && n === e.lastChild && n.nodeType === 3) {
      n.nodeValue = t;
      return;
    }
  }
  e.textContent = t;
}
var Hn = {
  animationIterationCount: !0,
  aspectRatio: !0,
  borderImageOutset: !0,
  borderImageSlice: !0,
  borderImageWidth: !0,
  boxFlex: !0,
  boxFlexGroup: !0,
  boxOrdinalGroup: !0,
  columnCount: !0,
  columns: !0,
  flex: !0,
  flexGrow: !0,
  flexPositive: !0,
  flexShrink: !0,
  flexNegative: !0,
  flexOrder: !0,
  gridArea: !0,
  gridRow: !0,
  gridRowEnd: !0,
  gridRowSpan: !0,
  gridRowStart: !0,
  gridColumn: !0,
  gridColumnEnd: !0,
  gridColumnSpan: !0,
  gridColumnStart: !0,
  fontWeight: !0,
  lineClamp: !0,
  lineHeight: !0,
  opacity: !0,
  order: !0,
  orphans: !0,
  tabSize: !0,
  widows: !0,
  zIndex: !0,
  zoom: !0,
  fillOpacity: !0,
  floodOpacity: !0,
  stopOpacity: !0,
  strokeDasharray: !0,
  strokeDashoffset: !0,
  strokeMiterlimit: !0,
  strokeOpacity: !0,
  strokeWidth: !0
}, Zc = ["Webkit", "ms", "Moz", "O"];
Object.keys(Hn).forEach(function(e) {
  Zc.forEach(function(t) {
    t = t + e.charAt(0).toUpperCase() + e.substring(1), Hn[t] = Hn[e];
  });
});
function Ou(e, t, n) {
  return t == null || typeof t == "boolean" || t === "" ? "" : n || typeof t != "number" || t === 0 || Hn.hasOwnProperty(e) && Hn[e] ? ("" + t).trim() : t + "px";
}
function Du(e, t) {
  e = e.style;
  for (var n in t) if (t.hasOwnProperty(n)) {
    var r = n.indexOf("--") === 0, l = Ou(n, t[n], r);
    n === "float" && (n = "cssFloat"), r ? e.setProperty(n, l) : e[n] = l;
  }
}
var Jc = Z({ menuitem: !0 }, { area: !0, base: !0, br: !0, col: !0, embed: !0, hr: !0, img: !0, input: !0, keygen: !0, link: !0, meta: !0, param: !0, source: !0, track: !0, wbr: !0 });
function wo(e, t) {
  if (t) {
    if (Jc[e] && (t.children != null || t.dangerouslySetInnerHTML != null)) throw Error(w(137, e));
    if (t.dangerouslySetInnerHTML != null) {
      if (t.children != null) throw Error(w(60));
      if (typeof t.dangerouslySetInnerHTML != "object" || !("__html" in t.dangerouslySetInnerHTML)) throw Error(w(61));
    }
    if (t.style != null && typeof t.style != "object") throw Error(w(62));
  }
}
function So(e, t) {
  if (e.indexOf("-") === -1) return typeof t.is == "string";
  switch (e) {
    case "annotation-xml":
    case "color-profile":
    case "font-face":
    case "font-face-src":
    case "font-face-uri":
    case "font-face-format":
    case "font-face-name":
    case "missing-glyph":
      return !1;
    default:
      return !0;
  }
}
var ko = null;
function pi(e) {
  return e = e.target || e.srcElement || window, e.correspondingUseElement && (e = e.correspondingUseElement), e.nodeType === 3 ? e.parentNode : e;
}
var Co = null, fn = null, pn = null;
function us(e) {
  if (e = wr(e)) {
    if (typeof Co != "function") throw Error(w(280));
    var t = e.stateNode;
    t && (t = jl(t), Co(e.stateNode, e.type, t));
  }
}
function Iu(e) {
  fn ? pn ? pn.push(e) : pn = [e] : fn = e;
}
function Au() {
  if (fn) {
    var e = fn, t = pn;
    if (pn = fn = null, us(e), t) for (e = 0; e < t.length; e++) us(t[e]);
  }
}
function Mu(e, t) {
  return e(t);
}
function Bu() {
}
var Wl = !1;
function Fu(e, t, n) {
  if (Wl) return e(t, n);
  Wl = !0;
  try {
    return Mu(e, t, n);
  } finally {
    Wl = !1, (fn !== null || pn !== null) && (Bu(), Au());
  }
}
function nr(e, t) {
  var n = e.stateNode;
  if (n === null) return null;
  var r = jl(n);
  if (r === null) return null;
  n = r[t];
  e: switch (t) {
    case "onClick":
    case "onClickCapture":
    case "onDoubleClick":
    case "onDoubleClickCapture":
    case "onMouseDown":
    case "onMouseDownCapture":
    case "onMouseMove":
    case "onMouseMoveCapture":
    case "onMouseUp":
    case "onMouseUpCapture":
    case "onMouseEnter":
      (r = !r.disabled) || (e = e.type, r = !(e === "button" || e === "input" || e === "select" || e === "textarea")), e = !r;
      break e;
    default:
      e = !1;
  }
  if (e) return null;
  if (n && typeof n != "function") throw Error(w(231, t, typeof n));
  return n;
}
var Eo = !1;
if (ut) try {
  var On = {};
  Object.defineProperty(On, "passive", { get: function() {
    Eo = !0;
  } }), window.addEventListener("test", On, On), window.removeEventListener("test", On, On);
} catch {
  Eo = !1;
}
function qc(e, t, n, r, l, o, i, s, u) {
  var f = Array.prototype.slice.call(arguments, 3);
  try {
    t.apply(n, f);
  } catch (m) {
    this.onError(m);
  }
}
var Kn = !1, qr = null, br = !1, jo = null, bc = { onError: function(e) {
  Kn = !0, qr = e;
} };
function ed(e, t, n, r, l, o, i, s, u) {
  Kn = !1, qr = null, qc.apply(bc, arguments);
}
function td(e, t, n, r, l, o, i, s, u) {
  if (ed.apply(this, arguments), Kn) {
    if (Kn) {
      var f = qr;
      Kn = !1, qr = null;
    } else throw Error(w(198));
    br || (br = !0, jo = f);
  }
}
function Zt(e) {
  var t = e, n = e;
  if (e.alternate) for (; t.return; ) t = t.return;
  else {
    e = t;
    do
      t = e, t.flags & 4098 && (n = t.return), e = t.return;
    while (e);
  }
  return t.tag === 3 ? n : null;
}
function Uu(e) {
  if (e.tag === 13) {
    var t = e.memoizedState;
    if (t === null && (e = e.alternate, e !== null && (t = e.memoizedState)), t !== null) return t.dehydrated;
  }
  return null;
}
function as(e) {
  if (Zt(e) !== e) throw Error(w(188));
}
function nd(e) {
  var t = e.alternate;
  if (!t) {
    if (t = Zt(e), t === null) throw Error(w(188));
    return t !== e ? null : e;
  }
  for (var n = e, r = t; ; ) {
    var l = n.return;
    if (l === null) break;
    var o = l.alternate;
    if (o === null) {
      if (r = l.return, r !== null) {
        n = r;
        continue;
      }
      break;
    }
    if (l.child === o.child) {
      for (o = l.child; o; ) {
        if (o === n) return as(l), e;
        if (o === r) return as(l), t;
        o = o.sibling;
      }
      throw Error(w(188));
    }
    if (n.return !== r.return) n = l, r = o;
    else {
      for (var i = !1, s = l.child; s; ) {
        if (s === n) {
          i = !0, n = l, r = o;
          break;
        }
        if (s === r) {
          i = !0, r = l, n = o;
          break;
        }
        s = s.sibling;
      }
      if (!i) {
        for (s = o.child; s; ) {
          if (s === n) {
            i = !0, n = o, r = l;
            break;
          }
          if (s === r) {
            i = !0, r = o, n = l;
            break;
          }
          s = s.sibling;
        }
        if (!i) throw Error(w(189));
      }
    }
    if (n.alternate !== r) throw Error(w(190));
  }
  if (n.tag !== 3) throw Error(w(188));
  return n.stateNode.current === n ? e : t;
}
function Wu(e) {
  return e = nd(e), e !== null ? $u(e) : null;
}
function $u(e) {
  if (e.tag === 5 || e.tag === 6) return e;
  for (e = e.child; e !== null; ) {
    var t = $u(e);
    if (t !== null) return t;
    e = e.sibling;
  }
  return null;
}
var Vu = Pe.unstable_scheduleCallback, cs = Pe.unstable_cancelCallback, rd = Pe.unstable_shouldYield, ld = Pe.unstable_requestPaint, q = Pe.unstable_now, od = Pe.unstable_getCurrentPriorityLevel, mi = Pe.unstable_ImmediatePriority, Hu = Pe.unstable_UserBlockingPriority, el = Pe.unstable_NormalPriority, id = Pe.unstable_LowPriority, Ku = Pe.unstable_IdlePriority, Sl = null, tt = null;
function sd(e) {
  if (tt && typeof tt.onCommitFiberRoot == "function") try {
    tt.onCommitFiberRoot(Sl, e, void 0, (e.current.flags & 128) === 128);
  } catch {
  }
}
var Ye = Math.clz32 ? Math.clz32 : cd, ud = Math.log, ad = Math.LN2;
function cd(e) {
  return e >>>= 0, e === 0 ? 32 : 31 - (ud(e) / ad | 0) | 0;
}
var _r = 64, zr = 4194304;
function $n(e) {
  switch (e & -e) {
    case 1:
      return 1;
    case 2:
      return 2;
    case 4:
      return 4;
    case 8:
      return 8;
    case 16:
      return 16;
    case 32:
      return 32;
    case 64:
    case 128:
    case 256:
    case 512:
    case 1024:
    case 2048:
    case 4096:
    case 8192:
    case 16384:
    case 32768:
    case 65536:
    case 131072:
    case 262144:
    case 524288:
    case 1048576:
    case 2097152:
      return e & 4194240;
    case 4194304:
    case 8388608:
    case 16777216:
    case 33554432:
    case 67108864:
      return e & 130023424;
    case 134217728:
      return 134217728;
    case 268435456:
      return 268435456;
    case 536870912:
      return 536870912;
    case 1073741824:
      return 1073741824;
    default:
      return e;
  }
}
function tl(e, t) {
  var n = e.pendingLanes;
  if (n === 0) return 0;
  var r = 0, l = e.suspendedLanes, o = e.pingedLanes, i = n & 268435455;
  if (i !== 0) {
    var s = i & ~l;
    s !== 0 ? r = $n(s) : (o &= i, o !== 0 && (r = $n(o)));
  } else i = n & ~l, i !== 0 ? r = $n(i) : o !== 0 && (r = $n(o));
  if (r === 0) return 0;
  if (t !== 0 && t !== r && !(t & l) && (l = r & -r, o = t & -t, l >= o || l === 16 && (o & 4194240) !== 0)) return t;
  if (r & 4 && (r |= n & 16), t = e.entangledLanes, t !== 0) for (e = e.entanglements, t &= r; 0 < t; ) n = 31 - Ye(t), l = 1 << n, r |= e[n], t &= ~l;
  return r;
}
function dd(e, t) {
  switch (e) {
    case 1:
    case 2:
    case 4:
      return t + 250;
    case 8:
    case 16:
    case 32:
    case 64:
    case 128:
    case 256:
    case 512:
    case 1024:
    case 2048:
    case 4096:
    case 8192:
    case 16384:
    case 32768:
    case 65536:
    case 131072:
    case 262144:
    case 524288:
    case 1048576:
    case 2097152:
      return t + 5e3;
    case 4194304:
    case 8388608:
    case 16777216:
    case 33554432:
    case 67108864:
      return -1;
    case 134217728:
    case 268435456:
    case 536870912:
    case 1073741824:
      return -1;
    default:
      return -1;
  }
}
function fd(e, t) {
  for (var n = e.suspendedLanes, r = e.pingedLanes, l = e.expirationTimes, o = e.pendingLanes; 0 < o; ) {
    var i = 31 - Ye(o), s = 1 << i, u = l[i];
    u === -1 ? (!(s & n) || s & r) && (l[i] = dd(s, t)) : u <= t && (e.expiredLanes |= s), o &= ~s;
  }
}
function _o(e) {
  return e = e.pendingLanes & -1073741825, e !== 0 ? e : e & 1073741824 ? 1073741824 : 0;
}
function Qu() {
  var e = _r;
  return _r <<= 1, !(_r & 4194240) && (_r = 64), e;
}
function $l(e) {
  for (var t = [], n = 0; 31 > n; n++) t.push(e);
  return t;
}
function gr(e, t, n) {
  e.pendingLanes |= t, t !== 536870912 && (e.suspendedLanes = 0, e.pingedLanes = 0), e = e.eventTimes, t = 31 - Ye(t), e[t] = n;
}
function pd(e, t) {
  var n = e.pendingLanes & ~t;
  e.pendingLanes = t, e.suspendedLanes = 0, e.pingedLanes = 0, e.expiredLanes &= t, e.mutableReadLanes &= t, e.entangledLanes &= t, t = e.entanglements;
  var r = e.eventTimes;
  for (e = e.expirationTimes; 0 < n; ) {
    var l = 31 - Ye(n), o = 1 << l;
    t[l] = 0, r[l] = -1, e[l] = -1, n &= ~o;
  }
}
function hi(e, t) {
  var n = e.entangledLanes |= t;
  for (e = e.entanglements; n; ) {
    var r = 31 - Ye(n), l = 1 << r;
    l & t | e[r] & t && (e[r] |= t), n &= ~l;
  }
}
var W = 0;
function Yu(e) {
  return e &= -e, 1 < e ? 4 < e ? e & 268435455 ? 16 : 536870912 : 4 : 1;
}
var Xu, yi, Gu, Zu, Ju, zo = !1, Tr = [], kt = null, Ct = null, Et = null, rr = /* @__PURE__ */ new Map(), lr = /* @__PURE__ */ new Map(), vt = [], md = "mousedown mouseup touchcancel touchend touchstart auxclick dblclick pointercancel pointerdown pointerup dragend dragstart drop compositionend compositionstart keydown keypress keyup input textInput copy cut paste click change contextmenu reset submit".split(" ");
function ds(e, t) {
  switch (e) {
    case "focusin":
    case "focusout":
      kt = null;
      break;
    case "dragenter":
    case "dragleave":
      Ct = null;
      break;
    case "mouseover":
    case "mouseout":
      Et = null;
      break;
    case "pointerover":
    case "pointerout":
      rr.delete(t.pointerId);
      break;
    case "gotpointercapture":
    case "lostpointercapture":
      lr.delete(t.pointerId);
  }
}
function Dn(e, t, n, r, l, o) {
  return e === null || e.nativeEvent !== o ? (e = { blockedOn: t, domEventName: n, eventSystemFlags: r, nativeEvent: o, targetContainers: [l] }, t !== null && (t = wr(t), t !== null && yi(t)), e) : (e.eventSystemFlags |= r, t = e.targetContainers, l !== null && t.indexOf(l) === -1 && t.push(l), e);
}
function hd(e, t, n, r, l) {
  switch (t) {
    case "focusin":
      return kt = Dn(kt, e, t, n, r, l), !0;
    case "dragenter":
      return Ct = Dn(Ct, e, t, n, r, l), !0;
    case "mouseover":
      return Et = Dn(Et, e, t, n, r, l), !0;
    case "pointerover":
      var o = l.pointerId;
      return rr.set(o, Dn(rr.get(o) || null, e, t, n, r, l)), !0;
    case "gotpointercapture":
      return o = l.pointerId, lr.set(o, Dn(lr.get(o) || null, e, t, n, r, l)), !0;
  }
  return !1;
}
function qu(e) {
  var t = Ft(e.target);
  if (t !== null) {
    var n = Zt(t);
    if (n !== null) {
      if (t = n.tag, t === 13) {
        if (t = Uu(n), t !== null) {
          e.blockedOn = t, Ju(e.priority, function() {
            Gu(n);
          });
          return;
        }
      } else if (t === 3 && n.stateNode.current.memoizedState.isDehydrated) {
        e.blockedOn = n.tag === 3 ? n.stateNode.containerInfo : null;
        return;
      }
    }
  }
  e.blockedOn = null;
}
function Wr(e) {
  if (e.blockedOn !== null) return !1;
  for (var t = e.targetContainers; 0 < t.length; ) {
    var n = To(e.domEventName, e.eventSystemFlags, t[0], e.nativeEvent);
    if (n === null) {
      n = e.nativeEvent;
      var r = new n.constructor(n.type, n);
      ko = r, n.target.dispatchEvent(r), ko = null;
    } else return t = wr(n), t !== null && yi(t), e.blockedOn = n, !1;
    t.shift();
  }
  return !0;
}
function fs(e, t, n) {
  Wr(e) && n.delete(t);
}
function yd() {
  zo = !1, kt !== null && Wr(kt) && (kt = null), Ct !== null && Wr(Ct) && (Ct = null), Et !== null && Wr(Et) && (Et = null), rr.forEach(fs), lr.forEach(fs);
}
function In(e, t) {
  e.blockedOn === t && (e.blockedOn = null, zo || (zo = !0, Pe.unstable_scheduleCallback(Pe.unstable_NormalPriority, yd)));
}
function or(e) {
  function t(l) {
    return In(l, e);
  }
  if (0 < Tr.length) {
    In(Tr[0], e);
    for (var n = 1; n < Tr.length; n++) {
      var r = Tr[n];
      r.blockedOn === e && (r.blockedOn = null);
    }
  }
  for (kt !== null && In(kt, e), Ct !== null && In(Ct, e), Et !== null && In(Et, e), rr.forEach(t), lr.forEach(t), n = 0; n < vt.length; n++) r = vt[n], r.blockedOn === e && (r.blockedOn = null);
  for (; 0 < vt.length && (n = vt[0], n.blockedOn === null); ) qu(n), n.blockedOn === null && vt.shift();
}
var mn = ft.ReactCurrentBatchConfig, nl = !0;
function vd(e, t, n, r) {
  var l = W, o = mn.transition;
  mn.transition = null;
  try {
    W = 1, vi(e, t, n, r);
  } finally {
    W = l, mn.transition = o;
  }
}
function gd(e, t, n, r) {
  var l = W, o = mn.transition;
  mn.transition = null;
  try {
    W = 4, vi(e, t, n, r);
  } finally {
    W = l, mn.transition = o;
  }
}
function vi(e, t, n, r) {
  if (nl) {
    var l = To(e, t, n, r);
    if (l === null) ql(e, t, r, rl, n), ds(e, r);
    else if (hd(l, e, t, n, r)) r.stopPropagation();
    else if (ds(e, r), t & 4 && -1 < md.indexOf(e)) {
      for (; l !== null; ) {
        var o = wr(l);
        if (o !== null && Xu(o), o = To(e, t, n, r), o === null && ql(e, t, r, rl, n), o === l) break;
        l = o;
      }
      l !== null && r.stopPropagation();
    } else ql(e, t, r, null, n);
  }
}
var rl = null;
function To(e, t, n, r) {
  if (rl = null, e = pi(r), e = Ft(e), e !== null) if (t = Zt(e), t === null) e = null;
  else if (n = t.tag, n === 13) {
    if (e = Uu(t), e !== null) return e;
    e = null;
  } else if (n === 3) {
    if (t.stateNode.current.memoizedState.isDehydrated) return t.tag === 3 ? t.stateNode.containerInfo : null;
    e = null;
  } else t !== e && (e = null);
  return rl = e, null;
}
function bu(e) {
  switch (e) {
    case "cancel":
    case "click":
    case "close":
    case "contextmenu":
    case "copy":
    case "cut":
    case "auxclick":
    case "dblclick":
    case "dragend":
    case "dragstart":
    case "drop":
    case "focusin":
    case "focusout":
    case "input":
    case "invalid":
    case "keydown":
    case "keypress":
    case "keyup":
    case "mousedown":
    case "mouseup":
    case "paste":
    case "pause":
    case "play":
    case "pointercancel":
    case "pointerdown":
    case "pointerup":
    case "ratechange":
    case "reset":
    case "resize":
    case "seeked":
    case "submit":
    case "touchcancel":
    case "touchend":
    case "touchstart":
    case "volumechange":
    case "change":
    case "selectionchange":
    case "textInput":
    case "compositionstart":
    case "compositionend":
    case "compositionupdate":
    case "beforeblur":
    case "afterblur":
    case "beforeinput":
    case "blur":
    case "fullscreenchange":
    case "focus":
    case "hashchange":
    case "popstate":
    case "select":
    case "selectstart":
      return 1;
    case "drag":
    case "dragenter":
    case "dragexit":
    case "dragleave":
    case "dragover":
    case "mousemove":
    case "mouseout":
    case "mouseover":
    case "pointermove":
    case "pointerout":
    case "pointerover":
    case "scroll":
    case "toggle":
    case "touchmove":
    case "wheel":
    case "mouseenter":
    case "mouseleave":
    case "pointerenter":
    case "pointerleave":
      return 4;
    case "message":
      switch (od()) {
        case mi:
          return 1;
        case Hu:
          return 4;
        case el:
        case id:
          return 16;
        case Ku:
          return 536870912;
        default:
          return 16;
      }
    default:
      return 16;
  }
}
var xt = null, gi = null, $r = null;
function ea() {
  if ($r) return $r;
  var e, t = gi, n = t.length, r, l = "value" in xt ? xt.value : xt.textContent, o = l.length;
  for (e = 0; e < n && t[e] === l[e]; e++) ;
  var i = n - e;
  for (r = 1; r <= i && t[n - r] === l[o - r]; r++) ;
  return $r = l.slice(e, 1 < r ? 1 - r : void 0);
}
function Vr(e) {
  var t = e.keyCode;
  return "charCode" in e ? (e = e.charCode, e === 0 && t === 13 && (e = 13)) : e = t, e === 10 && (e = 13), 32 <= e || e === 13 ? e : 0;
}
function Pr() {
  return !0;
}
function ps() {
  return !1;
}
function Re(e) {
  function t(n, r, l, o, i) {
    this._reactName = n, this._targetInst = l, this.type = r, this.nativeEvent = o, this.target = i, this.currentTarget = null;
    for (var s in e) e.hasOwnProperty(s) && (n = e[s], this[s] = n ? n(o) : o[s]);
    return this.isDefaultPrevented = (o.defaultPrevented != null ? o.defaultPrevented : o.returnValue === !1) ? Pr : ps, this.isPropagationStopped = ps, this;
  }
  return Z(t.prototype, { preventDefault: function() {
    this.defaultPrevented = !0;
    var n = this.nativeEvent;
    n && (n.preventDefault ? n.preventDefault() : typeof n.returnValue != "unknown" && (n.returnValue = !1), this.isDefaultPrevented = Pr);
  }, stopPropagation: function() {
    var n = this.nativeEvent;
    n && (n.stopPropagation ? n.stopPropagation() : typeof n.cancelBubble != "unknown" && (n.cancelBubble = !0), this.isPropagationStopped = Pr);
  }, persist: function() {
  }, isPersistent: Pr }), t;
}
var _n = { eventPhase: 0, bubbles: 0, cancelable: 0, timeStamp: function(e) {
  return e.timeStamp || Date.now();
}, defaultPrevented: 0, isTrusted: 0 }, xi = Re(_n), xr = Z({}, _n, { view: 0, detail: 0 }), xd = Re(xr), Vl, Hl, An, kl = Z({}, xr, { screenX: 0, screenY: 0, clientX: 0, clientY: 0, pageX: 0, pageY: 0, ctrlKey: 0, shiftKey: 0, altKey: 0, metaKey: 0, getModifierState: wi, button: 0, buttons: 0, relatedTarget: function(e) {
  return e.relatedTarget === void 0 ? e.fromElement === e.srcElement ? e.toElement : e.fromElement : e.relatedTarget;
}, movementX: function(e) {
  return "movementX" in e ? e.movementX : (e !== An && (An && e.type === "mousemove" ? (Vl = e.screenX - An.screenX, Hl = e.screenY - An.screenY) : Hl = Vl = 0, An = e), Vl);
}, movementY: function(e) {
  return "movementY" in e ? e.movementY : Hl;
} }), ms = Re(kl), wd = Z({}, kl, { dataTransfer: 0 }), Sd = Re(wd), kd = Z({}, xr, { relatedTarget: 0 }), Kl = Re(kd), Cd = Z({}, _n, { animationName: 0, elapsedTime: 0, pseudoElement: 0 }), Ed = Re(Cd), jd = Z({}, _n, { clipboardData: function(e) {
  return "clipboardData" in e ? e.clipboardData : window.clipboardData;
} }), _d = Re(jd), zd = Z({}, _n, { data: 0 }), hs = Re(zd), Td = {
  Esc: "Escape",
  Spacebar: " ",
  Left: "ArrowLeft",
  Up: "ArrowUp",
  Right: "ArrowRight",
  Down: "ArrowDown",
  Del: "Delete",
  Win: "OS",
  Menu: "ContextMenu",
  Apps: "ContextMenu",
  Scroll: "ScrollLock",
  MozPrintableKey: "Unidentified"
}, Pd = {
  8: "Backspace",
  9: "Tab",
  12: "Clear",
  13: "Enter",
  16: "Shift",
  17: "Control",
  18: "Alt",
  19: "Pause",
  20: "CapsLock",
  27: "Escape",
  32: " ",
  33: "PageUp",
  34: "PageDown",
  35: "End",
  36: "Home",
  37: "ArrowLeft",
  38: "ArrowUp",
  39: "ArrowRight",
  40: "ArrowDown",
  45: "Insert",
  46: "Delete",
  112: "F1",
  113: "F2",
  114: "F3",
  115: "F4",
  116: "F5",
  117: "F6",
  118: "F7",
  119: "F8",
  120: "F9",
  121: "F10",
  122: "F11",
  123: "F12",
  144: "NumLock",
  145: "ScrollLock",
  224: "Meta"
}, Nd = { Alt: "altKey", Control: "ctrlKey", Meta: "metaKey", Shift: "shiftKey" };
function Rd(e) {
  var t = this.nativeEvent;
  return t.getModifierState ? t.getModifierState(e) : (e = Nd[e]) ? !!t[e] : !1;
}
function wi() {
  return Rd;
}
var Ld = Z({}, xr, { key: function(e) {
  if (e.key) {
    var t = Td[e.key] || e.key;
    if (t !== "Unidentified") return t;
  }
  return e.type === "keypress" ? (e = Vr(e), e === 13 ? "Enter" : String.fromCharCode(e)) : e.type === "keydown" || e.type === "keyup" ? Pd[e.keyCode] || "Unidentified" : "";
}, code: 0, location: 0, ctrlKey: 0, shiftKey: 0, altKey: 0, metaKey: 0, repeat: 0, locale: 0, getModifierState: wi, charCode: function(e) {
  return e.type === "keypress" ? Vr(e) : 0;
}, keyCode: function(e) {
  return e.type === "keydown" || e.type === "keyup" ? e.keyCode : 0;
}, which: function(e) {
  return e.type === "keypress" ? Vr(e) : e.type === "keydown" || e.type === "keyup" ? e.keyCode : 0;
} }), Od = Re(Ld), Dd = Z({}, kl, { pointerId: 0, width: 0, height: 0, pressure: 0, tangentialPressure: 0, tiltX: 0, tiltY: 0, twist: 0, pointerType: 0, isPrimary: 0 }), ys = Re(Dd), Id = Z({}, xr, { touches: 0, targetTouches: 0, changedTouches: 0, altKey: 0, metaKey: 0, ctrlKey: 0, shiftKey: 0, getModifierState: wi }), Ad = Re(Id), Md = Z({}, _n, { propertyName: 0, elapsedTime: 0, pseudoElement: 0 }), Bd = Re(Md), Fd = Z({}, kl, {
  deltaX: function(e) {
    return "deltaX" in e ? e.deltaX : "wheelDeltaX" in e ? -e.wheelDeltaX : 0;
  },
  deltaY: function(e) {
    return "deltaY" in e ? e.deltaY : "wheelDeltaY" in e ? -e.wheelDeltaY : "wheelDelta" in e ? -e.wheelDelta : 0;
  },
  deltaZ: 0,
  deltaMode: 0
}), Ud = Re(Fd), Wd = [9, 13, 27, 32], Si = ut && "CompositionEvent" in window, Qn = null;
ut && "documentMode" in document && (Qn = document.documentMode);
var $d = ut && "TextEvent" in window && !Qn, ta = ut && (!Si || Qn && 8 < Qn && 11 >= Qn), vs = " ", gs = !1;
function na(e, t) {
  switch (e) {
    case "keyup":
      return Wd.indexOf(t.keyCode) !== -1;
    case "keydown":
      return t.keyCode !== 229;
    case "keypress":
    case "mousedown":
    case "focusout":
      return !0;
    default:
      return !1;
  }
}
function ra(e) {
  return e = e.detail, typeof e == "object" && "data" in e ? e.data : null;
}
var en = !1;
function Vd(e, t) {
  switch (e) {
    case "compositionend":
      return ra(t);
    case "keypress":
      return t.which !== 32 ? null : (gs = !0, vs);
    case "textInput":
      return e = t.data, e === vs && gs ? null : e;
    default:
      return null;
  }
}
function Hd(e, t) {
  if (en) return e === "compositionend" || !Si && na(e, t) ? (e = ea(), $r = gi = xt = null, en = !1, e) : null;
  switch (e) {
    case "paste":
      return null;
    case "keypress":
      if (!(t.ctrlKey || t.altKey || t.metaKey) || t.ctrlKey && t.altKey) {
        if (t.char && 1 < t.char.length) return t.char;
        if (t.which) return String.fromCharCode(t.which);
      }
      return null;
    case "compositionend":
      return ta && t.locale !== "ko" ? null : t.data;
    default:
      return null;
  }
}
var Kd = { color: !0, date: !0, datetime: !0, "datetime-local": !0, email: !0, month: !0, number: !0, password: !0, range: !0, search: !0, tel: !0, text: !0, time: !0, url: !0, week: !0 };
function xs(e) {
  var t = e && e.nodeName && e.nodeName.toLowerCase();
  return t === "input" ? !!Kd[e.type] : t === "textarea";
}
function la(e, t, n, r) {
  Iu(r), t = ll(t, "onChange"), 0 < t.length && (n = new xi("onChange", "change", null, n, r), e.push({ event: n, listeners: t }));
}
var Yn = null, ir = null;
function Qd(e) {
  ha(e, 0);
}
function Cl(e) {
  var t = rn(e);
  if (Tu(t)) return e;
}
function Yd(e, t) {
  if (e === "change") return t;
}
var oa = !1;
if (ut) {
  var Ql;
  if (ut) {
    var Yl = "oninput" in document;
    if (!Yl) {
      var ws = document.createElement("div");
      ws.setAttribute("oninput", "return;"), Yl = typeof ws.oninput == "function";
    }
    Ql = Yl;
  } else Ql = !1;
  oa = Ql && (!document.documentMode || 9 < document.documentMode);
}
function Ss() {
  Yn && (Yn.detachEvent("onpropertychange", ia), ir = Yn = null);
}
function ia(e) {
  if (e.propertyName === "value" && Cl(ir)) {
    var t = [];
    la(t, ir, e, pi(e)), Fu(Qd, t);
  }
}
function Xd(e, t, n) {
  e === "focusin" ? (Ss(), Yn = t, ir = n, Yn.attachEvent("onpropertychange", ia)) : e === "focusout" && Ss();
}
function Gd(e) {
  if (e === "selectionchange" || e === "keyup" || e === "keydown") return Cl(ir);
}
function Zd(e, t) {
  if (e === "click") return Cl(t);
}
function Jd(e, t) {
  if (e === "input" || e === "change") return Cl(t);
}
function qd(e, t) {
  return e === t && (e !== 0 || 1 / e === 1 / t) || e !== e && t !== t;
}
var Ge = typeof Object.is == "function" ? Object.is : qd;
function sr(e, t) {
  if (Ge(e, t)) return !0;
  if (typeof e != "object" || e === null || typeof t != "object" || t === null) return !1;
  var n = Object.keys(e), r = Object.keys(t);
  if (n.length !== r.length) return !1;
  for (r = 0; r < n.length; r++) {
    var l = n[r];
    if (!ao.call(t, l) || !Ge(e[l], t[l])) return !1;
  }
  return !0;
}
function ks(e) {
  for (; e && e.firstChild; ) e = e.firstChild;
  return e;
}
function Cs(e, t) {
  var n = ks(e);
  e = 0;
  for (var r; n; ) {
    if (n.nodeType === 3) {
      if (r = e + n.textContent.length, e <= t && r >= t) return { node: n, offset: t - e };
      e = r;
    }
    e: {
      for (; n; ) {
        if (n.nextSibling) {
          n = n.nextSibling;
          break e;
        }
        n = n.parentNode;
      }
      n = void 0;
    }
    n = ks(n);
  }
}
function sa(e, t) {
  return e && t ? e === t ? !0 : e && e.nodeType === 3 ? !1 : t && t.nodeType === 3 ? sa(e, t.parentNode) : "contains" in e ? e.contains(t) : e.compareDocumentPosition ? !!(e.compareDocumentPosition(t) & 16) : !1 : !1;
}
function ua() {
  for (var e = window, t = Jr(); t instanceof e.HTMLIFrameElement; ) {
    try {
      var n = typeof t.contentWindow.location.href == "string";
    } catch {
      n = !1;
    }
    if (n) e = t.contentWindow;
    else break;
    t = Jr(e.document);
  }
  return t;
}
function ki(e) {
  var t = e && e.nodeName && e.nodeName.toLowerCase();
  return t && (t === "input" && (e.type === "text" || e.type === "search" || e.type === "tel" || e.type === "url" || e.type === "password") || t === "textarea" || e.contentEditable === "true");
}
function bd(e) {
  var t = ua(), n = e.focusedElem, r = e.selectionRange;
  if (t !== n && n && n.ownerDocument && sa(n.ownerDocument.documentElement, n)) {
    if (r !== null && ki(n)) {
      if (t = r.start, e = r.end, e === void 0 && (e = t), "selectionStart" in n) n.selectionStart = t, n.selectionEnd = Math.min(e, n.value.length);
      else if (e = (t = n.ownerDocument || document) && t.defaultView || window, e.getSelection) {
        e = e.getSelection();
        var l = n.textContent.length, o = Math.min(r.start, l);
        r = r.end === void 0 ? o : Math.min(r.end, l), !e.extend && o > r && (l = r, r = o, o = l), l = Cs(n, o);
        var i = Cs(
          n,
          r
        );
        l && i && (e.rangeCount !== 1 || e.anchorNode !== l.node || e.anchorOffset !== l.offset || e.focusNode !== i.node || e.focusOffset !== i.offset) && (t = t.createRange(), t.setStart(l.node, l.offset), e.removeAllRanges(), o > r ? (e.addRange(t), e.extend(i.node, i.offset)) : (t.setEnd(i.node, i.offset), e.addRange(t)));
      }
    }
    for (t = [], e = n; e = e.parentNode; ) e.nodeType === 1 && t.push({ element: e, left: e.scrollLeft, top: e.scrollTop });
    for (typeof n.focus == "function" && n.focus(), n = 0; n < t.length; n++) e = t[n], e.element.scrollLeft = e.left, e.element.scrollTop = e.top;
  }
}
var ef = ut && "documentMode" in document && 11 >= document.documentMode, tn = null, Po = null, Xn = null, No = !1;
function Es(e, t, n) {
  var r = n.window === n ? n.document : n.nodeType === 9 ? n : n.ownerDocument;
  No || tn == null || tn !== Jr(r) || (r = tn, "selectionStart" in r && ki(r) ? r = { start: r.selectionStart, end: r.selectionEnd } : (r = (r.ownerDocument && r.ownerDocument.defaultView || window).getSelection(), r = { anchorNode: r.anchorNode, anchorOffset: r.anchorOffset, focusNode: r.focusNode, focusOffset: r.focusOffset }), Xn && sr(Xn, r) || (Xn = r, r = ll(Po, "onSelect"), 0 < r.length && (t = new xi("onSelect", "select", null, t, n), e.push({ event: t, listeners: r }), t.target = tn)));
}
function Nr(e, t) {
  var n = {};
  return n[e.toLowerCase()] = t.toLowerCase(), n["Webkit" + e] = "webkit" + t, n["Moz" + e] = "moz" + t, n;
}
var nn = { animationend: Nr("Animation", "AnimationEnd"), animationiteration: Nr("Animation", "AnimationIteration"), animationstart: Nr("Animation", "AnimationStart"), transitionend: Nr("Transition", "TransitionEnd") }, Xl = {}, aa = {};
ut && (aa = document.createElement("div").style, "AnimationEvent" in window || (delete nn.animationend.animation, delete nn.animationiteration.animation, delete nn.animationstart.animation), "TransitionEvent" in window || delete nn.transitionend.transition);
function El(e) {
  if (Xl[e]) return Xl[e];
  if (!nn[e]) return e;
  var t = nn[e], n;
  for (n in t) if (t.hasOwnProperty(n) && n in aa) return Xl[e] = t[n];
  return e;
}
var ca = El("animationend"), da = El("animationiteration"), fa = El("animationstart"), pa = El("transitionend"), ma = /* @__PURE__ */ new Map(), js = "abort auxClick cancel canPlay canPlayThrough click close contextMenu copy cut drag dragEnd dragEnter dragExit dragLeave dragOver dragStart drop durationChange emptied encrypted ended error gotPointerCapture input invalid keyDown keyPress keyUp load loadedData loadedMetadata loadStart lostPointerCapture mouseDown mouseMove mouseOut mouseOver mouseUp paste pause play playing pointerCancel pointerDown pointerMove pointerOut pointerOver pointerUp progress rateChange reset resize seeked seeking stalled submit suspend timeUpdate touchCancel touchEnd touchStart volumeChange scroll toggle touchMove waiting wheel".split(" ");
function Lt(e, t) {
  ma.set(e, t), Gt(t, [e]);
}
for (var Gl = 0; Gl < js.length; Gl++) {
  var Zl = js[Gl], tf = Zl.toLowerCase(), nf = Zl[0].toUpperCase() + Zl.slice(1);
  Lt(tf, "on" + nf);
}
Lt(ca, "onAnimationEnd");
Lt(da, "onAnimationIteration");
Lt(fa, "onAnimationStart");
Lt("dblclick", "onDoubleClick");
Lt("focusin", "onFocus");
Lt("focusout", "onBlur");
Lt(pa, "onTransitionEnd");
vn("onMouseEnter", ["mouseout", "mouseover"]);
vn("onMouseLeave", ["mouseout", "mouseover"]);
vn("onPointerEnter", ["pointerout", "pointerover"]);
vn("onPointerLeave", ["pointerout", "pointerover"]);
Gt("onChange", "change click focusin focusout input keydown keyup selectionchange".split(" "));
Gt("onSelect", "focusout contextmenu dragend focusin keydown keyup mousedown mouseup selectionchange".split(" "));
Gt("onBeforeInput", ["compositionend", "keypress", "textInput", "paste"]);
Gt("onCompositionEnd", "compositionend focusout keydown keypress keyup mousedown".split(" "));
Gt("onCompositionStart", "compositionstart focusout keydown keypress keyup mousedown".split(" "));
Gt("onCompositionUpdate", "compositionupdate focusout keydown keypress keyup mousedown".split(" "));
var Vn = "abort canplay canplaythrough durationchange emptied encrypted ended error loadeddata loadedmetadata loadstart pause play playing progress ratechange resize seeked seeking stalled suspend timeupdate volumechange waiting".split(" "), rf = new Set("cancel close invalid load scroll toggle".split(" ").concat(Vn));
function _s(e, t, n) {
  var r = e.type || "unknown-event";
  e.currentTarget = n, td(r, t, void 0, e), e.currentTarget = null;
}
function ha(e, t) {
  t = (t & 4) !== 0;
  for (var n = 0; n < e.length; n++) {
    var r = e[n], l = r.event;
    r = r.listeners;
    e: {
      var o = void 0;
      if (t) for (var i = r.length - 1; 0 <= i; i--) {
        var s = r[i], u = s.instance, f = s.currentTarget;
        if (s = s.listener, u !== o && l.isPropagationStopped()) break e;
        _s(l, s, f), o = u;
      }
      else for (i = 0; i < r.length; i++) {
        if (s = r[i], u = s.instance, f = s.currentTarget, s = s.listener, u !== o && l.isPropagationStopped()) break e;
        _s(l, s, f), o = u;
      }
    }
  }
  if (br) throw e = jo, br = !1, jo = null, e;
}
function K(e, t) {
  var n = t[Io];
  n === void 0 && (n = t[Io] = /* @__PURE__ */ new Set());
  var r = e + "__bubble";
  n.has(r) || (ya(t, e, 2, !1), n.add(r));
}
function Jl(e, t, n) {
  var r = 0;
  t && (r |= 4), ya(n, e, r, t);
}
var Rr = "_reactListening" + Math.random().toString(36).slice(2);
function ur(e) {
  if (!e[Rr]) {
    e[Rr] = !0, Cu.forEach(function(n) {
      n !== "selectionchange" && (rf.has(n) || Jl(n, !1, e), Jl(n, !0, e));
    });
    var t = e.nodeType === 9 ? e : e.ownerDocument;
    t === null || t[Rr] || (t[Rr] = !0, Jl("selectionchange", !1, t));
  }
}
function ya(e, t, n, r) {
  switch (bu(t)) {
    case 1:
      var l = vd;
      break;
    case 4:
      l = gd;
      break;
    default:
      l = vi;
  }
  n = l.bind(null, t, n, e), l = void 0, !Eo || t !== "touchstart" && t !== "touchmove" && t !== "wheel" || (l = !0), r ? l !== void 0 ? e.addEventListener(t, n, { capture: !0, passive: l }) : e.addEventListener(t, n, !0) : l !== void 0 ? e.addEventListener(t, n, { passive: l }) : e.addEventListener(t, n, !1);
}
function ql(e, t, n, r, l) {
  var o = r;
  if (!(t & 1) && !(t & 2) && r !== null) e: for (; ; ) {
    if (r === null) return;
    var i = r.tag;
    if (i === 3 || i === 4) {
      var s = r.stateNode.containerInfo;
      if (s === l || s.nodeType === 8 && s.parentNode === l) break;
      if (i === 4) for (i = r.return; i !== null; ) {
        var u = i.tag;
        if ((u === 3 || u === 4) && (u = i.stateNode.containerInfo, u === l || u.nodeType === 8 && u.parentNode === l)) return;
        i = i.return;
      }
      for (; s !== null; ) {
        if (i = Ft(s), i === null) return;
        if (u = i.tag, u === 5 || u === 6) {
          r = o = i;
          continue e;
        }
        s = s.parentNode;
      }
    }
    r = r.return;
  }
  Fu(function() {
    var f = o, m = pi(n), y = [];
    e: {
      var h = ma.get(e);
      if (h !== void 0) {
        var g = xi, x = e;
        switch (e) {
          case "keypress":
            if (Vr(n) === 0) break e;
          case "keydown":
          case "keyup":
            g = Od;
            break;
          case "focusin":
            x = "focus", g = Kl;
            break;
          case "focusout":
            x = "blur", g = Kl;
            break;
          case "beforeblur":
          case "afterblur":
            g = Kl;
            break;
          case "click":
            if (n.button === 2) break e;
          case "auxclick":
          case "dblclick":
          case "mousedown":
          case "mousemove":
          case "mouseup":
          case "mouseout":
          case "mouseover":
          case "contextmenu":
            g = ms;
            break;
          case "drag":
          case "dragend":
          case "dragenter":
          case "dragexit":
          case "dragleave":
          case "dragover":
          case "dragstart":
          case "drop":
            g = Sd;
            break;
          case "touchcancel":
          case "touchend":
          case "touchmove":
          case "touchstart":
            g = Ad;
            break;
          case ca:
          case da:
          case fa:
            g = Ed;
            break;
          case pa:
            g = Bd;
            break;
          case "scroll":
            g = xd;
            break;
          case "wheel":
            g = Ud;
            break;
          case "copy":
          case "cut":
          case "paste":
            g = _d;
            break;
          case "gotpointercapture":
          case "lostpointercapture":
          case "pointercancel":
          case "pointerdown":
          case "pointermove":
          case "pointerout":
          case "pointerover":
          case "pointerup":
            g = ys;
        }
        var S = (t & 4) !== 0, O = !S && e === "scroll", d = S ? h !== null ? h + "Capture" : null : h;
        S = [];
        for (var c = f, p; c !== null; ) {
          p = c;
          var v = p.stateNode;
          if (p.tag === 5 && v !== null && (p = v, d !== null && (v = nr(c, d), v != null && S.push(ar(c, v, p)))), O) break;
          c = c.return;
        }
        0 < S.length && (h = new g(h, x, null, n, m), y.push({ event: h, listeners: S }));
      }
    }
    if (!(t & 7)) {
      e: {
        if (h = e === "mouseover" || e === "pointerover", g = e === "mouseout" || e === "pointerout", h && n !== ko && (x = n.relatedTarget || n.fromElement) && (Ft(x) || x[at])) break e;
        if ((g || h) && (h = m.window === m ? m : (h = m.ownerDocument) ? h.defaultView || h.parentWindow : window, g ? (x = n.relatedTarget || n.toElement, g = f, x = x ? Ft(x) : null, x !== null && (O = Zt(x), x !== O || x.tag !== 5 && x.tag !== 6) && (x = null)) : (g = null, x = f), g !== x)) {
          if (S = ms, v = "onMouseLeave", d = "onMouseEnter", c = "mouse", (e === "pointerout" || e === "pointerover") && (S = ys, v = "onPointerLeave", d = "onPointerEnter", c = "pointer"), O = g == null ? h : rn(g), p = x == null ? h : rn(x), h = new S(v, c + "leave", g, n, m), h.target = O, h.relatedTarget = p, v = null, Ft(m) === f && (S = new S(d, c + "enter", x, n, m), S.target = p, S.relatedTarget = O, v = S), O = v, g && x) t: {
            for (S = g, d = x, c = 0, p = S; p; p = Jt(p)) c++;
            for (p = 0, v = d; v; v = Jt(v)) p++;
            for (; 0 < c - p; ) S = Jt(S), c--;
            for (; 0 < p - c; ) d = Jt(d), p--;
            for (; c--; ) {
              if (S === d || d !== null && S === d.alternate) break t;
              S = Jt(S), d = Jt(d);
            }
            S = null;
          }
          else S = null;
          g !== null && zs(y, h, g, S, !1), x !== null && O !== null && zs(y, O, x, S, !0);
        }
      }
      e: {
        if (h = f ? rn(f) : window, g = h.nodeName && h.nodeName.toLowerCase(), g === "select" || g === "input" && h.type === "file") var C = Yd;
        else if (xs(h)) if (oa) C = Jd;
        else {
          C = Gd;
          var j = Xd;
        }
        else (g = h.nodeName) && g.toLowerCase() === "input" && (h.type === "checkbox" || h.type === "radio") && (C = Zd);
        if (C && (C = C(e, f))) {
          la(y, C, n, m);
          break e;
        }
        j && j(e, h, f), e === "focusout" && (j = h._wrapperState) && j.controlled && h.type === "number" && vo(h, "number", h.value);
      }
      switch (j = f ? rn(f) : window, e) {
        case "focusin":
          (xs(j) || j.contentEditable === "true") && (tn = j, Po = f, Xn = null);
          break;
        case "focusout":
          Xn = Po = tn = null;
          break;
        case "mousedown":
          No = !0;
          break;
        case "contextmenu":
        case "mouseup":
        case "dragend":
          No = !1, Es(y, n, m);
          break;
        case "selectionchange":
          if (ef) break;
        case "keydown":
        case "keyup":
          Es(y, n, m);
      }
      var T;
      if (Si) e: {
        switch (e) {
          case "compositionstart":
            var _ = "onCompositionStart";
            break e;
          case "compositionend":
            _ = "onCompositionEnd";
            break e;
          case "compositionupdate":
            _ = "onCompositionUpdate";
            break e;
        }
        _ = void 0;
      }
      else en ? na(e, n) && (_ = "onCompositionEnd") : e === "keydown" && n.keyCode === 229 && (_ = "onCompositionStart");
      _ && (ta && n.locale !== "ko" && (en || _ !== "onCompositionStart" ? _ === "onCompositionEnd" && en && (T = ea()) : (xt = m, gi = "value" in xt ? xt.value : xt.textContent, en = !0)), j = ll(f, _), 0 < j.length && (_ = new hs(_, e, null, n, m), y.push({ event: _, listeners: j }), T ? _.data = T : (T = ra(n), T !== null && (_.data = T)))), (T = $d ? Vd(e, n) : Hd(e, n)) && (f = ll(f, "onBeforeInput"), 0 < f.length && (m = new hs("onBeforeInput", "beforeinput", null, n, m), y.push({ event: m, listeners: f }), m.data = T));
    }
    ha(y, t);
  });
}
function ar(e, t, n) {
  return { instance: e, listener: t, currentTarget: n };
}
function ll(e, t) {
  for (var n = t + "Capture", r = []; e !== null; ) {
    var l = e, o = l.stateNode;
    l.tag === 5 && o !== null && (l = o, o = nr(e, n), o != null && r.unshift(ar(e, o, l)), o = nr(e, t), o != null && r.push(ar(e, o, l))), e = e.return;
  }
  return r;
}
function Jt(e) {
  if (e === null) return null;
  do
    e = e.return;
  while (e && e.tag !== 5);
  return e || null;
}
function zs(e, t, n, r, l) {
  for (var o = t._reactName, i = []; n !== null && n !== r; ) {
    var s = n, u = s.alternate, f = s.stateNode;
    if (u !== null && u === r) break;
    s.tag === 5 && f !== null && (s = f, l ? (u = nr(n, o), u != null && i.unshift(ar(n, u, s))) : l || (u = nr(n, o), u != null && i.push(ar(n, u, s)))), n = n.return;
  }
  i.length !== 0 && e.push({ event: t, listeners: i });
}
var lf = /\r\n?/g, of = /\u0000|\uFFFD/g;
function Ts(e) {
  return (typeof e == "string" ? e : "" + e).replace(lf, `
`).replace(of, "");
}
function Lr(e, t, n) {
  if (t = Ts(t), Ts(e) !== t && n) throw Error(w(425));
}
function ol() {
}
var Ro = null, Lo = null;
function Oo(e, t) {
  return e === "textarea" || e === "noscript" || typeof t.children == "string" || typeof t.children == "number" || typeof t.dangerouslySetInnerHTML == "object" && t.dangerouslySetInnerHTML !== null && t.dangerouslySetInnerHTML.__html != null;
}
var Do = typeof setTimeout == "function" ? setTimeout : void 0, sf = typeof clearTimeout == "function" ? clearTimeout : void 0, Ps = typeof Promise == "function" ? Promise : void 0, uf = typeof queueMicrotask == "function" ? queueMicrotask : typeof Ps < "u" ? function(e) {
  return Ps.resolve(null).then(e).catch(af);
} : Do;
function af(e) {
  setTimeout(function() {
    throw e;
  });
}
function bl(e, t) {
  var n = t, r = 0;
  do {
    var l = n.nextSibling;
    if (e.removeChild(n), l && l.nodeType === 8) if (n = l.data, n === "/$") {
      if (r === 0) {
        e.removeChild(l), or(t);
        return;
      }
      r--;
    } else n !== "$" && n !== "$?" && n !== "$!" || r++;
    n = l;
  } while (n);
  or(t);
}
function jt(e) {
  for (; e != null; e = e.nextSibling) {
    var t = e.nodeType;
    if (t === 1 || t === 3) break;
    if (t === 8) {
      if (t = e.data, t === "$" || t === "$!" || t === "$?") break;
      if (t === "/$") return null;
    }
  }
  return e;
}
function Ns(e) {
  e = e.previousSibling;
  for (var t = 0; e; ) {
    if (e.nodeType === 8) {
      var n = e.data;
      if (n === "$" || n === "$!" || n === "$?") {
        if (t === 0) return e;
        t--;
      } else n === "/$" && t++;
    }
    e = e.previousSibling;
  }
  return null;
}
var zn = Math.random().toString(36).slice(2), et = "__reactFiber$" + zn, cr = "__reactProps$" + zn, at = "__reactContainer$" + zn, Io = "__reactEvents$" + zn, cf = "__reactListeners$" + zn, df = "__reactHandles$" + zn;
function Ft(e) {
  var t = e[et];
  if (t) return t;
  for (var n = e.parentNode; n; ) {
    if (t = n[at] || n[et]) {
      if (n = t.alternate, t.child !== null || n !== null && n.child !== null) for (e = Ns(e); e !== null; ) {
        if (n = e[et]) return n;
        e = Ns(e);
      }
      return t;
    }
    e = n, n = e.parentNode;
  }
  return null;
}
function wr(e) {
  return e = e[et] || e[at], !e || e.tag !== 5 && e.tag !== 6 && e.tag !== 13 && e.tag !== 3 ? null : e;
}
function rn(e) {
  if (e.tag === 5 || e.tag === 6) return e.stateNode;
  throw Error(w(33));
}
function jl(e) {
  return e[cr] || null;
}
var Ao = [], ln = -1;
function Ot(e) {
  return { current: e };
}
function Q(e) {
  0 > ln || (e.current = Ao[ln], Ao[ln] = null, ln--);
}
function H(e, t) {
  ln++, Ao[ln] = e.current, e.current = t;
}
var Rt = {}, me = Ot(Rt), Ce = Ot(!1), Ht = Rt;
function gn(e, t) {
  var n = e.type.contextTypes;
  if (!n) return Rt;
  var r = e.stateNode;
  if (r && r.__reactInternalMemoizedUnmaskedChildContext === t) return r.__reactInternalMemoizedMaskedChildContext;
  var l = {}, o;
  for (o in n) l[o] = t[o];
  return r && (e = e.stateNode, e.__reactInternalMemoizedUnmaskedChildContext = t, e.__reactInternalMemoizedMaskedChildContext = l), l;
}
function Ee(e) {
  return e = e.childContextTypes, e != null;
}
function il() {
  Q(Ce), Q(me);
}
function Rs(e, t, n) {
  if (me.current !== Rt) throw Error(w(168));
  H(me, t), H(Ce, n);
}
function va(e, t, n) {
  var r = e.stateNode;
  if (t = t.childContextTypes, typeof r.getChildContext != "function") return n;
  r = r.getChildContext();
  for (var l in r) if (!(l in t)) throw Error(w(108, Xc(e) || "Unknown", l));
  return Z({}, n, r);
}
function sl(e) {
  return e = (e = e.stateNode) && e.__reactInternalMemoizedMergedChildContext || Rt, Ht = me.current, H(me, e), H(Ce, Ce.current), !0;
}
function Ls(e, t, n) {
  var r = e.stateNode;
  if (!r) throw Error(w(169));
  n ? (e = va(e, t, Ht), r.__reactInternalMemoizedMergedChildContext = e, Q(Ce), Q(me), H(me, e)) : Q(Ce), H(Ce, n);
}
var lt = null, _l = !1, eo = !1;
function ga(e) {
  lt === null ? lt = [e] : lt.push(e);
}
function ff(e) {
  _l = !0, ga(e);
}
function Dt() {
  if (!eo && lt !== null) {
    eo = !0;
    var e = 0, t = W;
    try {
      var n = lt;
      for (W = 1; e < n.length; e++) {
        var r = n[e];
        do
          r = r(!0);
        while (r !== null);
      }
      lt = null, _l = !1;
    } catch (l) {
      throw lt !== null && (lt = lt.slice(e + 1)), Vu(mi, Dt), l;
    } finally {
      W = t, eo = !1;
    }
  }
  return null;
}
var on = [], sn = 0, ul = null, al = 0, De = [], Ie = 0, Kt = null, ot = 1, it = "";
function Mt(e, t) {
  on[sn++] = al, on[sn++] = ul, ul = e, al = t;
}
function xa(e, t, n) {
  De[Ie++] = ot, De[Ie++] = it, De[Ie++] = Kt, Kt = e;
  var r = ot;
  e = it;
  var l = 32 - Ye(r) - 1;
  r &= ~(1 << l), n += 1;
  var o = 32 - Ye(t) + l;
  if (30 < o) {
    var i = l - l % 5;
    o = (r & (1 << i) - 1).toString(32), r >>= i, l -= i, ot = 1 << 32 - Ye(t) + l | n << l | r, it = o + e;
  } else ot = 1 << o | n << l | r, it = e;
}
function Ci(e) {
  e.return !== null && (Mt(e, 1), xa(e, 1, 0));
}
function Ei(e) {
  for (; e === ul; ) ul = on[--sn], on[sn] = null, al = on[--sn], on[sn] = null;
  for (; e === Kt; ) Kt = De[--Ie], De[Ie] = null, it = De[--Ie], De[Ie] = null, ot = De[--Ie], De[Ie] = null;
}
var Te = null, ze = null, Y = !1, Qe = null;
function wa(e, t) {
  var n = Ae(5, null, null, 0);
  n.elementType = "DELETED", n.stateNode = t, n.return = e, t = e.deletions, t === null ? (e.deletions = [n], e.flags |= 16) : t.push(n);
}
function Os(e, t) {
  switch (e.tag) {
    case 5:
      var n = e.type;
      return t = t.nodeType !== 1 || n.toLowerCase() !== t.nodeName.toLowerCase() ? null : t, t !== null ? (e.stateNode = t, Te = e, ze = jt(t.firstChild), !0) : !1;
    case 6:
      return t = e.pendingProps === "" || t.nodeType !== 3 ? null : t, t !== null ? (e.stateNode = t, Te = e, ze = null, !0) : !1;
    case 13:
      return t = t.nodeType !== 8 ? null : t, t !== null ? (n = Kt !== null ? { id: ot, overflow: it } : null, e.memoizedState = { dehydrated: t, treeContext: n, retryLane: 1073741824 }, n = Ae(18, null, null, 0), n.stateNode = t, n.return = e, e.child = n, Te = e, ze = null, !0) : !1;
    default:
      return !1;
  }
}
function Mo(e) {
  return (e.mode & 1) !== 0 && (e.flags & 128) === 0;
}
function Bo(e) {
  if (Y) {
    var t = ze;
    if (t) {
      var n = t;
      if (!Os(e, t)) {
        if (Mo(e)) throw Error(w(418));
        t = jt(n.nextSibling);
        var r = Te;
        t && Os(e, t) ? wa(r, n) : (e.flags = e.flags & -4097 | 2, Y = !1, Te = e);
      }
    } else {
      if (Mo(e)) throw Error(w(418));
      e.flags = e.flags & -4097 | 2, Y = !1, Te = e;
    }
  }
}
function Ds(e) {
  for (e = e.return; e !== null && e.tag !== 5 && e.tag !== 3 && e.tag !== 13; ) e = e.return;
  Te = e;
}
function Or(e) {
  if (e !== Te) return !1;
  if (!Y) return Ds(e), Y = !0, !1;
  var t;
  if ((t = e.tag !== 3) && !(t = e.tag !== 5) && (t = e.type, t = t !== "head" && t !== "body" && !Oo(e.type, e.memoizedProps)), t && (t = ze)) {
    if (Mo(e)) throw Sa(), Error(w(418));
    for (; t; ) wa(e, t), t = jt(t.nextSibling);
  }
  if (Ds(e), e.tag === 13) {
    if (e = e.memoizedState, e = e !== null ? e.dehydrated : null, !e) throw Error(w(317));
    e: {
      for (e = e.nextSibling, t = 0; e; ) {
        if (e.nodeType === 8) {
          var n = e.data;
          if (n === "/$") {
            if (t === 0) {
              ze = jt(e.nextSibling);
              break e;
            }
            t--;
          } else n !== "$" && n !== "$!" && n !== "$?" || t++;
        }
        e = e.nextSibling;
      }
      ze = null;
    }
  } else ze = Te ? jt(e.stateNode.nextSibling) : null;
  return !0;
}
function Sa() {
  for (var e = ze; e; ) e = jt(e.nextSibling);
}
function xn() {
  ze = Te = null, Y = !1;
}
function ji(e) {
  Qe === null ? Qe = [e] : Qe.push(e);
}
var pf = ft.ReactCurrentBatchConfig;
function Mn(e, t, n) {
  if (e = n.ref, e !== null && typeof e != "function" && typeof e != "object") {
    if (n._owner) {
      if (n = n._owner, n) {
        if (n.tag !== 1) throw Error(w(309));
        var r = n.stateNode;
      }
      if (!r) throw Error(w(147, e));
      var l = r, o = "" + e;
      return t !== null && t.ref !== null && typeof t.ref == "function" && t.ref._stringRef === o ? t.ref : (t = function(i) {
        var s = l.refs;
        i === null ? delete s[o] : s[o] = i;
      }, t._stringRef = o, t);
    }
    if (typeof e != "string") throw Error(w(284));
    if (!n._owner) throw Error(w(290, e));
  }
  return e;
}
function Dr(e, t) {
  throw e = Object.prototype.toString.call(t), Error(w(31, e === "[object Object]" ? "object with keys {" + Object.keys(t).join(", ") + "}" : e));
}
function Is(e) {
  var t = e._init;
  return t(e._payload);
}
function ka(e) {
  function t(d, c) {
    if (e) {
      var p = d.deletions;
      p === null ? (d.deletions = [c], d.flags |= 16) : p.push(c);
    }
  }
  function n(d, c) {
    if (!e) return null;
    for (; c !== null; ) t(d, c), c = c.sibling;
    return null;
  }
  function r(d, c) {
    for (d = /* @__PURE__ */ new Map(); c !== null; ) c.key !== null ? d.set(c.key, c) : d.set(c.index, c), c = c.sibling;
    return d;
  }
  function l(d, c) {
    return d = Pt(d, c), d.index = 0, d.sibling = null, d;
  }
  function o(d, c, p) {
    return d.index = p, e ? (p = d.alternate, p !== null ? (p = p.index, p < c ? (d.flags |= 2, c) : p) : (d.flags |= 2, c)) : (d.flags |= 1048576, c);
  }
  function i(d) {
    return e && d.alternate === null && (d.flags |= 2), d;
  }
  function s(d, c, p, v) {
    return c === null || c.tag !== 6 ? (c = so(p, d.mode, v), c.return = d, c) : (c = l(c, p), c.return = d, c);
  }
  function u(d, c, p, v) {
    var C = p.type;
    return C === bt ? m(d, c, p.props.children, v, p.key) : c !== null && (c.elementType === C || typeof C == "object" && C !== null && C.$$typeof === mt && Is(C) === c.type) ? (v = l(c, p.props), v.ref = Mn(d, c, p), v.return = d, v) : (v = Zr(p.type, p.key, p.props, null, d.mode, v), v.ref = Mn(d, c, p), v.return = d, v);
  }
  function f(d, c, p, v) {
    return c === null || c.tag !== 4 || c.stateNode.containerInfo !== p.containerInfo || c.stateNode.implementation !== p.implementation ? (c = uo(p, d.mode, v), c.return = d, c) : (c = l(c, p.children || []), c.return = d, c);
  }
  function m(d, c, p, v, C) {
    return c === null || c.tag !== 7 ? (c = Vt(p, d.mode, v, C), c.return = d, c) : (c = l(c, p), c.return = d, c);
  }
  function y(d, c, p) {
    if (typeof c == "string" && c !== "" || typeof c == "number") return c = so("" + c, d.mode, p), c.return = d, c;
    if (typeof c == "object" && c !== null) {
      switch (c.$$typeof) {
        case Cr:
          return p = Zr(c.type, c.key, c.props, null, d.mode, p), p.ref = Mn(d, null, c), p.return = d, p;
        case qt:
          return c = uo(c, d.mode, p), c.return = d, c;
        case mt:
          var v = c._init;
          return y(d, v(c._payload), p);
      }
      if (Wn(c) || Ln(c)) return c = Vt(c, d.mode, p, null), c.return = d, c;
      Dr(d, c);
    }
    return null;
  }
  function h(d, c, p, v) {
    var C = c !== null ? c.key : null;
    if (typeof p == "string" && p !== "" || typeof p == "number") return C !== null ? null : s(d, c, "" + p, v);
    if (typeof p == "object" && p !== null) {
      switch (p.$$typeof) {
        case Cr:
          return p.key === C ? u(d, c, p, v) : null;
        case qt:
          return p.key === C ? f(d, c, p, v) : null;
        case mt:
          return C = p._init, h(
            d,
            c,
            C(p._payload),
            v
          );
      }
      if (Wn(p) || Ln(p)) return C !== null ? null : m(d, c, p, v, null);
      Dr(d, p);
    }
    return null;
  }
  function g(d, c, p, v, C) {
    if (typeof v == "string" && v !== "" || typeof v == "number") return d = d.get(p) || null, s(c, d, "" + v, C);
    if (typeof v == "object" && v !== null) {
      switch (v.$$typeof) {
        case Cr:
          return d = d.get(v.key === null ? p : v.key) || null, u(c, d, v, C);
        case qt:
          return d = d.get(v.key === null ? p : v.key) || null, f(c, d, v, C);
        case mt:
          var j = v._init;
          return g(d, c, p, j(v._payload), C);
      }
      if (Wn(v) || Ln(v)) return d = d.get(p) || null, m(c, d, v, C, null);
      Dr(c, v);
    }
    return null;
  }
  function x(d, c, p, v) {
    for (var C = null, j = null, T = c, _ = c = 0, A = null; T !== null && _ < p.length; _++) {
      T.index > _ ? (A = T, T = null) : A = T.sibling;
      var N = h(d, T, p[_], v);
      if (N === null) {
        T === null && (T = A);
        break;
      }
      e && T && N.alternate === null && t(d, T), c = o(N, c, _), j === null ? C = N : j.sibling = N, j = N, T = A;
    }
    if (_ === p.length) return n(d, T), Y && Mt(d, _), C;
    if (T === null) {
      for (; _ < p.length; _++) T = y(d, p[_], v), T !== null && (c = o(T, c, _), j === null ? C = T : j.sibling = T, j = T);
      return Y && Mt(d, _), C;
    }
    for (T = r(d, T); _ < p.length; _++) A = g(T, d, _, p[_], v), A !== null && (e && A.alternate !== null && T.delete(A.key === null ? _ : A.key), c = o(A, c, _), j === null ? C = A : j.sibling = A, j = A);
    return e && T.forEach(function(ne) {
      return t(d, ne);
    }), Y && Mt(d, _), C;
  }
  function S(d, c, p, v) {
    var C = Ln(p);
    if (typeof C != "function") throw Error(w(150));
    if (p = C.call(p), p == null) throw Error(w(151));
    for (var j = C = null, T = c, _ = c = 0, A = null, N = p.next(); T !== null && !N.done; _++, N = p.next()) {
      T.index > _ ? (A = T, T = null) : A = T.sibling;
      var ne = h(d, T, N.value, v);
      if (ne === null) {
        T === null && (T = A);
        break;
      }
      e && T && ne.alternate === null && t(d, T), c = o(ne, c, _), j === null ? C = ne : j.sibling = ne, j = ne, T = A;
    }
    if (N.done) return n(
      d,
      T
    ), Y && Mt(d, _), C;
    if (T === null) {
      for (; !N.done; _++, N = p.next()) N = y(d, N.value, v), N !== null && (c = o(N, c, _), j === null ? C = N : j.sibling = N, j = N);
      return Y && Mt(d, _), C;
    }
    for (T = r(d, T); !N.done; _++, N = p.next()) N = g(T, d, _, N.value, v), N !== null && (e && N.alternate !== null && T.delete(N.key === null ? _ : N.key), c = o(N, c, _), j === null ? C = N : j.sibling = N, j = N);
    return e && T.forEach(function(ue) {
      return t(d, ue);
    }), Y && Mt(d, _), C;
  }
  function O(d, c, p, v) {
    if (typeof p == "object" && p !== null && p.type === bt && p.key === null && (p = p.props.children), typeof p == "object" && p !== null) {
      switch (p.$$typeof) {
        case Cr:
          e: {
            for (var C = p.key, j = c; j !== null; ) {
              if (j.key === C) {
                if (C = p.type, C === bt) {
                  if (j.tag === 7) {
                    n(d, j.sibling), c = l(j, p.props.children), c.return = d, d = c;
                    break e;
                  }
                } else if (j.elementType === C || typeof C == "object" && C !== null && C.$$typeof === mt && Is(C) === j.type) {
                  n(d, j.sibling), c = l(j, p.props), c.ref = Mn(d, j, p), c.return = d, d = c;
                  break e;
                }
                n(d, j);
                break;
              } else t(d, j);
              j = j.sibling;
            }
            p.type === bt ? (c = Vt(p.props.children, d.mode, v, p.key), c.return = d, d = c) : (v = Zr(p.type, p.key, p.props, null, d.mode, v), v.ref = Mn(d, c, p), v.return = d, d = v);
          }
          return i(d);
        case qt:
          e: {
            for (j = p.key; c !== null; ) {
              if (c.key === j) if (c.tag === 4 && c.stateNode.containerInfo === p.containerInfo && c.stateNode.implementation === p.implementation) {
                n(d, c.sibling), c = l(c, p.children || []), c.return = d, d = c;
                break e;
              } else {
                n(d, c);
                break;
              }
              else t(d, c);
              c = c.sibling;
            }
            c = uo(p, d.mode, v), c.return = d, d = c;
          }
          return i(d);
        case mt:
          return j = p._init, O(d, c, j(p._payload), v);
      }
      if (Wn(p)) return x(d, c, p, v);
      if (Ln(p)) return S(d, c, p, v);
      Dr(d, p);
    }
    return typeof p == "string" && p !== "" || typeof p == "number" ? (p = "" + p, c !== null && c.tag === 6 ? (n(d, c.sibling), c = l(c, p), c.return = d, d = c) : (n(d, c), c = so(p, d.mode, v), c.return = d, d = c), i(d)) : n(d, c);
  }
  return O;
}
var wn = ka(!0), Ca = ka(!1), cl = Ot(null), dl = null, un = null, _i = null;
function zi() {
  _i = un = dl = null;
}
function Ti(e) {
  var t = cl.current;
  Q(cl), e._currentValue = t;
}
function Fo(e, t, n) {
  for (; e !== null; ) {
    var r = e.alternate;
    if ((e.childLanes & t) !== t ? (e.childLanes |= t, r !== null && (r.childLanes |= t)) : r !== null && (r.childLanes & t) !== t && (r.childLanes |= t), e === n) break;
    e = e.return;
  }
}
function hn(e, t) {
  dl = e, _i = un = null, e = e.dependencies, e !== null && e.firstContext !== null && (e.lanes & t && (ke = !0), e.firstContext = null);
}
function Be(e) {
  var t = e._currentValue;
  if (_i !== e) if (e = { context: e, memoizedValue: t, next: null }, un === null) {
    if (dl === null) throw Error(w(308));
    un = e, dl.dependencies = { lanes: 0, firstContext: e };
  } else un = un.next = e;
  return t;
}
var Ut = null;
function Pi(e) {
  Ut === null ? Ut = [e] : Ut.push(e);
}
function Ea(e, t, n, r) {
  var l = t.interleaved;
  return l === null ? (n.next = n, Pi(t)) : (n.next = l.next, l.next = n), t.interleaved = n, ct(e, r);
}
function ct(e, t) {
  e.lanes |= t;
  var n = e.alternate;
  for (n !== null && (n.lanes |= t), n = e, e = e.return; e !== null; ) e.childLanes |= t, n = e.alternate, n !== null && (n.childLanes |= t), n = e, e = e.return;
  return n.tag === 3 ? n.stateNode : null;
}
var ht = !1;
function Ni(e) {
  e.updateQueue = { baseState: e.memoizedState, firstBaseUpdate: null, lastBaseUpdate: null, shared: { pending: null, interleaved: null, lanes: 0 }, effects: null };
}
function ja(e, t) {
  e = e.updateQueue, t.updateQueue === e && (t.updateQueue = { baseState: e.baseState, firstBaseUpdate: e.firstBaseUpdate, lastBaseUpdate: e.lastBaseUpdate, shared: e.shared, effects: e.effects });
}
function st(e, t) {
  return { eventTime: e, lane: t, tag: 0, payload: null, callback: null, next: null };
}
function _t(e, t, n) {
  var r = e.updateQueue;
  if (r === null) return null;
  if (r = r.shared, M & 2) {
    var l = r.pending;
    return l === null ? t.next = t : (t.next = l.next, l.next = t), r.pending = t, ct(e, n);
  }
  return l = r.interleaved, l === null ? (t.next = t, Pi(r)) : (t.next = l.next, l.next = t), r.interleaved = t, ct(e, n);
}
function Hr(e, t, n) {
  if (t = t.updateQueue, t !== null && (t = t.shared, (n & 4194240) !== 0)) {
    var r = t.lanes;
    r &= e.pendingLanes, n |= r, t.lanes = n, hi(e, n);
  }
}
function As(e, t) {
  var n = e.updateQueue, r = e.alternate;
  if (r !== null && (r = r.updateQueue, n === r)) {
    var l = null, o = null;
    if (n = n.firstBaseUpdate, n !== null) {
      do {
        var i = { eventTime: n.eventTime, lane: n.lane, tag: n.tag, payload: n.payload, callback: n.callback, next: null };
        o === null ? l = o = i : o = o.next = i, n = n.next;
      } while (n !== null);
      o === null ? l = o = t : o = o.next = t;
    } else l = o = t;
    n = { baseState: r.baseState, firstBaseUpdate: l, lastBaseUpdate: o, shared: r.shared, effects: r.effects }, e.updateQueue = n;
    return;
  }
  e = n.lastBaseUpdate, e === null ? n.firstBaseUpdate = t : e.next = t, n.lastBaseUpdate = t;
}
function fl(e, t, n, r) {
  var l = e.updateQueue;
  ht = !1;
  var o = l.firstBaseUpdate, i = l.lastBaseUpdate, s = l.shared.pending;
  if (s !== null) {
    l.shared.pending = null;
    var u = s, f = u.next;
    u.next = null, i === null ? o = f : i.next = f, i = u;
    var m = e.alternate;
    m !== null && (m = m.updateQueue, s = m.lastBaseUpdate, s !== i && (s === null ? m.firstBaseUpdate = f : s.next = f, m.lastBaseUpdate = u));
  }
  if (o !== null) {
    var y = l.baseState;
    i = 0, m = f = u = null, s = o;
    do {
      var h = s.lane, g = s.eventTime;
      if ((r & h) === h) {
        m !== null && (m = m.next = {
          eventTime: g,
          lane: 0,
          tag: s.tag,
          payload: s.payload,
          callback: s.callback,
          next: null
        });
        e: {
          var x = e, S = s;
          switch (h = t, g = n, S.tag) {
            case 1:
              if (x = S.payload, typeof x == "function") {
                y = x.call(g, y, h);
                break e;
              }
              y = x;
              break e;
            case 3:
              x.flags = x.flags & -65537 | 128;
            case 0:
              if (x = S.payload, h = typeof x == "function" ? x.call(g, y, h) : x, h == null) break e;
              y = Z({}, y, h);
              break e;
            case 2:
              ht = !0;
          }
        }
        s.callback !== null && s.lane !== 0 && (e.flags |= 64, h = l.effects, h === null ? l.effects = [s] : h.push(s));
      } else g = { eventTime: g, lane: h, tag: s.tag, payload: s.payload, callback: s.callback, next: null }, m === null ? (f = m = g, u = y) : m = m.next = g, i |= h;
      if (s = s.next, s === null) {
        if (s = l.shared.pending, s === null) break;
        h = s, s = h.next, h.next = null, l.lastBaseUpdate = h, l.shared.pending = null;
      }
    } while (!0);
    if (m === null && (u = y), l.baseState = u, l.firstBaseUpdate = f, l.lastBaseUpdate = m, t = l.shared.interleaved, t !== null) {
      l = t;
      do
        i |= l.lane, l = l.next;
      while (l !== t);
    } else o === null && (l.shared.lanes = 0);
    Yt |= i, e.lanes = i, e.memoizedState = y;
  }
}
function Ms(e, t, n) {
  if (e = t.effects, t.effects = null, e !== null) for (t = 0; t < e.length; t++) {
    var r = e[t], l = r.callback;
    if (l !== null) {
      if (r.callback = null, r = n, typeof l != "function") throw Error(w(191, l));
      l.call(r);
    }
  }
}
var Sr = {}, nt = Ot(Sr), dr = Ot(Sr), fr = Ot(Sr);
function Wt(e) {
  if (e === Sr) throw Error(w(174));
  return e;
}
function Ri(e, t) {
  switch (H(fr, t), H(dr, e), H(nt, Sr), e = t.nodeType, e) {
    case 9:
    case 11:
      t = (t = t.documentElement) ? t.namespaceURI : xo(null, "");
      break;
    default:
      e = e === 8 ? t.parentNode : t, t = e.namespaceURI || null, e = e.tagName, t = xo(t, e);
  }
  Q(nt), H(nt, t);
}
function Sn() {
  Q(nt), Q(dr), Q(fr);
}
function _a(e) {
  Wt(fr.current);
  var t = Wt(nt.current), n = xo(t, e.type);
  t !== n && (H(dr, e), H(nt, n));
}
function Li(e) {
  dr.current === e && (Q(nt), Q(dr));
}
var X = Ot(0);
function pl(e) {
  for (var t = e; t !== null; ) {
    if (t.tag === 13) {
      var n = t.memoizedState;
      if (n !== null && (n = n.dehydrated, n === null || n.data === "$?" || n.data === "$!")) return t;
    } else if (t.tag === 19 && t.memoizedProps.revealOrder !== void 0) {
      if (t.flags & 128) return t;
    } else if (t.child !== null) {
      t.child.return = t, t = t.child;
      continue;
    }
    if (t === e) break;
    for (; t.sibling === null; ) {
      if (t.return === null || t.return === e) return null;
      t = t.return;
    }
    t.sibling.return = t.return, t = t.sibling;
  }
  return null;
}
var to = [];
function Oi() {
  for (var e = 0; e < to.length; e++) to[e]._workInProgressVersionPrimary = null;
  to.length = 0;
}
var Kr = ft.ReactCurrentDispatcher, no = ft.ReactCurrentBatchConfig, Qt = 0, G = null, ee = null, re = null, ml = !1, Gn = !1, pr = 0, mf = 0;
function de() {
  throw Error(w(321));
}
function Di(e, t) {
  if (t === null) return !1;
  for (var n = 0; n < t.length && n < e.length; n++) if (!Ge(e[n], t[n])) return !1;
  return !0;
}
function Ii(e, t, n, r, l, o) {
  if (Qt = o, G = t, t.memoizedState = null, t.updateQueue = null, t.lanes = 0, Kr.current = e === null || e.memoizedState === null ? gf : xf, e = n(r, l), Gn) {
    o = 0;
    do {
      if (Gn = !1, pr = 0, 25 <= o) throw Error(w(301));
      o += 1, re = ee = null, t.updateQueue = null, Kr.current = wf, e = n(r, l);
    } while (Gn);
  }
  if (Kr.current = hl, t = ee !== null && ee.next !== null, Qt = 0, re = ee = G = null, ml = !1, t) throw Error(w(300));
  return e;
}
function Ai() {
  var e = pr !== 0;
  return pr = 0, e;
}
function be() {
  var e = { memoizedState: null, baseState: null, baseQueue: null, queue: null, next: null };
  return re === null ? G.memoizedState = re = e : re = re.next = e, re;
}
function Fe() {
  if (ee === null) {
    var e = G.alternate;
    e = e !== null ? e.memoizedState : null;
  } else e = ee.next;
  var t = re === null ? G.memoizedState : re.next;
  if (t !== null) re = t, ee = e;
  else {
    if (e === null) throw Error(w(310));
    ee = e, e = { memoizedState: ee.memoizedState, baseState: ee.baseState, baseQueue: ee.baseQueue, queue: ee.queue, next: null }, re === null ? G.memoizedState = re = e : re = re.next = e;
  }
  return re;
}
function mr(e, t) {
  return typeof t == "function" ? t(e) : t;
}
function ro(e) {
  var t = Fe(), n = t.queue;
  if (n === null) throw Error(w(311));
  n.lastRenderedReducer = e;
  var r = ee, l = r.baseQueue, o = n.pending;
  if (o !== null) {
    if (l !== null) {
      var i = l.next;
      l.next = o.next, o.next = i;
    }
    r.baseQueue = l = o, n.pending = null;
  }
  if (l !== null) {
    o = l.next, r = r.baseState;
    var s = i = null, u = null, f = o;
    do {
      var m = f.lane;
      if ((Qt & m) === m) u !== null && (u = u.next = { lane: 0, action: f.action, hasEagerState: f.hasEagerState, eagerState: f.eagerState, next: null }), r = f.hasEagerState ? f.eagerState : e(r, f.action);
      else {
        var y = {
          lane: m,
          action: f.action,
          hasEagerState: f.hasEagerState,
          eagerState: f.eagerState,
          next: null
        };
        u === null ? (s = u = y, i = r) : u = u.next = y, G.lanes |= m, Yt |= m;
      }
      f = f.next;
    } while (f !== null && f !== o);
    u === null ? i = r : u.next = s, Ge(r, t.memoizedState) || (ke = !0), t.memoizedState = r, t.baseState = i, t.baseQueue = u, n.lastRenderedState = r;
  }
  if (e = n.interleaved, e !== null) {
    l = e;
    do
      o = l.lane, G.lanes |= o, Yt |= o, l = l.next;
    while (l !== e);
  } else l === null && (n.lanes = 0);
  return [t.memoizedState, n.dispatch];
}
function lo(e) {
  var t = Fe(), n = t.queue;
  if (n === null) throw Error(w(311));
  n.lastRenderedReducer = e;
  var r = n.dispatch, l = n.pending, o = t.memoizedState;
  if (l !== null) {
    n.pending = null;
    var i = l = l.next;
    do
      o = e(o, i.action), i = i.next;
    while (i !== l);
    Ge(o, t.memoizedState) || (ke = !0), t.memoizedState = o, t.baseQueue === null && (t.baseState = o), n.lastRenderedState = o;
  }
  return [o, r];
}
function za() {
}
function Ta(e, t) {
  var n = G, r = Fe(), l = t(), o = !Ge(r.memoizedState, l);
  if (o && (r.memoizedState = l, ke = !0), r = r.queue, Mi(Ra.bind(null, n, r, e), [e]), r.getSnapshot !== t || o || re !== null && re.memoizedState.tag & 1) {
    if (n.flags |= 2048, hr(9, Na.bind(null, n, r, l, t), void 0, null), le === null) throw Error(w(349));
    Qt & 30 || Pa(n, t, l);
  }
  return l;
}
function Pa(e, t, n) {
  e.flags |= 16384, e = { getSnapshot: t, value: n }, t = G.updateQueue, t === null ? (t = { lastEffect: null, stores: null }, G.updateQueue = t, t.stores = [e]) : (n = t.stores, n === null ? t.stores = [e] : n.push(e));
}
function Na(e, t, n, r) {
  t.value = n, t.getSnapshot = r, La(t) && Oa(e);
}
function Ra(e, t, n) {
  return n(function() {
    La(t) && Oa(e);
  });
}
function La(e) {
  var t = e.getSnapshot;
  e = e.value;
  try {
    var n = t();
    return !Ge(e, n);
  } catch {
    return !0;
  }
}
function Oa(e) {
  var t = ct(e, 1);
  t !== null && Xe(t, e, 1, -1);
}
function Bs(e) {
  var t = be();
  return typeof e == "function" && (e = e()), t.memoizedState = t.baseState = e, e = { pending: null, interleaved: null, lanes: 0, dispatch: null, lastRenderedReducer: mr, lastRenderedState: e }, t.queue = e, e = e.dispatch = vf.bind(null, G, e), [t.memoizedState, e];
}
function hr(e, t, n, r) {
  return e = { tag: e, create: t, destroy: n, deps: r, next: null }, t = G.updateQueue, t === null ? (t = { lastEffect: null, stores: null }, G.updateQueue = t, t.lastEffect = e.next = e) : (n = t.lastEffect, n === null ? t.lastEffect = e.next = e : (r = n.next, n.next = e, e.next = r, t.lastEffect = e)), e;
}
function Da() {
  return Fe().memoizedState;
}
function Qr(e, t, n, r) {
  var l = be();
  G.flags |= e, l.memoizedState = hr(1 | t, n, void 0, r === void 0 ? null : r);
}
function zl(e, t, n, r) {
  var l = Fe();
  r = r === void 0 ? null : r;
  var o = void 0;
  if (ee !== null) {
    var i = ee.memoizedState;
    if (o = i.destroy, r !== null && Di(r, i.deps)) {
      l.memoizedState = hr(t, n, o, r);
      return;
    }
  }
  G.flags |= e, l.memoizedState = hr(1 | t, n, o, r);
}
function Fs(e, t) {
  return Qr(8390656, 8, e, t);
}
function Mi(e, t) {
  return zl(2048, 8, e, t);
}
function Ia(e, t) {
  return zl(4, 2, e, t);
}
function Aa(e, t) {
  return zl(4, 4, e, t);
}
function Ma(e, t) {
  if (typeof t == "function") return e = e(), t(e), function() {
    t(null);
  };
  if (t != null) return e = e(), t.current = e, function() {
    t.current = null;
  };
}
function Ba(e, t, n) {
  return n = n != null ? n.concat([e]) : null, zl(4, 4, Ma.bind(null, t, e), n);
}
function Bi() {
}
function Fa(e, t) {
  var n = Fe();
  t = t === void 0 ? null : t;
  var r = n.memoizedState;
  return r !== null && t !== null && Di(t, r[1]) ? r[0] : (n.memoizedState = [e, t], e);
}
function Ua(e, t) {
  var n = Fe();
  t = t === void 0 ? null : t;
  var r = n.memoizedState;
  return r !== null && t !== null && Di(t, r[1]) ? r[0] : (e = e(), n.memoizedState = [e, t], e);
}
function Wa(e, t, n) {
  return Qt & 21 ? (Ge(n, t) || (n = Qu(), G.lanes |= n, Yt |= n, e.baseState = !0), t) : (e.baseState && (e.baseState = !1, ke = !0), e.memoizedState = n);
}
function hf(e, t) {
  var n = W;
  W = n !== 0 && 4 > n ? n : 4, e(!0);
  var r = no.transition;
  no.transition = {};
  try {
    e(!1), t();
  } finally {
    W = n, no.transition = r;
  }
}
function $a() {
  return Fe().memoizedState;
}
function yf(e, t, n) {
  var r = Tt(e);
  if (n = { lane: r, action: n, hasEagerState: !1, eagerState: null, next: null }, Va(e)) Ha(t, n);
  else if (n = Ea(e, t, n, r), n !== null) {
    var l = ye();
    Xe(n, e, r, l), Ka(n, t, r);
  }
}
function vf(e, t, n) {
  var r = Tt(e), l = { lane: r, action: n, hasEagerState: !1, eagerState: null, next: null };
  if (Va(e)) Ha(t, l);
  else {
    var o = e.alternate;
    if (e.lanes === 0 && (o === null || o.lanes === 0) && (o = t.lastRenderedReducer, o !== null)) try {
      var i = t.lastRenderedState, s = o(i, n);
      if (l.hasEagerState = !0, l.eagerState = s, Ge(s, i)) {
        var u = t.interleaved;
        u === null ? (l.next = l, Pi(t)) : (l.next = u.next, u.next = l), t.interleaved = l;
        return;
      }
    } catch {
    } finally {
    }
    n = Ea(e, t, l, r), n !== null && (l = ye(), Xe(n, e, r, l), Ka(n, t, r));
  }
}
function Va(e) {
  var t = e.alternate;
  return e === G || t !== null && t === G;
}
function Ha(e, t) {
  Gn = ml = !0;
  var n = e.pending;
  n === null ? t.next = t : (t.next = n.next, n.next = t), e.pending = t;
}
function Ka(e, t, n) {
  if (n & 4194240) {
    var r = t.lanes;
    r &= e.pendingLanes, n |= r, t.lanes = n, hi(e, n);
  }
}
var hl = { readContext: Be, useCallback: de, useContext: de, useEffect: de, useImperativeHandle: de, useInsertionEffect: de, useLayoutEffect: de, useMemo: de, useReducer: de, useRef: de, useState: de, useDebugValue: de, useDeferredValue: de, useTransition: de, useMutableSource: de, useSyncExternalStore: de, useId: de, unstable_isNewReconciler: !1 }, gf = { readContext: Be, useCallback: function(e, t) {
  return be().memoizedState = [e, t === void 0 ? null : t], e;
}, useContext: Be, useEffect: Fs, useImperativeHandle: function(e, t, n) {
  return n = n != null ? n.concat([e]) : null, Qr(
    4194308,
    4,
    Ma.bind(null, t, e),
    n
  );
}, useLayoutEffect: function(e, t) {
  return Qr(4194308, 4, e, t);
}, useInsertionEffect: function(e, t) {
  return Qr(4, 2, e, t);
}, useMemo: function(e, t) {
  var n = be();
  return t = t === void 0 ? null : t, e = e(), n.memoizedState = [e, t], e;
}, useReducer: function(e, t, n) {
  var r = be();
  return t = n !== void 0 ? n(t) : t, r.memoizedState = r.baseState = t, e = { pending: null, interleaved: null, lanes: 0, dispatch: null, lastRenderedReducer: e, lastRenderedState: t }, r.queue = e, e = e.dispatch = yf.bind(null, G, e), [r.memoizedState, e];
}, useRef: function(e) {
  var t = be();
  return e = { current: e }, t.memoizedState = e;
}, useState: Bs, useDebugValue: Bi, useDeferredValue: function(e) {
  return be().memoizedState = e;
}, useTransition: function() {
  var e = Bs(!1), t = e[0];
  return e = hf.bind(null, e[1]), be().memoizedState = e, [t, e];
}, useMutableSource: function() {
}, useSyncExternalStore: function(e, t, n) {
  var r = G, l = be();
  if (Y) {
    if (n === void 0) throw Error(w(407));
    n = n();
  } else {
    if (n = t(), le === null) throw Error(w(349));
    Qt & 30 || Pa(r, t, n);
  }
  l.memoizedState = n;
  var o = { value: n, getSnapshot: t };
  return l.queue = o, Fs(Ra.bind(
    null,
    r,
    o,
    e
  ), [e]), r.flags |= 2048, hr(9, Na.bind(null, r, o, n, t), void 0, null), n;
}, useId: function() {
  var e = be(), t = le.identifierPrefix;
  if (Y) {
    var n = it, r = ot;
    n = (r & ~(1 << 32 - Ye(r) - 1)).toString(32) + n, t = ":" + t + "R" + n, n = pr++, 0 < n && (t += "H" + n.toString(32)), t += ":";
  } else n = mf++, t = ":" + t + "r" + n.toString(32) + ":";
  return e.memoizedState = t;
}, unstable_isNewReconciler: !1 }, xf = {
  readContext: Be,
  useCallback: Fa,
  useContext: Be,
  useEffect: Mi,
  useImperativeHandle: Ba,
  useInsertionEffect: Ia,
  useLayoutEffect: Aa,
  useMemo: Ua,
  useReducer: ro,
  useRef: Da,
  useState: function() {
    return ro(mr);
  },
  useDebugValue: Bi,
  useDeferredValue: function(e) {
    var t = Fe();
    return Wa(t, ee.memoizedState, e);
  },
  useTransition: function() {
    var e = ro(mr)[0], t = Fe().memoizedState;
    return [e, t];
  },
  useMutableSource: za,
  useSyncExternalStore: Ta,
  useId: $a,
  unstable_isNewReconciler: !1
}, wf = { readContext: Be, useCallback: Fa, useContext: Be, useEffect: Mi, useImperativeHandle: Ba, useInsertionEffect: Ia, useLayoutEffect: Aa, useMemo: Ua, useReducer: lo, useRef: Da, useState: function() {
  return lo(mr);
}, useDebugValue: Bi, useDeferredValue: function(e) {
  var t = Fe();
  return ee === null ? t.memoizedState = e : Wa(t, ee.memoizedState, e);
}, useTransition: function() {
  var e = lo(mr)[0], t = Fe().memoizedState;
  return [e, t];
}, useMutableSource: za, useSyncExternalStore: Ta, useId: $a, unstable_isNewReconciler: !1 };
function He(e, t) {
  if (e && e.defaultProps) {
    t = Z({}, t), e = e.defaultProps;
    for (var n in e) t[n] === void 0 && (t[n] = e[n]);
    return t;
  }
  return t;
}
function Uo(e, t, n, r) {
  t = e.memoizedState, n = n(r, t), n = n == null ? t : Z({}, t, n), e.memoizedState = n, e.lanes === 0 && (e.updateQueue.baseState = n);
}
var Tl = { isMounted: function(e) {
  return (e = e._reactInternals) ? Zt(e) === e : !1;
}, enqueueSetState: function(e, t, n) {
  e = e._reactInternals;
  var r = ye(), l = Tt(e), o = st(r, l);
  o.payload = t, n != null && (o.callback = n), t = _t(e, o, l), t !== null && (Xe(t, e, l, r), Hr(t, e, l));
}, enqueueReplaceState: function(e, t, n) {
  e = e._reactInternals;
  var r = ye(), l = Tt(e), o = st(r, l);
  o.tag = 1, o.payload = t, n != null && (o.callback = n), t = _t(e, o, l), t !== null && (Xe(t, e, l, r), Hr(t, e, l));
}, enqueueForceUpdate: function(e, t) {
  e = e._reactInternals;
  var n = ye(), r = Tt(e), l = st(n, r);
  l.tag = 2, t != null && (l.callback = t), t = _t(e, l, r), t !== null && (Xe(t, e, r, n), Hr(t, e, r));
} };
function Us(e, t, n, r, l, o, i) {
  return e = e.stateNode, typeof e.shouldComponentUpdate == "function" ? e.shouldComponentUpdate(r, o, i) : t.prototype && t.prototype.isPureReactComponent ? !sr(n, r) || !sr(l, o) : !0;
}
function Qa(e, t, n) {
  var r = !1, l = Rt, o = t.contextType;
  return typeof o == "object" && o !== null ? o = Be(o) : (l = Ee(t) ? Ht : me.current, r = t.contextTypes, o = (r = r != null) ? gn(e, l) : Rt), t = new t(n, o), e.memoizedState = t.state !== null && t.state !== void 0 ? t.state : null, t.updater = Tl, e.stateNode = t, t._reactInternals = e, r && (e = e.stateNode, e.__reactInternalMemoizedUnmaskedChildContext = l, e.__reactInternalMemoizedMaskedChildContext = o), t;
}
function Ws(e, t, n, r) {
  e = t.state, typeof t.componentWillReceiveProps == "function" && t.componentWillReceiveProps(n, r), typeof t.UNSAFE_componentWillReceiveProps == "function" && t.UNSAFE_componentWillReceiveProps(n, r), t.state !== e && Tl.enqueueReplaceState(t, t.state, null);
}
function Wo(e, t, n, r) {
  var l = e.stateNode;
  l.props = n, l.state = e.memoizedState, l.refs = {}, Ni(e);
  var o = t.contextType;
  typeof o == "object" && o !== null ? l.context = Be(o) : (o = Ee(t) ? Ht : me.current, l.context = gn(e, o)), l.state = e.memoizedState, o = t.getDerivedStateFromProps, typeof o == "function" && (Uo(e, t, o, n), l.state = e.memoizedState), typeof t.getDerivedStateFromProps == "function" || typeof l.getSnapshotBeforeUpdate == "function" || typeof l.UNSAFE_componentWillMount != "function" && typeof l.componentWillMount != "function" || (t = l.state, typeof l.componentWillMount == "function" && l.componentWillMount(), typeof l.UNSAFE_componentWillMount == "function" && l.UNSAFE_componentWillMount(), t !== l.state && Tl.enqueueReplaceState(l, l.state, null), fl(e, n, l, r), l.state = e.memoizedState), typeof l.componentDidMount == "function" && (e.flags |= 4194308);
}
function kn(e, t) {
  try {
    var n = "", r = t;
    do
      n += Yc(r), r = r.return;
    while (r);
    var l = n;
  } catch (o) {
    l = `
Error generating stack: ` + o.message + `
` + o.stack;
  }
  return { value: e, source: t, stack: l, digest: null };
}
function oo(e, t, n) {
  return { value: e, source: null, stack: n ?? null, digest: t ?? null };
}
function $o(e, t) {
  try {
    console.error(t.value);
  } catch (n) {
    setTimeout(function() {
      throw n;
    });
  }
}
var Sf = typeof WeakMap == "function" ? WeakMap : Map;
function Ya(e, t, n) {
  n = st(-1, n), n.tag = 3, n.payload = { element: null };
  var r = t.value;
  return n.callback = function() {
    vl || (vl = !0, qo = r), $o(e, t);
  }, n;
}
function Xa(e, t, n) {
  n = st(-1, n), n.tag = 3;
  var r = e.type.getDerivedStateFromError;
  if (typeof r == "function") {
    var l = t.value;
    n.payload = function() {
      return r(l);
    }, n.callback = function() {
      $o(e, t);
    };
  }
  var o = e.stateNode;
  return o !== null && typeof o.componentDidCatch == "function" && (n.callback = function() {
    $o(e, t), typeof r != "function" && (zt === null ? zt = /* @__PURE__ */ new Set([this]) : zt.add(this));
    var i = t.stack;
    this.componentDidCatch(t.value, { componentStack: i !== null ? i : "" });
  }), n;
}
function $s(e, t, n) {
  var r = e.pingCache;
  if (r === null) {
    r = e.pingCache = new Sf();
    var l = /* @__PURE__ */ new Set();
    r.set(t, l);
  } else l = r.get(t), l === void 0 && (l = /* @__PURE__ */ new Set(), r.set(t, l));
  l.has(n) || (l.add(n), e = If.bind(null, e, t, n), t.then(e, e));
}
function Vs(e) {
  do {
    var t;
    if ((t = e.tag === 13) && (t = e.memoizedState, t = t !== null ? t.dehydrated !== null : !0), t) return e;
    e = e.return;
  } while (e !== null);
  return null;
}
function Hs(e, t, n, r, l) {
  return e.mode & 1 ? (e.flags |= 65536, e.lanes = l, e) : (e === t ? e.flags |= 65536 : (e.flags |= 128, n.flags |= 131072, n.flags &= -52805, n.tag === 1 && (n.alternate === null ? n.tag = 17 : (t = st(-1, 1), t.tag = 2, _t(n, t, 1))), n.lanes |= 1), e);
}
var kf = ft.ReactCurrentOwner, ke = !1;
function he(e, t, n, r) {
  t.child = e === null ? Ca(t, null, n, r) : wn(t, e.child, n, r);
}
function Ks(e, t, n, r, l) {
  n = n.render;
  var o = t.ref;
  return hn(t, l), r = Ii(e, t, n, r, o, l), n = Ai(), e !== null && !ke ? (t.updateQueue = e.updateQueue, t.flags &= -2053, e.lanes &= ~l, dt(e, t, l)) : (Y && n && Ci(t), t.flags |= 1, he(e, t, r, l), t.child);
}
function Qs(e, t, n, r, l) {
  if (e === null) {
    var o = n.type;
    return typeof o == "function" && !Qi(o) && o.defaultProps === void 0 && n.compare === null && n.defaultProps === void 0 ? (t.tag = 15, t.type = o, Ga(e, t, o, r, l)) : (e = Zr(n.type, null, r, t, t.mode, l), e.ref = t.ref, e.return = t, t.child = e);
  }
  if (o = e.child, !(e.lanes & l)) {
    var i = o.memoizedProps;
    if (n = n.compare, n = n !== null ? n : sr, n(i, r) && e.ref === t.ref) return dt(e, t, l);
  }
  return t.flags |= 1, e = Pt(o, r), e.ref = t.ref, e.return = t, t.child = e;
}
function Ga(e, t, n, r, l) {
  if (e !== null) {
    var o = e.memoizedProps;
    if (sr(o, r) && e.ref === t.ref) if (ke = !1, t.pendingProps = r = o, (e.lanes & l) !== 0) e.flags & 131072 && (ke = !0);
    else return t.lanes = e.lanes, dt(e, t, l);
  }
  return Vo(e, t, n, r, l);
}
function Za(e, t, n) {
  var r = t.pendingProps, l = r.children, o = e !== null ? e.memoizedState : null;
  if (r.mode === "hidden") if (!(t.mode & 1)) t.memoizedState = { baseLanes: 0, cachePool: null, transitions: null }, H(cn, _e), _e |= n;
  else {
    if (!(n & 1073741824)) return e = o !== null ? o.baseLanes | n : n, t.lanes = t.childLanes = 1073741824, t.memoizedState = { baseLanes: e, cachePool: null, transitions: null }, t.updateQueue = null, H(cn, _e), _e |= e, null;
    t.memoizedState = { baseLanes: 0, cachePool: null, transitions: null }, r = o !== null ? o.baseLanes : n, H(cn, _e), _e |= r;
  }
  else o !== null ? (r = o.baseLanes | n, t.memoizedState = null) : r = n, H(cn, _e), _e |= r;
  return he(e, t, l, n), t.child;
}
function Ja(e, t) {
  var n = t.ref;
  (e === null && n !== null || e !== null && e.ref !== n) && (t.flags |= 512, t.flags |= 2097152);
}
function Vo(e, t, n, r, l) {
  var o = Ee(n) ? Ht : me.current;
  return o = gn(t, o), hn(t, l), n = Ii(e, t, n, r, o, l), r = Ai(), e !== null && !ke ? (t.updateQueue = e.updateQueue, t.flags &= -2053, e.lanes &= ~l, dt(e, t, l)) : (Y && r && Ci(t), t.flags |= 1, he(e, t, n, l), t.child);
}
function Ys(e, t, n, r, l) {
  if (Ee(n)) {
    var o = !0;
    sl(t);
  } else o = !1;
  if (hn(t, l), t.stateNode === null) Yr(e, t), Qa(t, n, r), Wo(t, n, r, l), r = !0;
  else if (e === null) {
    var i = t.stateNode, s = t.memoizedProps;
    i.props = s;
    var u = i.context, f = n.contextType;
    typeof f == "object" && f !== null ? f = Be(f) : (f = Ee(n) ? Ht : me.current, f = gn(t, f));
    var m = n.getDerivedStateFromProps, y = typeof m == "function" || typeof i.getSnapshotBeforeUpdate == "function";
    y || typeof i.UNSAFE_componentWillReceiveProps != "function" && typeof i.componentWillReceiveProps != "function" || (s !== r || u !== f) && Ws(t, i, r, f), ht = !1;
    var h = t.memoizedState;
    i.state = h, fl(t, r, i, l), u = t.memoizedState, s !== r || h !== u || Ce.current || ht ? (typeof m == "function" && (Uo(t, n, m, r), u = t.memoizedState), (s = ht || Us(t, n, s, r, h, u, f)) ? (y || typeof i.UNSAFE_componentWillMount != "function" && typeof i.componentWillMount != "function" || (typeof i.componentWillMount == "function" && i.componentWillMount(), typeof i.UNSAFE_componentWillMount == "function" && i.UNSAFE_componentWillMount()), typeof i.componentDidMount == "function" && (t.flags |= 4194308)) : (typeof i.componentDidMount == "function" && (t.flags |= 4194308), t.memoizedProps = r, t.memoizedState = u), i.props = r, i.state = u, i.context = f, r = s) : (typeof i.componentDidMount == "function" && (t.flags |= 4194308), r = !1);
  } else {
    i = t.stateNode, ja(e, t), s = t.memoizedProps, f = t.type === t.elementType ? s : He(t.type, s), i.props = f, y = t.pendingProps, h = i.context, u = n.contextType, typeof u == "object" && u !== null ? u = Be(u) : (u = Ee(n) ? Ht : me.current, u = gn(t, u));
    var g = n.getDerivedStateFromProps;
    (m = typeof g == "function" || typeof i.getSnapshotBeforeUpdate == "function") || typeof i.UNSAFE_componentWillReceiveProps != "function" && typeof i.componentWillReceiveProps != "function" || (s !== y || h !== u) && Ws(t, i, r, u), ht = !1, h = t.memoizedState, i.state = h, fl(t, r, i, l);
    var x = t.memoizedState;
    s !== y || h !== x || Ce.current || ht ? (typeof g == "function" && (Uo(t, n, g, r), x = t.memoizedState), (f = ht || Us(t, n, f, r, h, x, u) || !1) ? (m || typeof i.UNSAFE_componentWillUpdate != "function" && typeof i.componentWillUpdate != "function" || (typeof i.componentWillUpdate == "function" && i.componentWillUpdate(r, x, u), typeof i.UNSAFE_componentWillUpdate == "function" && i.UNSAFE_componentWillUpdate(r, x, u)), typeof i.componentDidUpdate == "function" && (t.flags |= 4), typeof i.getSnapshotBeforeUpdate == "function" && (t.flags |= 1024)) : (typeof i.componentDidUpdate != "function" || s === e.memoizedProps && h === e.memoizedState || (t.flags |= 4), typeof i.getSnapshotBeforeUpdate != "function" || s === e.memoizedProps && h === e.memoizedState || (t.flags |= 1024), t.memoizedProps = r, t.memoizedState = x), i.props = r, i.state = x, i.context = u, r = f) : (typeof i.componentDidUpdate != "function" || s === e.memoizedProps && h === e.memoizedState || (t.flags |= 4), typeof i.getSnapshotBeforeUpdate != "function" || s === e.memoizedProps && h === e.memoizedState || (t.flags |= 1024), r = !1);
  }
  return Ho(e, t, n, r, o, l);
}
function Ho(e, t, n, r, l, o) {
  Ja(e, t);
  var i = (t.flags & 128) !== 0;
  if (!r && !i) return l && Ls(t, n, !1), dt(e, t, o);
  r = t.stateNode, kf.current = t;
  var s = i && typeof n.getDerivedStateFromError != "function" ? null : r.render();
  return t.flags |= 1, e !== null && i ? (t.child = wn(t, e.child, null, o), t.child = wn(t, null, s, o)) : he(e, t, s, o), t.memoizedState = r.state, l && Ls(t, n, !0), t.child;
}
function qa(e) {
  var t = e.stateNode;
  t.pendingContext ? Rs(e, t.pendingContext, t.pendingContext !== t.context) : t.context && Rs(e, t.context, !1), Ri(e, t.containerInfo);
}
function Xs(e, t, n, r, l) {
  return xn(), ji(l), t.flags |= 256, he(e, t, n, r), t.child;
}
var Ko = { dehydrated: null, treeContext: null, retryLane: 0 };
function Qo(e) {
  return { baseLanes: e, cachePool: null, transitions: null };
}
function ba(e, t, n) {
  var r = t.pendingProps, l = X.current, o = !1, i = (t.flags & 128) !== 0, s;
  if ((s = i) || (s = e !== null && e.memoizedState === null ? !1 : (l & 2) !== 0), s ? (o = !0, t.flags &= -129) : (e === null || e.memoizedState !== null) && (l |= 1), H(X, l & 1), e === null)
    return Bo(t), e = t.memoizedState, e !== null && (e = e.dehydrated, e !== null) ? (t.mode & 1 ? e.data === "$!" ? t.lanes = 8 : t.lanes = 1073741824 : t.lanes = 1, null) : (i = r.children, e = r.fallback, o ? (r = t.mode, o = t.child, i = { mode: "hidden", children: i }, !(r & 1) && o !== null ? (o.childLanes = 0, o.pendingProps = i) : o = Rl(i, r, 0, null), e = Vt(e, r, n, null), o.return = t, e.return = t, o.sibling = e, t.child = o, t.child.memoizedState = Qo(n), t.memoizedState = Ko, e) : Fi(t, i));
  if (l = e.memoizedState, l !== null && (s = l.dehydrated, s !== null)) return Cf(e, t, i, r, s, l, n);
  if (o) {
    o = r.fallback, i = t.mode, l = e.child, s = l.sibling;
    var u = { mode: "hidden", children: r.children };
    return !(i & 1) && t.child !== l ? (r = t.child, r.childLanes = 0, r.pendingProps = u, t.deletions = null) : (r = Pt(l, u), r.subtreeFlags = l.subtreeFlags & 14680064), s !== null ? o = Pt(s, o) : (o = Vt(o, i, n, null), o.flags |= 2), o.return = t, r.return = t, r.sibling = o, t.child = r, r = o, o = t.child, i = e.child.memoizedState, i = i === null ? Qo(n) : { baseLanes: i.baseLanes | n, cachePool: null, transitions: i.transitions }, o.memoizedState = i, o.childLanes = e.childLanes & ~n, t.memoizedState = Ko, r;
  }
  return o = e.child, e = o.sibling, r = Pt(o, { mode: "visible", children: r.children }), !(t.mode & 1) && (r.lanes = n), r.return = t, r.sibling = null, e !== null && (n = t.deletions, n === null ? (t.deletions = [e], t.flags |= 16) : n.push(e)), t.child = r, t.memoizedState = null, r;
}
function Fi(e, t) {
  return t = Rl({ mode: "visible", children: t }, e.mode, 0, null), t.return = e, e.child = t;
}
function Ir(e, t, n, r) {
  return r !== null && ji(r), wn(t, e.child, null, n), e = Fi(t, t.pendingProps.children), e.flags |= 2, t.memoizedState = null, e;
}
function Cf(e, t, n, r, l, o, i) {
  if (n)
    return t.flags & 256 ? (t.flags &= -257, r = oo(Error(w(422))), Ir(e, t, i, r)) : t.memoizedState !== null ? (t.child = e.child, t.flags |= 128, null) : (o = r.fallback, l = t.mode, r = Rl({ mode: "visible", children: r.children }, l, 0, null), o = Vt(o, l, i, null), o.flags |= 2, r.return = t, o.return = t, r.sibling = o, t.child = r, t.mode & 1 && wn(t, e.child, null, i), t.child.memoizedState = Qo(i), t.memoizedState = Ko, o);
  if (!(t.mode & 1)) return Ir(e, t, i, null);
  if (l.data === "$!") {
    if (r = l.nextSibling && l.nextSibling.dataset, r) var s = r.dgst;
    return r = s, o = Error(w(419)), r = oo(o, r, void 0), Ir(e, t, i, r);
  }
  if (s = (i & e.childLanes) !== 0, ke || s) {
    if (r = le, r !== null) {
      switch (i & -i) {
        case 4:
          l = 2;
          break;
        case 16:
          l = 8;
          break;
        case 64:
        case 128:
        case 256:
        case 512:
        case 1024:
        case 2048:
        case 4096:
        case 8192:
        case 16384:
        case 32768:
        case 65536:
        case 131072:
        case 262144:
        case 524288:
        case 1048576:
        case 2097152:
        case 4194304:
        case 8388608:
        case 16777216:
        case 33554432:
        case 67108864:
          l = 32;
          break;
        case 536870912:
          l = 268435456;
          break;
        default:
          l = 0;
      }
      l = l & (r.suspendedLanes | i) ? 0 : l, l !== 0 && l !== o.retryLane && (o.retryLane = l, ct(e, l), Xe(r, e, l, -1));
    }
    return Ki(), r = oo(Error(w(421))), Ir(e, t, i, r);
  }
  return l.data === "$?" ? (t.flags |= 128, t.child = e.child, t = Af.bind(null, e), l._reactRetry = t, null) : (e = o.treeContext, ze = jt(l.nextSibling), Te = t, Y = !0, Qe = null, e !== null && (De[Ie++] = ot, De[Ie++] = it, De[Ie++] = Kt, ot = e.id, it = e.overflow, Kt = t), t = Fi(t, r.children), t.flags |= 4096, t);
}
function Gs(e, t, n) {
  e.lanes |= t;
  var r = e.alternate;
  r !== null && (r.lanes |= t), Fo(e.return, t, n);
}
function io(e, t, n, r, l) {
  var o = e.memoizedState;
  o === null ? e.memoizedState = { isBackwards: t, rendering: null, renderingStartTime: 0, last: r, tail: n, tailMode: l } : (o.isBackwards = t, o.rendering = null, o.renderingStartTime = 0, o.last = r, o.tail = n, o.tailMode = l);
}
function ec(e, t, n) {
  var r = t.pendingProps, l = r.revealOrder, o = r.tail;
  if (he(e, t, r.children, n), r = X.current, r & 2) r = r & 1 | 2, t.flags |= 128;
  else {
    if (e !== null && e.flags & 128) e: for (e = t.child; e !== null; ) {
      if (e.tag === 13) e.memoizedState !== null && Gs(e, n, t);
      else if (e.tag === 19) Gs(e, n, t);
      else if (e.child !== null) {
        e.child.return = e, e = e.child;
        continue;
      }
      if (e === t) break e;
      for (; e.sibling === null; ) {
        if (e.return === null || e.return === t) break e;
        e = e.return;
      }
      e.sibling.return = e.return, e = e.sibling;
    }
    r &= 1;
  }
  if (H(X, r), !(t.mode & 1)) t.memoizedState = null;
  else switch (l) {
    case "forwards":
      for (n = t.child, l = null; n !== null; ) e = n.alternate, e !== null && pl(e) === null && (l = n), n = n.sibling;
      n = l, n === null ? (l = t.child, t.child = null) : (l = n.sibling, n.sibling = null), io(t, !1, l, n, o);
      break;
    case "backwards":
      for (n = null, l = t.child, t.child = null; l !== null; ) {
        if (e = l.alternate, e !== null && pl(e) === null) {
          t.child = l;
          break;
        }
        e = l.sibling, l.sibling = n, n = l, l = e;
      }
      io(t, !0, n, null, o);
      break;
    case "together":
      io(t, !1, null, null, void 0);
      break;
    default:
      t.memoizedState = null;
  }
  return t.child;
}
function Yr(e, t) {
  !(t.mode & 1) && e !== null && (e.alternate = null, t.alternate = null, t.flags |= 2);
}
function dt(e, t, n) {
  if (e !== null && (t.dependencies = e.dependencies), Yt |= t.lanes, !(n & t.childLanes)) return null;
  if (e !== null && t.child !== e.child) throw Error(w(153));
  if (t.child !== null) {
    for (e = t.child, n = Pt(e, e.pendingProps), t.child = n, n.return = t; e.sibling !== null; ) e = e.sibling, n = n.sibling = Pt(e, e.pendingProps), n.return = t;
    n.sibling = null;
  }
  return t.child;
}
function Ef(e, t, n) {
  switch (t.tag) {
    case 3:
      qa(t), xn();
      break;
    case 5:
      _a(t);
      break;
    case 1:
      Ee(t.type) && sl(t);
      break;
    case 4:
      Ri(t, t.stateNode.containerInfo);
      break;
    case 10:
      var r = t.type._context, l = t.memoizedProps.value;
      H(cl, r._currentValue), r._currentValue = l;
      break;
    case 13:
      if (r = t.memoizedState, r !== null)
        return r.dehydrated !== null ? (H(X, X.current & 1), t.flags |= 128, null) : n & t.child.childLanes ? ba(e, t, n) : (H(X, X.current & 1), e = dt(e, t, n), e !== null ? e.sibling : null);
      H(X, X.current & 1);
      break;
    case 19:
      if (r = (n & t.childLanes) !== 0, e.flags & 128) {
        if (r) return ec(e, t, n);
        t.flags |= 128;
      }
      if (l = t.memoizedState, l !== null && (l.rendering = null, l.tail = null, l.lastEffect = null), H(X, X.current), r) break;
      return null;
    case 22:
    case 23:
      return t.lanes = 0, Za(e, t, n);
  }
  return dt(e, t, n);
}
var tc, Yo, nc, rc;
tc = function(e, t) {
  for (var n = t.child; n !== null; ) {
    if (n.tag === 5 || n.tag === 6) e.appendChild(n.stateNode);
    else if (n.tag !== 4 && n.child !== null) {
      n.child.return = n, n = n.child;
      continue;
    }
    if (n === t) break;
    for (; n.sibling === null; ) {
      if (n.return === null || n.return === t) return;
      n = n.return;
    }
    n.sibling.return = n.return, n = n.sibling;
  }
};
Yo = function() {
};
nc = function(e, t, n, r) {
  var l = e.memoizedProps;
  if (l !== r) {
    e = t.stateNode, Wt(nt.current);
    var o = null;
    switch (n) {
      case "input":
        l = ho(e, l), r = ho(e, r), o = [];
        break;
      case "select":
        l = Z({}, l, { value: void 0 }), r = Z({}, r, { value: void 0 }), o = [];
        break;
      case "textarea":
        l = go(e, l), r = go(e, r), o = [];
        break;
      default:
        typeof l.onClick != "function" && typeof r.onClick == "function" && (e.onclick = ol);
    }
    wo(n, r);
    var i;
    n = null;
    for (f in l) if (!r.hasOwnProperty(f) && l.hasOwnProperty(f) && l[f] != null) if (f === "style") {
      var s = l[f];
      for (i in s) s.hasOwnProperty(i) && (n || (n = {}), n[i] = "");
    } else f !== "dangerouslySetInnerHTML" && f !== "children" && f !== "suppressContentEditableWarning" && f !== "suppressHydrationWarning" && f !== "autoFocus" && (er.hasOwnProperty(f) ? o || (o = []) : (o = o || []).push(f, null));
    for (f in r) {
      var u = r[f];
      if (s = l != null ? l[f] : void 0, r.hasOwnProperty(f) && u !== s && (u != null || s != null)) if (f === "style") if (s) {
        for (i in s) !s.hasOwnProperty(i) || u && u.hasOwnProperty(i) || (n || (n = {}), n[i] = "");
        for (i in u) u.hasOwnProperty(i) && s[i] !== u[i] && (n || (n = {}), n[i] = u[i]);
      } else n || (o || (o = []), o.push(
        f,
        n
      )), n = u;
      else f === "dangerouslySetInnerHTML" ? (u = u ? u.__html : void 0, s = s ? s.__html : void 0, u != null && s !== u && (o = o || []).push(f, u)) : f === "children" ? typeof u != "string" && typeof u != "number" || (o = o || []).push(f, "" + u) : f !== "suppressContentEditableWarning" && f !== "suppressHydrationWarning" && (er.hasOwnProperty(f) ? (u != null && f === "onScroll" && K("scroll", e), o || s === u || (o = [])) : (o = o || []).push(f, u));
    }
    n && (o = o || []).push("style", n);
    var f = o;
    (t.updateQueue = f) && (t.flags |= 4);
  }
};
rc = function(e, t, n, r) {
  n !== r && (t.flags |= 4);
};
function Bn(e, t) {
  if (!Y) switch (e.tailMode) {
    case "hidden":
      t = e.tail;
      for (var n = null; t !== null; ) t.alternate !== null && (n = t), t = t.sibling;
      n === null ? e.tail = null : n.sibling = null;
      break;
    case "collapsed":
      n = e.tail;
      for (var r = null; n !== null; ) n.alternate !== null && (r = n), n = n.sibling;
      r === null ? t || e.tail === null ? e.tail = null : e.tail.sibling = null : r.sibling = null;
  }
}
function fe(e) {
  var t = e.alternate !== null && e.alternate.child === e.child, n = 0, r = 0;
  if (t) for (var l = e.child; l !== null; ) n |= l.lanes | l.childLanes, r |= l.subtreeFlags & 14680064, r |= l.flags & 14680064, l.return = e, l = l.sibling;
  else for (l = e.child; l !== null; ) n |= l.lanes | l.childLanes, r |= l.subtreeFlags, r |= l.flags, l.return = e, l = l.sibling;
  return e.subtreeFlags |= r, e.childLanes = n, t;
}
function jf(e, t, n) {
  var r = t.pendingProps;
  switch (Ei(t), t.tag) {
    case 2:
    case 16:
    case 15:
    case 0:
    case 11:
    case 7:
    case 8:
    case 12:
    case 9:
    case 14:
      return fe(t), null;
    case 1:
      return Ee(t.type) && il(), fe(t), null;
    case 3:
      return r = t.stateNode, Sn(), Q(Ce), Q(me), Oi(), r.pendingContext && (r.context = r.pendingContext, r.pendingContext = null), (e === null || e.child === null) && (Or(t) ? t.flags |= 4 : e === null || e.memoizedState.isDehydrated && !(t.flags & 256) || (t.flags |= 1024, Qe !== null && (ti(Qe), Qe = null))), Yo(e, t), fe(t), null;
    case 5:
      Li(t);
      var l = Wt(fr.current);
      if (n = t.type, e !== null && t.stateNode != null) nc(e, t, n, r, l), e.ref !== t.ref && (t.flags |= 512, t.flags |= 2097152);
      else {
        if (!r) {
          if (t.stateNode === null) throw Error(w(166));
          return fe(t), null;
        }
        if (e = Wt(nt.current), Or(t)) {
          r = t.stateNode, n = t.type;
          var o = t.memoizedProps;
          switch (r[et] = t, r[cr] = o, e = (t.mode & 1) !== 0, n) {
            case "dialog":
              K("cancel", r), K("close", r);
              break;
            case "iframe":
            case "object":
            case "embed":
              K("load", r);
              break;
            case "video":
            case "audio":
              for (l = 0; l < Vn.length; l++) K(Vn[l], r);
              break;
            case "source":
              K("error", r);
              break;
            case "img":
            case "image":
            case "link":
              K(
                "error",
                r
              ), K("load", r);
              break;
            case "details":
              K("toggle", r);
              break;
            case "input":
              ls(r, o), K("invalid", r);
              break;
            case "select":
              r._wrapperState = { wasMultiple: !!o.multiple }, K("invalid", r);
              break;
            case "textarea":
              is(r, o), K("invalid", r);
          }
          wo(n, o), l = null;
          for (var i in o) if (o.hasOwnProperty(i)) {
            var s = o[i];
            i === "children" ? typeof s == "string" ? r.textContent !== s && (o.suppressHydrationWarning !== !0 && Lr(r.textContent, s, e), l = ["children", s]) : typeof s == "number" && r.textContent !== "" + s && (o.suppressHydrationWarning !== !0 && Lr(
              r.textContent,
              s,
              e
            ), l = ["children", "" + s]) : er.hasOwnProperty(i) && s != null && i === "onScroll" && K("scroll", r);
          }
          switch (n) {
            case "input":
              Er(r), os(r, o, !0);
              break;
            case "textarea":
              Er(r), ss(r);
              break;
            case "select":
            case "option":
              break;
            default:
              typeof o.onClick == "function" && (r.onclick = ol);
          }
          r = l, t.updateQueue = r, r !== null && (t.flags |= 4);
        } else {
          i = l.nodeType === 9 ? l : l.ownerDocument, e === "http://www.w3.org/1999/xhtml" && (e = Ru(n)), e === "http://www.w3.org/1999/xhtml" ? n === "script" ? (e = i.createElement("div"), e.innerHTML = "<script><\/script>", e = e.removeChild(e.firstChild)) : typeof r.is == "string" ? e = i.createElement(n, { is: r.is }) : (e = i.createElement(n), n === "select" && (i = e, r.multiple ? i.multiple = !0 : r.size && (i.size = r.size))) : e = i.createElementNS(e, n), e[et] = t, e[cr] = r, tc(e, t, !1, !1), t.stateNode = e;
          e: {
            switch (i = So(n, r), n) {
              case "dialog":
                K("cancel", e), K("close", e), l = r;
                break;
              case "iframe":
              case "object":
              case "embed":
                K("load", e), l = r;
                break;
              case "video":
              case "audio":
                for (l = 0; l < Vn.length; l++) K(Vn[l], e);
                l = r;
                break;
              case "source":
                K("error", e), l = r;
                break;
              case "img":
              case "image":
              case "link":
                K(
                  "error",
                  e
                ), K("load", e), l = r;
                break;
              case "details":
                K("toggle", e), l = r;
                break;
              case "input":
                ls(e, r), l = ho(e, r), K("invalid", e);
                break;
              case "option":
                l = r;
                break;
              case "select":
                e._wrapperState = { wasMultiple: !!r.multiple }, l = Z({}, r, { value: void 0 }), K("invalid", e);
                break;
              case "textarea":
                is(e, r), l = go(e, r), K("invalid", e);
                break;
              default:
                l = r;
            }
            wo(n, l), s = l;
            for (o in s) if (s.hasOwnProperty(o)) {
              var u = s[o];
              o === "style" ? Du(e, u) : o === "dangerouslySetInnerHTML" ? (u = u ? u.__html : void 0, u != null && Lu(e, u)) : o === "children" ? typeof u == "string" ? (n !== "textarea" || u !== "") && tr(e, u) : typeof u == "number" && tr(e, "" + u) : o !== "suppressContentEditableWarning" && o !== "suppressHydrationWarning" && o !== "autoFocus" && (er.hasOwnProperty(o) ? u != null && o === "onScroll" && K("scroll", e) : u != null && ai(e, o, u, i));
            }
            switch (n) {
              case "input":
                Er(e), os(e, r, !1);
                break;
              case "textarea":
                Er(e), ss(e);
                break;
              case "option":
                r.value != null && e.setAttribute("value", "" + Nt(r.value));
                break;
              case "select":
                e.multiple = !!r.multiple, o = r.value, o != null ? dn(e, !!r.multiple, o, !1) : r.defaultValue != null && dn(
                  e,
                  !!r.multiple,
                  r.defaultValue,
                  !0
                );
                break;
              default:
                typeof l.onClick == "function" && (e.onclick = ol);
            }
            switch (n) {
              case "button":
              case "input":
              case "select":
              case "textarea":
                r = !!r.autoFocus;
                break e;
              case "img":
                r = !0;
                break e;
              default:
                r = !1;
            }
          }
          r && (t.flags |= 4);
        }
        t.ref !== null && (t.flags |= 512, t.flags |= 2097152);
      }
      return fe(t), null;
    case 6:
      if (e && t.stateNode != null) rc(e, t, e.memoizedProps, r);
      else {
        if (typeof r != "string" && t.stateNode === null) throw Error(w(166));
        if (n = Wt(fr.current), Wt(nt.current), Or(t)) {
          if (r = t.stateNode, n = t.memoizedProps, r[et] = t, (o = r.nodeValue !== n) && (e = Te, e !== null)) switch (e.tag) {
            case 3:
              Lr(r.nodeValue, n, (e.mode & 1) !== 0);
              break;
            case 5:
              e.memoizedProps.suppressHydrationWarning !== !0 && Lr(r.nodeValue, n, (e.mode & 1) !== 0);
          }
          o && (t.flags |= 4);
        } else r = (n.nodeType === 9 ? n : n.ownerDocument).createTextNode(r), r[et] = t, t.stateNode = r;
      }
      return fe(t), null;
    case 13:
      if (Q(X), r = t.memoizedState, e === null || e.memoizedState !== null && e.memoizedState.dehydrated !== null) {
        if (Y && ze !== null && t.mode & 1 && !(t.flags & 128)) Sa(), xn(), t.flags |= 98560, o = !1;
        else if (o = Or(t), r !== null && r.dehydrated !== null) {
          if (e === null) {
            if (!o) throw Error(w(318));
            if (o = t.memoizedState, o = o !== null ? o.dehydrated : null, !o) throw Error(w(317));
            o[et] = t;
          } else xn(), !(t.flags & 128) && (t.memoizedState = null), t.flags |= 4;
          fe(t), o = !1;
        } else Qe !== null && (ti(Qe), Qe = null), o = !0;
        if (!o) return t.flags & 65536 ? t : null;
      }
      return t.flags & 128 ? (t.lanes = n, t) : (r = r !== null, r !== (e !== null && e.memoizedState !== null) && r && (t.child.flags |= 8192, t.mode & 1 && (e === null || X.current & 1 ? te === 0 && (te = 3) : Ki())), t.updateQueue !== null && (t.flags |= 4), fe(t), null);
    case 4:
      return Sn(), Yo(e, t), e === null && ur(t.stateNode.containerInfo), fe(t), null;
    case 10:
      return Ti(t.type._context), fe(t), null;
    case 17:
      return Ee(t.type) && il(), fe(t), null;
    case 19:
      if (Q(X), o = t.memoizedState, o === null) return fe(t), null;
      if (r = (t.flags & 128) !== 0, i = o.rendering, i === null) if (r) Bn(o, !1);
      else {
        if (te !== 0 || e !== null && e.flags & 128) for (e = t.child; e !== null; ) {
          if (i = pl(e), i !== null) {
            for (t.flags |= 128, Bn(o, !1), r = i.updateQueue, r !== null && (t.updateQueue = r, t.flags |= 4), t.subtreeFlags = 0, r = n, n = t.child; n !== null; ) o = n, e = r, o.flags &= 14680066, i = o.alternate, i === null ? (o.childLanes = 0, o.lanes = e, o.child = null, o.subtreeFlags = 0, o.memoizedProps = null, o.memoizedState = null, o.updateQueue = null, o.dependencies = null, o.stateNode = null) : (o.childLanes = i.childLanes, o.lanes = i.lanes, o.child = i.child, o.subtreeFlags = 0, o.deletions = null, o.memoizedProps = i.memoizedProps, o.memoizedState = i.memoizedState, o.updateQueue = i.updateQueue, o.type = i.type, e = i.dependencies, o.dependencies = e === null ? null : { lanes: e.lanes, firstContext: e.firstContext }), n = n.sibling;
            return H(X, X.current & 1 | 2), t.child;
          }
          e = e.sibling;
        }
        o.tail !== null && q() > Cn && (t.flags |= 128, r = !0, Bn(o, !1), t.lanes = 4194304);
      }
      else {
        if (!r) if (e = pl(i), e !== null) {
          if (t.flags |= 128, r = !0, n = e.updateQueue, n !== null && (t.updateQueue = n, t.flags |= 4), Bn(o, !0), o.tail === null && o.tailMode === "hidden" && !i.alternate && !Y) return fe(t), null;
        } else 2 * q() - o.renderingStartTime > Cn && n !== 1073741824 && (t.flags |= 128, r = !0, Bn(o, !1), t.lanes = 4194304);
        o.isBackwards ? (i.sibling = t.child, t.child = i) : (n = o.last, n !== null ? n.sibling = i : t.child = i, o.last = i);
      }
      return o.tail !== null ? (t = o.tail, o.rendering = t, o.tail = t.sibling, o.renderingStartTime = q(), t.sibling = null, n = X.current, H(X, r ? n & 1 | 2 : n & 1), t) : (fe(t), null);
    case 22:
    case 23:
      return Hi(), r = t.memoizedState !== null, e !== null && e.memoizedState !== null !== r && (t.flags |= 8192), r && t.mode & 1 ? _e & 1073741824 && (fe(t), t.subtreeFlags & 6 && (t.flags |= 8192)) : fe(t), null;
    case 24:
      return null;
    case 25:
      return null;
  }
  throw Error(w(156, t.tag));
}
function _f(e, t) {
  switch (Ei(t), t.tag) {
    case 1:
      return Ee(t.type) && il(), e = t.flags, e & 65536 ? (t.flags = e & -65537 | 128, t) : null;
    case 3:
      return Sn(), Q(Ce), Q(me), Oi(), e = t.flags, e & 65536 && !(e & 128) ? (t.flags = e & -65537 | 128, t) : null;
    case 5:
      return Li(t), null;
    case 13:
      if (Q(X), e = t.memoizedState, e !== null && e.dehydrated !== null) {
        if (t.alternate === null) throw Error(w(340));
        xn();
      }
      return e = t.flags, e & 65536 ? (t.flags = e & -65537 | 128, t) : null;
    case 19:
      return Q(X), null;
    case 4:
      return Sn(), null;
    case 10:
      return Ti(t.type._context), null;
    case 22:
    case 23:
      return Hi(), null;
    case 24:
      return null;
    default:
      return null;
  }
}
var Ar = !1, pe = !1, zf = typeof WeakSet == "function" ? WeakSet : Set, z = null;
function an(e, t) {
  var n = e.ref;
  if (n !== null) if (typeof n == "function") try {
    n(null);
  } catch (r) {
    J(e, t, r);
  }
  else n.current = null;
}
function Xo(e, t, n) {
  try {
    n();
  } catch (r) {
    J(e, t, r);
  }
}
var Zs = !1;
function Tf(e, t) {
  if (Ro = nl, e = ua(), ki(e)) {
    if ("selectionStart" in e) var n = { start: e.selectionStart, end: e.selectionEnd };
    else e: {
      n = (n = e.ownerDocument) && n.defaultView || window;
      var r = n.getSelection && n.getSelection();
      if (r && r.rangeCount !== 0) {
        n = r.anchorNode;
        var l = r.anchorOffset, o = r.focusNode;
        r = r.focusOffset;
        try {
          n.nodeType, o.nodeType;
        } catch {
          n = null;
          break e;
        }
        var i = 0, s = -1, u = -1, f = 0, m = 0, y = e, h = null;
        t: for (; ; ) {
          for (var g; y !== n || l !== 0 && y.nodeType !== 3 || (s = i + l), y !== o || r !== 0 && y.nodeType !== 3 || (u = i + r), y.nodeType === 3 && (i += y.nodeValue.length), (g = y.firstChild) !== null; )
            h = y, y = g;
          for (; ; ) {
            if (y === e) break t;
            if (h === n && ++f === l && (s = i), h === o && ++m === r && (u = i), (g = y.nextSibling) !== null) break;
            y = h, h = y.parentNode;
          }
          y = g;
        }
        n = s === -1 || u === -1 ? null : { start: s, end: u };
      } else n = null;
    }
    n = n || { start: 0, end: 0 };
  } else n = null;
  for (Lo = { focusedElem: e, selectionRange: n }, nl = !1, z = t; z !== null; ) if (t = z, e = t.child, (t.subtreeFlags & 1028) !== 0 && e !== null) e.return = t, z = e;
  else for (; z !== null; ) {
    t = z;
    try {
      var x = t.alternate;
      if (t.flags & 1024) switch (t.tag) {
        case 0:
        case 11:
        case 15:
          break;
        case 1:
          if (x !== null) {
            var S = x.memoizedProps, O = x.memoizedState, d = t.stateNode, c = d.getSnapshotBeforeUpdate(t.elementType === t.type ? S : He(t.type, S), O);
            d.__reactInternalSnapshotBeforeUpdate = c;
          }
          break;
        case 3:
          var p = t.stateNode.containerInfo;
          p.nodeType === 1 ? p.textContent = "" : p.nodeType === 9 && p.documentElement && p.removeChild(p.documentElement);
          break;
        case 5:
        case 6:
        case 4:
        case 17:
          break;
        default:
          throw Error(w(163));
      }
    } catch (v) {
      J(t, t.return, v);
    }
    if (e = t.sibling, e !== null) {
      e.return = t.return, z = e;
      break;
    }
    z = t.return;
  }
  return x = Zs, Zs = !1, x;
}
function Zn(e, t, n) {
  var r = t.updateQueue;
  if (r = r !== null ? r.lastEffect : null, r !== null) {
    var l = r = r.next;
    do {
      if ((l.tag & e) === e) {
        var o = l.destroy;
        l.destroy = void 0, o !== void 0 && Xo(t, n, o);
      }
      l = l.next;
    } while (l !== r);
  }
}
function Pl(e, t) {
  if (t = t.updateQueue, t = t !== null ? t.lastEffect : null, t !== null) {
    var n = t = t.next;
    do {
      if ((n.tag & e) === e) {
        var r = n.create;
        n.destroy = r();
      }
      n = n.next;
    } while (n !== t);
  }
}
function Go(e) {
  var t = e.ref;
  if (t !== null) {
    var n = e.stateNode;
    switch (e.tag) {
      case 5:
        e = n;
        break;
      default:
        e = n;
    }
    typeof t == "function" ? t(e) : t.current = e;
  }
}
function lc(e) {
  var t = e.alternate;
  t !== null && (e.alternate = null, lc(t)), e.child = null, e.deletions = null, e.sibling = null, e.tag === 5 && (t = e.stateNode, t !== null && (delete t[et], delete t[cr], delete t[Io], delete t[cf], delete t[df])), e.stateNode = null, e.return = null, e.dependencies = null, e.memoizedProps = null, e.memoizedState = null, e.pendingProps = null, e.stateNode = null, e.updateQueue = null;
}
function oc(e) {
  return e.tag === 5 || e.tag === 3 || e.tag === 4;
}
function Js(e) {
  e: for (; ; ) {
    for (; e.sibling === null; ) {
      if (e.return === null || oc(e.return)) return null;
      e = e.return;
    }
    for (e.sibling.return = e.return, e = e.sibling; e.tag !== 5 && e.tag !== 6 && e.tag !== 18; ) {
      if (e.flags & 2 || e.child === null || e.tag === 4) continue e;
      e.child.return = e, e = e.child;
    }
    if (!(e.flags & 2)) return e.stateNode;
  }
}
function Zo(e, t, n) {
  var r = e.tag;
  if (r === 5 || r === 6) e = e.stateNode, t ? n.nodeType === 8 ? n.parentNode.insertBefore(e, t) : n.insertBefore(e, t) : (n.nodeType === 8 ? (t = n.parentNode, t.insertBefore(e, n)) : (t = n, t.appendChild(e)), n = n._reactRootContainer, n != null || t.onclick !== null || (t.onclick = ol));
  else if (r !== 4 && (e = e.child, e !== null)) for (Zo(e, t, n), e = e.sibling; e !== null; ) Zo(e, t, n), e = e.sibling;
}
function Jo(e, t, n) {
  var r = e.tag;
  if (r === 5 || r === 6) e = e.stateNode, t ? n.insertBefore(e, t) : n.appendChild(e);
  else if (r !== 4 && (e = e.child, e !== null)) for (Jo(e, t, n), e = e.sibling; e !== null; ) Jo(e, t, n), e = e.sibling;
}
var oe = null, Ke = !1;
function pt(e, t, n) {
  for (n = n.child; n !== null; ) ic(e, t, n), n = n.sibling;
}
function ic(e, t, n) {
  if (tt && typeof tt.onCommitFiberUnmount == "function") try {
    tt.onCommitFiberUnmount(Sl, n);
  } catch {
  }
  switch (n.tag) {
    case 5:
      pe || an(n, t);
    case 6:
      var r = oe, l = Ke;
      oe = null, pt(e, t, n), oe = r, Ke = l, oe !== null && (Ke ? (e = oe, n = n.stateNode, e.nodeType === 8 ? e.parentNode.removeChild(n) : e.removeChild(n)) : oe.removeChild(n.stateNode));
      break;
    case 18:
      oe !== null && (Ke ? (e = oe, n = n.stateNode, e.nodeType === 8 ? bl(e.parentNode, n) : e.nodeType === 1 && bl(e, n), or(e)) : bl(oe, n.stateNode));
      break;
    case 4:
      r = oe, l = Ke, oe = n.stateNode.containerInfo, Ke = !0, pt(e, t, n), oe = r, Ke = l;
      break;
    case 0:
    case 11:
    case 14:
    case 15:
      if (!pe && (r = n.updateQueue, r !== null && (r = r.lastEffect, r !== null))) {
        l = r = r.next;
        do {
          var o = l, i = o.destroy;
          o = o.tag, i !== void 0 && (o & 2 || o & 4) && Xo(n, t, i), l = l.next;
        } while (l !== r);
      }
      pt(e, t, n);
      break;
    case 1:
      if (!pe && (an(n, t), r = n.stateNode, typeof r.componentWillUnmount == "function")) try {
        r.props = n.memoizedProps, r.state = n.memoizedState, r.componentWillUnmount();
      } catch (s) {
        J(n, t, s);
      }
      pt(e, t, n);
      break;
    case 21:
      pt(e, t, n);
      break;
    case 22:
      n.mode & 1 ? (pe = (r = pe) || n.memoizedState !== null, pt(e, t, n), pe = r) : pt(e, t, n);
      break;
    default:
      pt(e, t, n);
  }
}
function qs(e) {
  var t = e.updateQueue;
  if (t !== null) {
    e.updateQueue = null;
    var n = e.stateNode;
    n === null && (n = e.stateNode = new zf()), t.forEach(function(r) {
      var l = Mf.bind(null, e, r);
      n.has(r) || (n.add(r), r.then(l, l));
    });
  }
}
function Ve(e, t) {
  var n = t.deletions;
  if (n !== null) for (var r = 0; r < n.length; r++) {
    var l = n[r];
    try {
      var o = e, i = t, s = i;
      e: for (; s !== null; ) {
        switch (s.tag) {
          case 5:
            oe = s.stateNode, Ke = !1;
            break e;
          case 3:
            oe = s.stateNode.containerInfo, Ke = !0;
            break e;
          case 4:
            oe = s.stateNode.containerInfo, Ke = !0;
            break e;
        }
        s = s.return;
      }
      if (oe === null) throw Error(w(160));
      ic(o, i, l), oe = null, Ke = !1;
      var u = l.alternate;
      u !== null && (u.return = null), l.return = null;
    } catch (f) {
      J(l, t, f);
    }
  }
  if (t.subtreeFlags & 12854) for (t = t.child; t !== null; ) sc(t, e), t = t.sibling;
}
function sc(e, t) {
  var n = e.alternate, r = e.flags;
  switch (e.tag) {
    case 0:
    case 11:
    case 14:
    case 15:
      if (Ve(t, e), qe(e), r & 4) {
        try {
          Zn(3, e, e.return), Pl(3, e);
        } catch (S) {
          J(e, e.return, S);
        }
        try {
          Zn(5, e, e.return);
        } catch (S) {
          J(e, e.return, S);
        }
      }
      break;
    case 1:
      Ve(t, e), qe(e), r & 512 && n !== null && an(n, n.return);
      break;
    case 5:
      if (Ve(t, e), qe(e), r & 512 && n !== null && an(n, n.return), e.flags & 32) {
        var l = e.stateNode;
        try {
          tr(l, "");
        } catch (S) {
          J(e, e.return, S);
        }
      }
      if (r & 4 && (l = e.stateNode, l != null)) {
        var o = e.memoizedProps, i = n !== null ? n.memoizedProps : o, s = e.type, u = e.updateQueue;
        if (e.updateQueue = null, u !== null) try {
          s === "input" && o.type === "radio" && o.name != null && Pu(l, o), So(s, i);
          var f = So(s, o);
          for (i = 0; i < u.length; i += 2) {
            var m = u[i], y = u[i + 1];
            m === "style" ? Du(l, y) : m === "dangerouslySetInnerHTML" ? Lu(l, y) : m === "children" ? tr(l, y) : ai(l, m, y, f);
          }
          switch (s) {
            case "input":
              yo(l, o);
              break;
            case "textarea":
              Nu(l, o);
              break;
            case "select":
              var h = l._wrapperState.wasMultiple;
              l._wrapperState.wasMultiple = !!o.multiple;
              var g = o.value;
              g != null ? dn(l, !!o.multiple, g, !1) : h !== !!o.multiple && (o.defaultValue != null ? dn(
                l,
                !!o.multiple,
                o.defaultValue,
                !0
              ) : dn(l, !!o.multiple, o.multiple ? [] : "", !1));
          }
          l[cr] = o;
        } catch (S) {
          J(e, e.return, S);
        }
      }
      break;
    case 6:
      if (Ve(t, e), qe(e), r & 4) {
        if (e.stateNode === null) throw Error(w(162));
        l = e.stateNode, o = e.memoizedProps;
        try {
          l.nodeValue = o;
        } catch (S) {
          J(e, e.return, S);
        }
      }
      break;
    case 3:
      if (Ve(t, e), qe(e), r & 4 && n !== null && n.memoizedState.isDehydrated) try {
        or(t.containerInfo);
      } catch (S) {
        J(e, e.return, S);
      }
      break;
    case 4:
      Ve(t, e), qe(e);
      break;
    case 13:
      Ve(t, e), qe(e), l = e.child, l.flags & 8192 && (o = l.memoizedState !== null, l.stateNode.isHidden = o, !o || l.alternate !== null && l.alternate.memoizedState !== null || ($i = q())), r & 4 && qs(e);
      break;
    case 22:
      if (m = n !== null && n.memoizedState !== null, e.mode & 1 ? (pe = (f = pe) || m, Ve(t, e), pe = f) : Ve(t, e), qe(e), r & 8192) {
        if (f = e.memoizedState !== null, (e.stateNode.isHidden = f) && !m && e.mode & 1) for (z = e, m = e.child; m !== null; ) {
          for (y = z = m; z !== null; ) {
            switch (h = z, g = h.child, h.tag) {
              case 0:
              case 11:
              case 14:
              case 15:
                Zn(4, h, h.return);
                break;
              case 1:
                an(h, h.return);
                var x = h.stateNode;
                if (typeof x.componentWillUnmount == "function") {
                  r = h, n = h.return;
                  try {
                    t = r, x.props = t.memoizedProps, x.state = t.memoizedState, x.componentWillUnmount();
                  } catch (S) {
                    J(r, n, S);
                  }
                }
                break;
              case 5:
                an(h, h.return);
                break;
              case 22:
                if (h.memoizedState !== null) {
                  eu(y);
                  continue;
                }
            }
            g !== null ? (g.return = h, z = g) : eu(y);
          }
          m = m.sibling;
        }
        e: for (m = null, y = e; ; ) {
          if (y.tag === 5) {
            if (m === null) {
              m = y;
              try {
                l = y.stateNode, f ? (o = l.style, typeof o.setProperty == "function" ? o.setProperty("display", "none", "important") : o.display = "none") : (s = y.stateNode, u = y.memoizedProps.style, i = u != null && u.hasOwnProperty("display") ? u.display : null, s.style.display = Ou("display", i));
              } catch (S) {
                J(e, e.return, S);
              }
            }
          } else if (y.tag === 6) {
            if (m === null) try {
              y.stateNode.nodeValue = f ? "" : y.memoizedProps;
            } catch (S) {
              J(e, e.return, S);
            }
          } else if ((y.tag !== 22 && y.tag !== 23 || y.memoizedState === null || y === e) && y.child !== null) {
            y.child.return = y, y = y.child;
            continue;
          }
          if (y === e) break e;
          for (; y.sibling === null; ) {
            if (y.return === null || y.return === e) break e;
            m === y && (m = null), y = y.return;
          }
          m === y && (m = null), y.sibling.return = y.return, y = y.sibling;
        }
      }
      break;
    case 19:
      Ve(t, e), qe(e), r & 4 && qs(e);
      break;
    case 21:
      break;
    default:
      Ve(
        t,
        e
      ), qe(e);
  }
}
function qe(e) {
  var t = e.flags;
  if (t & 2) {
    try {
      e: {
        for (var n = e.return; n !== null; ) {
          if (oc(n)) {
            var r = n;
            break e;
          }
          n = n.return;
        }
        throw Error(w(160));
      }
      switch (r.tag) {
        case 5:
          var l = r.stateNode;
          r.flags & 32 && (tr(l, ""), r.flags &= -33);
          var o = Js(e);
          Jo(e, o, l);
          break;
        case 3:
        case 4:
          var i = r.stateNode.containerInfo, s = Js(e);
          Zo(e, s, i);
          break;
        default:
          throw Error(w(161));
      }
    } catch (u) {
      J(e, e.return, u);
    }
    e.flags &= -3;
  }
  t & 4096 && (e.flags &= -4097);
}
function Pf(e, t, n) {
  z = e, uc(e);
}
function uc(e, t, n) {
  for (var r = (e.mode & 1) !== 0; z !== null; ) {
    var l = z, o = l.child;
    if (l.tag === 22 && r) {
      var i = l.memoizedState !== null || Ar;
      if (!i) {
        var s = l.alternate, u = s !== null && s.memoizedState !== null || pe;
        s = Ar;
        var f = pe;
        if (Ar = i, (pe = u) && !f) for (z = l; z !== null; ) i = z, u = i.child, i.tag === 22 && i.memoizedState !== null ? tu(l) : u !== null ? (u.return = i, z = u) : tu(l);
        for (; o !== null; ) z = o, uc(o), o = o.sibling;
        z = l, Ar = s, pe = f;
      }
      bs(e);
    } else l.subtreeFlags & 8772 && o !== null ? (o.return = l, z = o) : bs(e);
  }
}
function bs(e) {
  for (; z !== null; ) {
    var t = z;
    if (t.flags & 8772) {
      var n = t.alternate;
      try {
        if (t.flags & 8772) switch (t.tag) {
          case 0:
          case 11:
          case 15:
            pe || Pl(5, t);
            break;
          case 1:
            var r = t.stateNode;
            if (t.flags & 4 && !pe) if (n === null) r.componentDidMount();
            else {
              var l = t.elementType === t.type ? n.memoizedProps : He(t.type, n.memoizedProps);
              r.componentDidUpdate(l, n.memoizedState, r.__reactInternalSnapshotBeforeUpdate);
            }
            var o = t.updateQueue;
            o !== null && Ms(t, o, r);
            break;
          case 3:
            var i = t.updateQueue;
            if (i !== null) {
              if (n = null, t.child !== null) switch (t.child.tag) {
                case 5:
                  n = t.child.stateNode;
                  break;
                case 1:
                  n = t.child.stateNode;
              }
              Ms(t, i, n);
            }
            break;
          case 5:
            var s = t.stateNode;
            if (n === null && t.flags & 4) {
              n = s;
              var u = t.memoizedProps;
              switch (t.type) {
                case "button":
                case "input":
                case "select":
                case "textarea":
                  u.autoFocus && n.focus();
                  break;
                case "img":
                  u.src && (n.src = u.src);
              }
            }
            break;
          case 6:
            break;
          case 4:
            break;
          case 12:
            break;
          case 13:
            if (t.memoizedState === null) {
              var f = t.alternate;
              if (f !== null) {
                var m = f.memoizedState;
                if (m !== null) {
                  var y = m.dehydrated;
                  y !== null && or(y);
                }
              }
            }
            break;
          case 19:
          case 17:
          case 21:
          case 22:
          case 23:
          case 25:
            break;
          default:
            throw Error(w(163));
        }
        pe || t.flags & 512 && Go(t);
      } catch (h) {
        J(t, t.return, h);
      }
    }
    if (t === e) {
      z = null;
      break;
    }
    if (n = t.sibling, n !== null) {
      n.return = t.return, z = n;
      break;
    }
    z = t.return;
  }
}
function eu(e) {
  for (; z !== null; ) {
    var t = z;
    if (t === e) {
      z = null;
      break;
    }
    var n = t.sibling;
    if (n !== null) {
      n.return = t.return, z = n;
      break;
    }
    z = t.return;
  }
}
function tu(e) {
  for (; z !== null; ) {
    var t = z;
    try {
      switch (t.tag) {
        case 0:
        case 11:
        case 15:
          var n = t.return;
          try {
            Pl(4, t);
          } catch (u) {
            J(t, n, u);
          }
          break;
        case 1:
          var r = t.stateNode;
          if (typeof r.componentDidMount == "function") {
            var l = t.return;
            try {
              r.componentDidMount();
            } catch (u) {
              J(t, l, u);
            }
          }
          var o = t.return;
          try {
            Go(t);
          } catch (u) {
            J(t, o, u);
          }
          break;
        case 5:
          var i = t.return;
          try {
            Go(t);
          } catch (u) {
            J(t, i, u);
          }
      }
    } catch (u) {
      J(t, t.return, u);
    }
    if (t === e) {
      z = null;
      break;
    }
    var s = t.sibling;
    if (s !== null) {
      s.return = t.return, z = s;
      break;
    }
    z = t.return;
  }
}
var Nf = Math.ceil, yl = ft.ReactCurrentDispatcher, Ui = ft.ReactCurrentOwner, Me = ft.ReactCurrentBatchConfig, M = 0, le = null, b = null, ie = 0, _e = 0, cn = Ot(0), te = 0, yr = null, Yt = 0, Nl = 0, Wi = 0, Jn = null, Se = null, $i = 0, Cn = 1 / 0, rt = null, vl = !1, qo = null, zt = null, Mr = !1, wt = null, gl = 0, qn = 0, bo = null, Xr = -1, Gr = 0;
function ye() {
  return M & 6 ? q() : Xr !== -1 ? Xr : Xr = q();
}
function Tt(e) {
  return e.mode & 1 ? M & 2 && ie !== 0 ? ie & -ie : pf.transition !== null ? (Gr === 0 && (Gr = Qu()), Gr) : (e = W, e !== 0 || (e = window.event, e = e === void 0 ? 16 : bu(e.type)), e) : 1;
}
function Xe(e, t, n, r) {
  if (50 < qn) throw qn = 0, bo = null, Error(w(185));
  gr(e, n, r), (!(M & 2) || e !== le) && (e === le && (!(M & 2) && (Nl |= n), te === 4 && gt(e, ie)), je(e, r), n === 1 && M === 0 && !(t.mode & 1) && (Cn = q() + 500, _l && Dt()));
}
function je(e, t) {
  var n = e.callbackNode;
  fd(e, t);
  var r = tl(e, e === le ? ie : 0);
  if (r === 0) n !== null && cs(n), e.callbackNode = null, e.callbackPriority = 0;
  else if (t = r & -r, e.callbackPriority !== t) {
    if (n != null && cs(n), t === 1) e.tag === 0 ? ff(nu.bind(null, e)) : ga(nu.bind(null, e)), uf(function() {
      !(M & 6) && Dt();
    }), n = null;
    else {
      switch (Yu(r)) {
        case 1:
          n = mi;
          break;
        case 4:
          n = Hu;
          break;
        case 16:
          n = el;
          break;
        case 536870912:
          n = Ku;
          break;
        default:
          n = el;
      }
      n = yc(n, ac.bind(null, e));
    }
    e.callbackPriority = t, e.callbackNode = n;
  }
}
function ac(e, t) {
  if (Xr = -1, Gr = 0, M & 6) throw Error(w(327));
  var n = e.callbackNode;
  if (yn() && e.callbackNode !== n) return null;
  var r = tl(e, e === le ? ie : 0);
  if (r === 0) return null;
  if (r & 30 || r & e.expiredLanes || t) t = xl(e, r);
  else {
    t = r;
    var l = M;
    M |= 2;
    var o = dc();
    (le !== e || ie !== t) && (rt = null, Cn = q() + 500, $t(e, t));
    do
      try {
        Of();
        break;
      } catch (s) {
        cc(e, s);
      }
    while (!0);
    zi(), yl.current = o, M = l, b !== null ? t = 0 : (le = null, ie = 0, t = te);
  }
  if (t !== 0) {
    if (t === 2 && (l = _o(e), l !== 0 && (r = l, t = ei(e, l))), t === 1) throw n = yr, $t(e, 0), gt(e, r), je(e, q()), n;
    if (t === 6) gt(e, r);
    else {
      if (l = e.current.alternate, !(r & 30) && !Rf(l) && (t = xl(e, r), t === 2 && (o = _o(e), o !== 0 && (r = o, t = ei(e, o))), t === 1)) throw n = yr, $t(e, 0), gt(e, r), je(e, q()), n;
      switch (e.finishedWork = l, e.finishedLanes = r, t) {
        case 0:
        case 1:
          throw Error(w(345));
        case 2:
          Bt(e, Se, rt);
          break;
        case 3:
          if (gt(e, r), (r & 130023424) === r && (t = $i + 500 - q(), 10 < t)) {
            if (tl(e, 0) !== 0) break;
            if (l = e.suspendedLanes, (l & r) !== r) {
              ye(), e.pingedLanes |= e.suspendedLanes & l;
              break;
            }
            e.timeoutHandle = Do(Bt.bind(null, e, Se, rt), t);
            break;
          }
          Bt(e, Se, rt);
          break;
        case 4:
          if (gt(e, r), (r & 4194240) === r) break;
          for (t = e.eventTimes, l = -1; 0 < r; ) {
            var i = 31 - Ye(r);
            o = 1 << i, i = t[i], i > l && (l = i), r &= ~o;
          }
          if (r = l, r = q() - r, r = (120 > r ? 120 : 480 > r ? 480 : 1080 > r ? 1080 : 1920 > r ? 1920 : 3e3 > r ? 3e3 : 4320 > r ? 4320 : 1960 * Nf(r / 1960)) - r, 10 < r) {
            e.timeoutHandle = Do(Bt.bind(null, e, Se, rt), r);
            break;
          }
          Bt(e, Se, rt);
          break;
        case 5:
          Bt(e, Se, rt);
          break;
        default:
          throw Error(w(329));
      }
    }
  }
  return je(e, q()), e.callbackNode === n ? ac.bind(null, e) : null;
}
function ei(e, t) {
  var n = Jn;
  return e.current.memoizedState.isDehydrated && ($t(e, t).flags |= 256), e = xl(e, t), e !== 2 && (t = Se, Se = n, t !== null && ti(t)), e;
}
function ti(e) {
  Se === null ? Se = e : Se.push.apply(Se, e);
}
function Rf(e) {
  for (var t = e; ; ) {
    if (t.flags & 16384) {
      var n = t.updateQueue;
      if (n !== null && (n = n.stores, n !== null)) for (var r = 0; r < n.length; r++) {
        var l = n[r], o = l.getSnapshot;
        l = l.value;
        try {
          if (!Ge(o(), l)) return !1;
        } catch {
          return !1;
        }
      }
    }
    if (n = t.child, t.subtreeFlags & 16384 && n !== null) n.return = t, t = n;
    else {
      if (t === e) break;
      for (; t.sibling === null; ) {
        if (t.return === null || t.return === e) return !0;
        t = t.return;
      }
      t.sibling.return = t.return, t = t.sibling;
    }
  }
  return !0;
}
function gt(e, t) {
  for (t &= ~Wi, t &= ~Nl, e.suspendedLanes |= t, e.pingedLanes &= ~t, e = e.expirationTimes; 0 < t; ) {
    var n = 31 - Ye(t), r = 1 << n;
    e[n] = -1, t &= ~r;
  }
}
function nu(e) {
  if (M & 6) throw Error(w(327));
  yn();
  var t = tl(e, 0);
  if (!(t & 1)) return je(e, q()), null;
  var n = xl(e, t);
  if (e.tag !== 0 && n === 2) {
    var r = _o(e);
    r !== 0 && (t = r, n = ei(e, r));
  }
  if (n === 1) throw n = yr, $t(e, 0), gt(e, t), je(e, q()), n;
  if (n === 6) throw Error(w(345));
  return e.finishedWork = e.current.alternate, e.finishedLanes = t, Bt(e, Se, rt), je(e, q()), null;
}
function Vi(e, t) {
  var n = M;
  M |= 1;
  try {
    return e(t);
  } finally {
    M = n, M === 0 && (Cn = q() + 500, _l && Dt());
  }
}
function Xt(e) {
  wt !== null && wt.tag === 0 && !(M & 6) && yn();
  var t = M;
  M |= 1;
  var n = Me.transition, r = W;
  try {
    if (Me.transition = null, W = 1, e) return e();
  } finally {
    W = r, Me.transition = n, M = t, !(M & 6) && Dt();
  }
}
function Hi() {
  _e = cn.current, Q(cn);
}
function $t(e, t) {
  e.finishedWork = null, e.finishedLanes = 0;
  var n = e.timeoutHandle;
  if (n !== -1 && (e.timeoutHandle = -1, sf(n)), b !== null) for (n = b.return; n !== null; ) {
    var r = n;
    switch (Ei(r), r.tag) {
      case 1:
        r = r.type.childContextTypes, r != null && il();
        break;
      case 3:
        Sn(), Q(Ce), Q(me), Oi();
        break;
      case 5:
        Li(r);
        break;
      case 4:
        Sn();
        break;
      case 13:
        Q(X);
        break;
      case 19:
        Q(X);
        break;
      case 10:
        Ti(r.type._context);
        break;
      case 22:
      case 23:
        Hi();
    }
    n = n.return;
  }
  if (le = e, b = e = Pt(e.current, null), ie = _e = t, te = 0, yr = null, Wi = Nl = Yt = 0, Se = Jn = null, Ut !== null) {
    for (t = 0; t < Ut.length; t++) if (n = Ut[t], r = n.interleaved, r !== null) {
      n.interleaved = null;
      var l = r.next, o = n.pending;
      if (o !== null) {
        var i = o.next;
        o.next = l, r.next = i;
      }
      n.pending = r;
    }
    Ut = null;
  }
  return e;
}
function cc(e, t) {
  do {
    var n = b;
    try {
      if (zi(), Kr.current = hl, ml) {
        for (var r = G.memoizedState; r !== null; ) {
          var l = r.queue;
          l !== null && (l.pending = null), r = r.next;
        }
        ml = !1;
      }
      if (Qt = 0, re = ee = G = null, Gn = !1, pr = 0, Ui.current = null, n === null || n.return === null) {
        te = 1, yr = t, b = null;
        break;
      }
      e: {
        var o = e, i = n.return, s = n, u = t;
        if (t = ie, s.flags |= 32768, u !== null && typeof u == "object" && typeof u.then == "function") {
          var f = u, m = s, y = m.tag;
          if (!(m.mode & 1) && (y === 0 || y === 11 || y === 15)) {
            var h = m.alternate;
            h ? (m.updateQueue = h.updateQueue, m.memoizedState = h.memoizedState, m.lanes = h.lanes) : (m.updateQueue = null, m.memoizedState = null);
          }
          var g = Vs(i);
          if (g !== null) {
            g.flags &= -257, Hs(g, i, s, o, t), g.mode & 1 && $s(o, f, t), t = g, u = f;
            var x = t.updateQueue;
            if (x === null) {
              var S = /* @__PURE__ */ new Set();
              S.add(u), t.updateQueue = S;
            } else x.add(u);
            break e;
          } else {
            if (!(t & 1)) {
              $s(o, f, t), Ki();
              break e;
            }
            u = Error(w(426));
          }
        } else if (Y && s.mode & 1) {
          var O = Vs(i);
          if (O !== null) {
            !(O.flags & 65536) && (O.flags |= 256), Hs(O, i, s, o, t), ji(kn(u, s));
            break e;
          }
        }
        o = u = kn(u, s), te !== 4 && (te = 2), Jn === null ? Jn = [o] : Jn.push(o), o = i;
        do {
          switch (o.tag) {
            case 3:
              o.flags |= 65536, t &= -t, o.lanes |= t;
              var d = Ya(o, u, t);
              As(o, d);
              break e;
            case 1:
              s = u;
              var c = o.type, p = o.stateNode;
              if (!(o.flags & 128) && (typeof c.getDerivedStateFromError == "function" || p !== null && typeof p.componentDidCatch == "function" && (zt === null || !zt.has(p)))) {
                o.flags |= 65536, t &= -t, o.lanes |= t;
                var v = Xa(o, s, t);
                As(o, v);
                break e;
              }
          }
          o = o.return;
        } while (o !== null);
      }
      pc(n);
    } catch (C) {
      t = C, b === n && n !== null && (b = n = n.return);
      continue;
    }
    break;
  } while (!0);
}
function dc() {
  var e = yl.current;
  return yl.current = hl, e === null ? hl : e;
}
function Ki() {
  (te === 0 || te === 3 || te === 2) && (te = 4), le === null || !(Yt & 268435455) && !(Nl & 268435455) || gt(le, ie);
}
function xl(e, t) {
  var n = M;
  M |= 2;
  var r = dc();
  (le !== e || ie !== t) && (rt = null, $t(e, t));
  do
    try {
      Lf();
      break;
    } catch (l) {
      cc(e, l);
    }
  while (!0);
  if (zi(), M = n, yl.current = r, b !== null) throw Error(w(261));
  return le = null, ie = 0, te;
}
function Lf() {
  for (; b !== null; ) fc(b);
}
function Of() {
  for (; b !== null && !rd(); ) fc(b);
}
function fc(e) {
  var t = hc(e.alternate, e, _e);
  e.memoizedProps = e.pendingProps, t === null ? pc(e) : b = t, Ui.current = null;
}
function pc(e) {
  var t = e;
  do {
    var n = t.alternate;
    if (e = t.return, t.flags & 32768) {
      if (n = _f(n, t), n !== null) {
        n.flags &= 32767, b = n;
        return;
      }
      if (e !== null) e.flags |= 32768, e.subtreeFlags = 0, e.deletions = null;
      else {
        te = 6, b = null;
        return;
      }
    } else if (n = jf(n, t, _e), n !== null) {
      b = n;
      return;
    }
    if (t = t.sibling, t !== null) {
      b = t;
      return;
    }
    b = t = e;
  } while (t !== null);
  te === 0 && (te = 5);
}
function Bt(e, t, n) {
  var r = W, l = Me.transition;
  try {
    Me.transition = null, W = 1, Df(e, t, n, r);
  } finally {
    Me.transition = l, W = r;
  }
  return null;
}
function Df(e, t, n, r) {
  do
    yn();
  while (wt !== null);
  if (M & 6) throw Error(w(327));
  n = e.finishedWork;
  var l = e.finishedLanes;
  if (n === null) return null;
  if (e.finishedWork = null, e.finishedLanes = 0, n === e.current) throw Error(w(177));
  e.callbackNode = null, e.callbackPriority = 0;
  var o = n.lanes | n.childLanes;
  if (pd(e, o), e === le && (b = le = null, ie = 0), !(n.subtreeFlags & 2064) && !(n.flags & 2064) || Mr || (Mr = !0, yc(el, function() {
    return yn(), null;
  })), o = (n.flags & 15990) !== 0, n.subtreeFlags & 15990 || o) {
    o = Me.transition, Me.transition = null;
    var i = W;
    W = 1;
    var s = M;
    M |= 4, Ui.current = null, Tf(e, n), sc(n, e), bd(Lo), nl = !!Ro, Lo = Ro = null, e.current = n, Pf(n), ld(), M = s, W = i, Me.transition = o;
  } else e.current = n;
  if (Mr && (Mr = !1, wt = e, gl = l), o = e.pendingLanes, o === 0 && (zt = null), sd(n.stateNode), je(e, q()), t !== null) for (r = e.onRecoverableError, n = 0; n < t.length; n++) l = t[n], r(l.value, { componentStack: l.stack, digest: l.digest });
  if (vl) throw vl = !1, e = qo, qo = null, e;
  return gl & 1 && e.tag !== 0 && yn(), o = e.pendingLanes, o & 1 ? e === bo ? qn++ : (qn = 0, bo = e) : qn = 0, Dt(), null;
}
function yn() {
  if (wt !== null) {
    var e = Yu(gl), t = Me.transition, n = W;
    try {
      if (Me.transition = null, W = 16 > e ? 16 : e, wt === null) var r = !1;
      else {
        if (e = wt, wt = null, gl = 0, M & 6) throw Error(w(331));
        var l = M;
        for (M |= 4, z = e.current; z !== null; ) {
          var o = z, i = o.child;
          if (z.flags & 16) {
            var s = o.deletions;
            if (s !== null) {
              for (var u = 0; u < s.length; u++) {
                var f = s[u];
                for (z = f; z !== null; ) {
                  var m = z;
                  switch (m.tag) {
                    case 0:
                    case 11:
                    case 15:
                      Zn(8, m, o);
                  }
                  var y = m.child;
                  if (y !== null) y.return = m, z = y;
                  else for (; z !== null; ) {
                    m = z;
                    var h = m.sibling, g = m.return;
                    if (lc(m), m === f) {
                      z = null;
                      break;
                    }
                    if (h !== null) {
                      h.return = g, z = h;
                      break;
                    }
                    z = g;
                  }
                }
              }
              var x = o.alternate;
              if (x !== null) {
                var S = x.child;
                if (S !== null) {
                  x.child = null;
                  do {
                    var O = S.sibling;
                    S.sibling = null, S = O;
                  } while (S !== null);
                }
              }
              z = o;
            }
          }
          if (o.subtreeFlags & 2064 && i !== null) i.return = o, z = i;
          else e: for (; z !== null; ) {
            if (o = z, o.flags & 2048) switch (o.tag) {
              case 0:
              case 11:
              case 15:
                Zn(9, o, o.return);
            }
            var d = o.sibling;
            if (d !== null) {
              d.return = o.return, z = d;
              break e;
            }
            z = o.return;
          }
        }
        var c = e.current;
        for (z = c; z !== null; ) {
          i = z;
          var p = i.child;
          if (i.subtreeFlags & 2064 && p !== null) p.return = i, z = p;
          else e: for (i = c; z !== null; ) {
            if (s = z, s.flags & 2048) try {
              switch (s.tag) {
                case 0:
                case 11:
                case 15:
                  Pl(9, s);
              }
            } catch (C) {
              J(s, s.return, C);
            }
            if (s === i) {
              z = null;
              break e;
            }
            var v = s.sibling;
            if (v !== null) {
              v.return = s.return, z = v;
              break e;
            }
            z = s.return;
          }
        }
        if (M = l, Dt(), tt && typeof tt.onPostCommitFiberRoot == "function") try {
          tt.onPostCommitFiberRoot(Sl, e);
        } catch {
        }
        r = !0;
      }
      return r;
    } finally {
      W = n, Me.transition = t;
    }
  }
  return !1;
}
function ru(e, t, n) {
  t = kn(n, t), t = Ya(e, t, 1), e = _t(e, t, 1), t = ye(), e !== null && (gr(e, 1, t), je(e, t));
}
function J(e, t, n) {
  if (e.tag === 3) ru(e, e, n);
  else for (; t !== null; ) {
    if (t.tag === 3) {
      ru(t, e, n);
      break;
    } else if (t.tag === 1) {
      var r = t.stateNode;
      if (typeof t.type.getDerivedStateFromError == "function" || typeof r.componentDidCatch == "function" && (zt === null || !zt.has(r))) {
        e = kn(n, e), e = Xa(t, e, 1), t = _t(t, e, 1), e = ye(), t !== null && (gr(t, 1, e), je(t, e));
        break;
      }
    }
    t = t.return;
  }
}
function If(e, t, n) {
  var r = e.pingCache;
  r !== null && r.delete(t), t = ye(), e.pingedLanes |= e.suspendedLanes & n, le === e && (ie & n) === n && (te === 4 || te === 3 && (ie & 130023424) === ie && 500 > q() - $i ? $t(e, 0) : Wi |= n), je(e, t);
}
function mc(e, t) {
  t === 0 && (e.mode & 1 ? (t = zr, zr <<= 1, !(zr & 130023424) && (zr = 4194304)) : t = 1);
  var n = ye();
  e = ct(e, t), e !== null && (gr(e, t, n), je(e, n));
}
function Af(e) {
  var t = e.memoizedState, n = 0;
  t !== null && (n = t.retryLane), mc(e, n);
}
function Mf(e, t) {
  var n = 0;
  switch (e.tag) {
    case 13:
      var r = e.stateNode, l = e.memoizedState;
      l !== null && (n = l.retryLane);
      break;
    case 19:
      r = e.stateNode;
      break;
    default:
      throw Error(w(314));
  }
  r !== null && r.delete(t), mc(e, n);
}
var hc;
hc = function(e, t, n) {
  if (e !== null) if (e.memoizedProps !== t.pendingProps || Ce.current) ke = !0;
  else {
    if (!(e.lanes & n) && !(t.flags & 128)) return ke = !1, Ef(e, t, n);
    ke = !!(e.flags & 131072);
  }
  else ke = !1, Y && t.flags & 1048576 && xa(t, al, t.index);
  switch (t.lanes = 0, t.tag) {
    case 2:
      var r = t.type;
      Yr(e, t), e = t.pendingProps;
      var l = gn(t, me.current);
      hn(t, n), l = Ii(null, t, r, e, l, n);
      var o = Ai();
      return t.flags |= 1, typeof l == "object" && l !== null && typeof l.render == "function" && l.$$typeof === void 0 ? (t.tag = 1, t.memoizedState = null, t.updateQueue = null, Ee(r) ? (o = !0, sl(t)) : o = !1, t.memoizedState = l.state !== null && l.state !== void 0 ? l.state : null, Ni(t), l.updater = Tl, t.stateNode = l, l._reactInternals = t, Wo(t, r, e, n), t = Ho(null, t, r, !0, o, n)) : (t.tag = 0, Y && o && Ci(t), he(null, t, l, n), t = t.child), t;
    case 16:
      r = t.elementType;
      e: {
        switch (Yr(e, t), e = t.pendingProps, l = r._init, r = l(r._payload), t.type = r, l = t.tag = Ff(r), e = He(r, e), l) {
          case 0:
            t = Vo(null, t, r, e, n);
            break e;
          case 1:
            t = Ys(null, t, r, e, n);
            break e;
          case 11:
            t = Ks(null, t, r, e, n);
            break e;
          case 14:
            t = Qs(null, t, r, He(r.type, e), n);
            break e;
        }
        throw Error(w(
          306,
          r,
          ""
        ));
      }
      return t;
    case 0:
      return r = t.type, l = t.pendingProps, l = t.elementType === r ? l : He(r, l), Vo(e, t, r, l, n);
    case 1:
      return r = t.type, l = t.pendingProps, l = t.elementType === r ? l : He(r, l), Ys(e, t, r, l, n);
    case 3:
      e: {
        if (qa(t), e === null) throw Error(w(387));
        r = t.pendingProps, o = t.memoizedState, l = o.element, ja(e, t), fl(t, r, null, n);
        var i = t.memoizedState;
        if (r = i.element, o.isDehydrated) if (o = { element: r, isDehydrated: !1, cache: i.cache, pendingSuspenseBoundaries: i.pendingSuspenseBoundaries, transitions: i.transitions }, t.updateQueue.baseState = o, t.memoizedState = o, t.flags & 256) {
          l = kn(Error(w(423)), t), t = Xs(e, t, r, n, l);
          break e;
        } else if (r !== l) {
          l = kn(Error(w(424)), t), t = Xs(e, t, r, n, l);
          break e;
        } else for (ze = jt(t.stateNode.containerInfo.firstChild), Te = t, Y = !0, Qe = null, n = Ca(t, null, r, n), t.child = n; n; ) n.flags = n.flags & -3 | 4096, n = n.sibling;
        else {
          if (xn(), r === l) {
            t = dt(e, t, n);
            break e;
          }
          he(e, t, r, n);
        }
        t = t.child;
      }
      return t;
    case 5:
      return _a(t), e === null && Bo(t), r = t.type, l = t.pendingProps, o = e !== null ? e.memoizedProps : null, i = l.children, Oo(r, l) ? i = null : o !== null && Oo(r, o) && (t.flags |= 32), Ja(e, t), he(e, t, i, n), t.child;
    case 6:
      return e === null && Bo(t), null;
    case 13:
      return ba(e, t, n);
    case 4:
      return Ri(t, t.stateNode.containerInfo), r = t.pendingProps, e === null ? t.child = wn(t, null, r, n) : he(e, t, r, n), t.child;
    case 11:
      return r = t.type, l = t.pendingProps, l = t.elementType === r ? l : He(r, l), Ks(e, t, r, l, n);
    case 7:
      return he(e, t, t.pendingProps, n), t.child;
    case 8:
      return he(e, t, t.pendingProps.children, n), t.child;
    case 12:
      return he(e, t, t.pendingProps.children, n), t.child;
    case 10:
      e: {
        if (r = t.type._context, l = t.pendingProps, o = t.memoizedProps, i = l.value, H(cl, r._currentValue), r._currentValue = i, o !== null) if (Ge(o.value, i)) {
          if (o.children === l.children && !Ce.current) {
            t = dt(e, t, n);
            break e;
          }
        } else for (o = t.child, o !== null && (o.return = t); o !== null; ) {
          var s = o.dependencies;
          if (s !== null) {
            i = o.child;
            for (var u = s.firstContext; u !== null; ) {
              if (u.context === r) {
                if (o.tag === 1) {
                  u = st(-1, n & -n), u.tag = 2;
                  var f = o.updateQueue;
                  if (f !== null) {
                    f = f.shared;
                    var m = f.pending;
                    m === null ? u.next = u : (u.next = m.next, m.next = u), f.pending = u;
                  }
                }
                o.lanes |= n, u = o.alternate, u !== null && (u.lanes |= n), Fo(
                  o.return,
                  n,
                  t
                ), s.lanes |= n;
                break;
              }
              u = u.next;
            }
          } else if (o.tag === 10) i = o.type === t.type ? null : o.child;
          else if (o.tag === 18) {
            if (i = o.return, i === null) throw Error(w(341));
            i.lanes |= n, s = i.alternate, s !== null && (s.lanes |= n), Fo(i, n, t), i = o.sibling;
          } else i = o.child;
          if (i !== null) i.return = o;
          else for (i = o; i !== null; ) {
            if (i === t) {
              i = null;
              break;
            }
            if (o = i.sibling, o !== null) {
              o.return = i.return, i = o;
              break;
            }
            i = i.return;
          }
          o = i;
        }
        he(e, t, l.children, n), t = t.child;
      }
      return t;
    case 9:
      return l = t.type, r = t.pendingProps.children, hn(t, n), l = Be(l), r = r(l), t.flags |= 1, he(e, t, r, n), t.child;
    case 14:
      return r = t.type, l = He(r, t.pendingProps), l = He(r.type, l), Qs(e, t, r, l, n);
    case 15:
      return Ga(e, t, t.type, t.pendingProps, n);
    case 17:
      return r = t.type, l = t.pendingProps, l = t.elementType === r ? l : He(r, l), Yr(e, t), t.tag = 1, Ee(r) ? (e = !0, sl(t)) : e = !1, hn(t, n), Qa(t, r, l), Wo(t, r, l, n), Ho(null, t, r, !0, e, n);
    case 19:
      return ec(e, t, n);
    case 22:
      return Za(e, t, n);
  }
  throw Error(w(156, t.tag));
};
function yc(e, t) {
  return Vu(e, t);
}
function Bf(e, t, n, r) {
  this.tag = e, this.key = n, this.sibling = this.child = this.return = this.stateNode = this.type = this.elementType = null, this.index = 0, this.ref = null, this.pendingProps = t, this.dependencies = this.memoizedState = this.updateQueue = this.memoizedProps = null, this.mode = r, this.subtreeFlags = this.flags = 0, this.deletions = null, this.childLanes = this.lanes = 0, this.alternate = null;
}
function Ae(e, t, n, r) {
  return new Bf(e, t, n, r);
}
function Qi(e) {
  return e = e.prototype, !(!e || !e.isReactComponent);
}
function Ff(e) {
  if (typeof e == "function") return Qi(e) ? 1 : 0;
  if (e != null) {
    if (e = e.$$typeof, e === di) return 11;
    if (e === fi) return 14;
  }
  return 2;
}
function Pt(e, t) {
  var n = e.alternate;
  return n === null ? (n = Ae(e.tag, t, e.key, e.mode), n.elementType = e.elementType, n.type = e.type, n.stateNode = e.stateNode, n.alternate = e, e.alternate = n) : (n.pendingProps = t, n.type = e.type, n.flags = 0, n.subtreeFlags = 0, n.deletions = null), n.flags = e.flags & 14680064, n.childLanes = e.childLanes, n.lanes = e.lanes, n.child = e.child, n.memoizedProps = e.memoizedProps, n.memoizedState = e.memoizedState, n.updateQueue = e.updateQueue, t = e.dependencies, n.dependencies = t === null ? null : { lanes: t.lanes, firstContext: t.firstContext }, n.sibling = e.sibling, n.index = e.index, n.ref = e.ref, n;
}
function Zr(e, t, n, r, l, o) {
  var i = 2;
  if (r = e, typeof e == "function") Qi(e) && (i = 1);
  else if (typeof e == "string") i = 5;
  else e: switch (e) {
    case bt:
      return Vt(n.children, l, o, t);
    case ci:
      i = 8, l |= 8;
      break;
    case co:
      return e = Ae(12, n, t, l | 2), e.elementType = co, e.lanes = o, e;
    case fo:
      return e = Ae(13, n, t, l), e.elementType = fo, e.lanes = o, e;
    case po:
      return e = Ae(19, n, t, l), e.elementType = po, e.lanes = o, e;
    case _u:
      return Rl(n, l, o, t);
    default:
      if (typeof e == "object" && e !== null) switch (e.$$typeof) {
        case Eu:
          i = 10;
          break e;
        case ju:
          i = 9;
          break e;
        case di:
          i = 11;
          break e;
        case fi:
          i = 14;
          break e;
        case mt:
          i = 16, r = null;
          break e;
      }
      throw Error(w(130, e == null ? e : typeof e, ""));
  }
  return t = Ae(i, n, t, l), t.elementType = e, t.type = r, t.lanes = o, t;
}
function Vt(e, t, n, r) {
  return e = Ae(7, e, r, t), e.lanes = n, e;
}
function Rl(e, t, n, r) {
  return e = Ae(22, e, r, t), e.elementType = _u, e.lanes = n, e.stateNode = { isHidden: !1 }, e;
}
function so(e, t, n) {
  return e = Ae(6, e, null, t), e.lanes = n, e;
}
function uo(e, t, n) {
  return t = Ae(4, e.children !== null ? e.children : [], e.key, t), t.lanes = n, t.stateNode = { containerInfo: e.containerInfo, pendingChildren: null, implementation: e.implementation }, t;
}
function Uf(e, t, n, r, l) {
  this.tag = t, this.containerInfo = e, this.finishedWork = this.pingCache = this.current = this.pendingChildren = null, this.timeoutHandle = -1, this.callbackNode = this.pendingContext = this.context = null, this.callbackPriority = 0, this.eventTimes = $l(0), this.expirationTimes = $l(-1), this.entangledLanes = this.finishedLanes = this.mutableReadLanes = this.expiredLanes = this.pingedLanes = this.suspendedLanes = this.pendingLanes = 0, this.entanglements = $l(0), this.identifierPrefix = r, this.onRecoverableError = l, this.mutableSourceEagerHydrationData = null;
}
function Yi(e, t, n, r, l, o, i, s, u) {
  return e = new Uf(e, t, n, s, u), t === 1 ? (t = 1, o === !0 && (t |= 8)) : t = 0, o = Ae(3, null, null, t), e.current = o, o.stateNode = e, o.memoizedState = { element: r, isDehydrated: n, cache: null, transitions: null, pendingSuspenseBoundaries: null }, Ni(o), e;
}
function Wf(e, t, n) {
  var r = 3 < arguments.length && arguments[3] !== void 0 ? arguments[3] : null;
  return { $$typeof: qt, key: r == null ? null : "" + r, children: e, containerInfo: t, implementation: n };
}
function vc(e) {
  if (!e) return Rt;
  e = e._reactInternals;
  e: {
    if (Zt(e) !== e || e.tag !== 1) throw Error(w(170));
    var t = e;
    do {
      switch (t.tag) {
        case 3:
          t = t.stateNode.context;
          break e;
        case 1:
          if (Ee(t.type)) {
            t = t.stateNode.__reactInternalMemoizedMergedChildContext;
            break e;
          }
      }
      t = t.return;
    } while (t !== null);
    throw Error(w(171));
  }
  if (e.tag === 1) {
    var n = e.type;
    if (Ee(n)) return va(e, n, t);
  }
  return t;
}
function gc(e, t, n, r, l, o, i, s, u) {
  return e = Yi(n, r, !0, e, l, o, i, s, u), e.context = vc(null), n = e.current, r = ye(), l = Tt(n), o = st(r, l), o.callback = t ?? null, _t(n, o, l), e.current.lanes = l, gr(e, l, r), je(e, r), e;
}
function Ll(e, t, n, r) {
  var l = t.current, o = ye(), i = Tt(l);
  return n = vc(n), t.context === null ? t.context = n : t.pendingContext = n, t = st(o, i), t.payload = { element: e }, r = r === void 0 ? null : r, r !== null && (t.callback = r), e = _t(l, t, i), e !== null && (Xe(e, l, i, o), Hr(e, l, i)), i;
}
function wl(e) {
  if (e = e.current, !e.child) return null;
  switch (e.child.tag) {
    case 5:
      return e.child.stateNode;
    default:
      return e.child.stateNode;
  }
}
function lu(e, t) {
  if (e = e.memoizedState, e !== null && e.dehydrated !== null) {
    var n = e.retryLane;
    e.retryLane = n !== 0 && n < t ? n : t;
  }
}
function Xi(e, t) {
  lu(e, t), (e = e.alternate) && lu(e, t);
}
function $f() {
  return null;
}
var xc = typeof reportError == "function" ? reportError : function(e) {
  console.error(e);
};
function Gi(e) {
  this._internalRoot = e;
}
Ol.prototype.render = Gi.prototype.render = function(e) {
  var t = this._internalRoot;
  if (t === null) throw Error(w(409));
  Ll(e, t, null, null);
};
Ol.prototype.unmount = Gi.prototype.unmount = function() {
  var e = this._internalRoot;
  if (e !== null) {
    this._internalRoot = null;
    var t = e.containerInfo;
    Xt(function() {
      Ll(null, e, null, null);
    }), t[at] = null;
  }
};
function Ol(e) {
  this._internalRoot = e;
}
Ol.prototype.unstable_scheduleHydration = function(e) {
  if (e) {
    var t = Zu();
    e = { blockedOn: null, target: e, priority: t };
    for (var n = 0; n < vt.length && t !== 0 && t < vt[n].priority; n++) ;
    vt.splice(n, 0, e), n === 0 && qu(e);
  }
};
function Zi(e) {
  return !(!e || e.nodeType !== 1 && e.nodeType !== 9 && e.nodeType !== 11);
}
function Dl(e) {
  return !(!e || e.nodeType !== 1 && e.nodeType !== 9 && e.nodeType !== 11 && (e.nodeType !== 8 || e.nodeValue !== " react-mount-point-unstable "));
}
function ou() {
}
function Vf(e, t, n, r, l) {
  if (l) {
    if (typeof r == "function") {
      var o = r;
      r = function() {
        var f = wl(i);
        o.call(f);
      };
    }
    var i = gc(t, r, e, 0, null, !1, !1, "", ou);
    return e._reactRootContainer = i, e[at] = i.current, ur(e.nodeType === 8 ? e.parentNode : e), Xt(), i;
  }
  for (; l = e.lastChild; ) e.removeChild(l);
  if (typeof r == "function") {
    var s = r;
    r = function() {
      var f = wl(u);
      s.call(f);
    };
  }
  var u = Yi(e, 0, !1, null, null, !1, !1, "", ou);
  return e._reactRootContainer = u, e[at] = u.current, ur(e.nodeType === 8 ? e.parentNode : e), Xt(function() {
    Ll(t, u, n, r);
  }), u;
}
function Il(e, t, n, r, l) {
  var o = n._reactRootContainer;
  if (o) {
    var i = o;
    if (typeof l == "function") {
      var s = l;
      l = function() {
        var u = wl(i);
        s.call(u);
      };
    }
    Ll(t, i, e, l);
  } else i = Vf(n, t, e, l, r);
  return wl(i);
}
Xu = function(e) {
  switch (e.tag) {
    case 3:
      var t = e.stateNode;
      if (t.current.memoizedState.isDehydrated) {
        var n = $n(t.pendingLanes);
        n !== 0 && (hi(t, n | 1), je(t, q()), !(M & 6) && (Cn = q() + 500, Dt()));
      }
      break;
    case 13:
      Xt(function() {
        var r = ct(e, 1);
        if (r !== null) {
          var l = ye();
          Xe(r, e, 1, l);
        }
      }), Xi(e, 1);
  }
};
yi = function(e) {
  if (e.tag === 13) {
    var t = ct(e, 134217728);
    if (t !== null) {
      var n = ye();
      Xe(t, e, 134217728, n);
    }
    Xi(e, 134217728);
  }
};
Gu = function(e) {
  if (e.tag === 13) {
    var t = Tt(e), n = ct(e, t);
    if (n !== null) {
      var r = ye();
      Xe(n, e, t, r);
    }
    Xi(e, t);
  }
};
Zu = function() {
  return W;
};
Ju = function(e, t) {
  var n = W;
  try {
    return W = e, t();
  } finally {
    W = n;
  }
};
Co = function(e, t, n) {
  switch (t) {
    case "input":
      if (yo(e, n), t = n.name, n.type === "radio" && t != null) {
        for (n = e; n.parentNode; ) n = n.parentNode;
        for (n = n.querySelectorAll("input[name=" + JSON.stringify("" + t) + '][type="radio"]'), t = 0; t < n.length; t++) {
          var r = n[t];
          if (r !== e && r.form === e.form) {
            var l = jl(r);
            if (!l) throw Error(w(90));
            Tu(r), yo(r, l);
          }
        }
      }
      break;
    case "textarea":
      Nu(e, n);
      break;
    case "select":
      t = n.value, t != null && dn(e, !!n.multiple, t, !1);
  }
};
Mu = Vi;
Bu = Xt;
var Hf = { usingClientEntryPoint: !1, Events: [wr, rn, jl, Iu, Au, Vi] }, Fn = { findFiberByHostInstance: Ft, bundleType: 0, version: "18.3.1", rendererPackageName: "react-dom" }, Kf = { bundleType: Fn.bundleType, version: Fn.version, rendererPackageName: Fn.rendererPackageName, rendererConfig: Fn.rendererConfig, overrideHookState: null, overrideHookStateDeletePath: null, overrideHookStateRenamePath: null, overrideProps: null, overridePropsDeletePath: null, overridePropsRenamePath: null, setErrorHandler: null, setSuspenseHandler: null, scheduleUpdate: null, currentDispatcherRef: ft.ReactCurrentDispatcher, findHostInstanceByFiber: function(e) {
  return e = Wu(e), e === null ? null : e.stateNode;
}, findFiberByHostInstance: Fn.findFiberByHostInstance || $f, findHostInstancesForRefresh: null, scheduleRefresh: null, scheduleRoot: null, setRefreshHandler: null, getCurrentFiber: null, reconcilerVersion: "18.3.1-next-f1338f8080-20240426" };
if (typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ < "u") {
  var Br = __REACT_DEVTOOLS_GLOBAL_HOOK__;
  if (!Br.isDisabled && Br.supportsFiber) try {
    Sl = Br.inject(Kf), tt = Br;
  } catch {
  }
}
Ne.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED = Hf;
Ne.createPortal = function(e, t) {
  var n = 2 < arguments.length && arguments[2] !== void 0 ? arguments[2] : null;
  if (!Zi(t)) throw Error(w(200));
  return Wf(e, t, null, n);
};
Ne.createRoot = function(e, t) {
  if (!Zi(e)) throw Error(w(299));
  var n = !1, r = "", l = xc;
  return t != null && (t.unstable_strictMode === !0 && (n = !0), t.identifierPrefix !== void 0 && (r = t.identifierPrefix), t.onRecoverableError !== void 0 && (l = t.onRecoverableError)), t = Yi(e, 1, !1, null, null, n, !1, r, l), e[at] = t.current, ur(e.nodeType === 8 ? e.parentNode : e), new Gi(t);
};
Ne.findDOMNode = function(e) {
  if (e == null) return null;
  if (e.nodeType === 1) return e;
  var t = e._reactInternals;
  if (t === void 0)
    throw typeof e.render == "function" ? Error(w(188)) : (e = Object.keys(e).join(","), Error(w(268, e)));
  return e = Wu(t), e = e === null ? null : e.stateNode, e;
};
Ne.flushSync = function(e) {
  return Xt(e);
};
Ne.hydrate = function(e, t, n) {
  if (!Dl(t)) throw Error(w(200));
  return Il(null, e, t, !0, n);
};
Ne.hydrateRoot = function(e, t, n) {
  if (!Zi(e)) throw Error(w(405));
  var r = n != null && n.hydratedSources || null, l = !1, o = "", i = xc;
  if (n != null && (n.unstable_strictMode === !0 && (l = !0), n.identifierPrefix !== void 0 && (o = n.identifierPrefix), n.onRecoverableError !== void 0 && (i = n.onRecoverableError)), t = gc(t, null, e, 1, n ?? null, l, !1, o, i), e[at] = t.current, ur(e), r) for (e = 0; e < r.length; e++) n = r[e], l = n._getVersion, l = l(n._source), t.mutableSourceEagerHydrationData == null ? t.mutableSourceEagerHydrationData = [n, l] : t.mutableSourceEagerHydrationData.push(
    n,
    l
  );
  return new Ol(t);
};
Ne.render = function(e, t, n) {
  if (!Dl(t)) throw Error(w(200));
  return Il(null, e, t, !1, n);
};
Ne.unmountComponentAtNode = function(e) {
  if (!Dl(e)) throw Error(w(40));
  return e._reactRootContainer ? (Xt(function() {
    Il(null, null, e, !1, function() {
      e._reactRootContainer = null, e[at] = null;
    });
  }), !0) : !1;
};
Ne.unstable_batchedUpdates = Vi;
Ne.unstable_renderSubtreeIntoContainer = function(e, t, n, r) {
  if (!Dl(n)) throw Error(w(200));
  if (e == null || e._reactInternals === void 0) throw Error(w(38));
  return Il(e, t, n, !1, r);
};
Ne.version = "18.3.1-next-f1338f8080-20240426";
function wc() {
  if (!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ > "u" || typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE != "function"))
    try {
      __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(wc);
    } catch (e) {
      console.error(e);
    }
}
wc(), wu.exports = Ne;
var Qf = wu.exports, iu = Qf;
bn.createRoot = iu.createRoot, bn.hydrateRoot = iu.hydrateRoot;
var Sc = { exports: {} }, Al = {};
/**
 * @license React
 * react-jsx-runtime.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */
var Yf = I, Xf = Symbol.for("react.element"), Gf = Symbol.for("react.fragment"), Zf = Object.prototype.hasOwnProperty, Jf = Yf.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED.ReactCurrentOwner, qf = { key: !0, ref: !0, __self: !0, __source: !0 };
function kc(e, t, n) {
  var r, l = {}, o = null, i = null;
  n !== void 0 && (o = "" + n), t.key !== void 0 && (o = "" + t.key), t.ref !== void 0 && (i = t.ref);
  for (r in t) Zf.call(t, r) && !qf.hasOwnProperty(r) && (l[r] = t[r]);
  if (e && e.defaultProps) for (r in t = e.defaultProps, t) l[r] === void 0 && (l[r] = t[r]);
  return { $$typeof: Xf, type: e, key: o, ref: i, props: l, _owner: Jf.current };
}
Al.Fragment = Gf;
Al.jsx = kc;
Al.jsxs = kc;
Sc.exports = Al;
var a = Sc.exports;
const yt = class yt {
  constructor(t = "*") {
    Oe(this, "listeners", /* @__PURE__ */ new Map());
    Oe(this, "targetOrigin");
    this.targetOrigin = t;
  }
  static getInstance(t) {
    return yt.instance || (yt.instance = new yt(t)), yt.instance;
  }
  /** Reset singleton - used in tests */
  static resetInstance() {
    yt.instance = void 0;
  }
  emit(t, n) {
    const r = { type: t, payload: n, timestamp: Date.now() }, l = this.listeners.get(t);
    l && l.forEach((o) => {
      try {
        o(n);
      } catch (i) {
        console.error(`[RampOS] Error in event handler for ${t}:`, i);
      }
    }), typeof window < "u" && window.parent && window.parent !== window && window.parent.postMessage({ source: "rampos-widget", event: r }, this.targetOrigin), typeof window < "u" && window.dispatchEvent(
      new CustomEvent(`rampos:${t.toLowerCase()}`, {
        detail: { ...r },
        bubbles: !0,
        composed: !0
      })
    );
  }
  on(t, n) {
    return this.listeners.has(t) || this.listeners.set(t, /* @__PURE__ */ new Set()), this.listeners.get(t).add(n), () => {
      var r;
      (r = this.listeners.get(t)) == null || r.delete(n);
    };
  }
  off(t, n) {
    var r;
    (r = this.listeners.get(t)) == null || r.delete(n);
  }
  removeAllListeners(t) {
    t ? this.listeners.delete(t) : this.listeners.clear();
  }
};
Oe(yt, "instance");
let En = yt;
function bf(e, t) {
  const n = (r) => {
    if (t != null && t.origin && r.origin !== t.origin) return;
    const l = r.data;
    (l == null ? void 0 : l.source) === "rampos-widget" && l.event && e(l.event);
  };
  return window.addEventListener("message", n), () => window.removeEventListener("message", n);
}
const At = {
  primaryColor: "#2563eb",
  backgroundColor: "#ffffff",
  textColor: "#1f2937",
  borderRadius: "8px",
  fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  errorColor: "#ef4444",
  successColor: "#10b981"
};
function Ji(e) {
  return {
    primaryColor: (e == null ? void 0 : e.primaryColor) ?? At.primaryColor,
    backgroundColor: (e == null ? void 0 : e.backgroundColor) ?? At.backgroundColor,
    textColor: (e == null ? void 0 : e.textColor) ?? At.textColor,
    borderRadius: (e == null ? void 0 : e.borderRadius) ?? At.borderRadius,
    fontFamily: (e == null ? void 0 : e.fontFamily) ?? At.fontFamily,
    errorColor: (e == null ? void 0 : e.errorColor) ?? At.errorColor,
    successColor: (e == null ? void 0 : e.successColor) ?? At.successColor
  };
}
const V = ({
  variant: e = "primary",
  fullWidth: t = !0,
  loading: n = !1,
  primaryColor: r = "#2563eb",
  children: l,
  disabled: o,
  style: i,
  ...s
}) => {
  const u = {
    border: "none",
    borderRadius: "6px",
    padding: "10px 16px",
    fontSize: "14px",
    fontWeight: 500,
    cursor: o || n ? "not-allowed" : "pointer",
    width: t ? "100%" : "auto",
    transition: "background-color 0.2s, opacity 0.2s",
    opacity: o || n ? 0.6 : 1,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    gap: "8px"
  }, f = {
    primary: {
      backgroundColor: r,
      color: "#ffffff"
    },
    secondary: {
      backgroundColor: "transparent",
      color: "#6b7280",
      border: "1px solid #d1d5db"
    },
    ghost: {
      backgroundColor: "transparent",
      color: r
    }
  };
  return /* @__PURE__ */ a.jsxs(
    "button",
    {
      style: { ...u, ...f[e], ...i },
      disabled: o || n,
      ...s,
      children: [
        n && /* @__PURE__ */ a.jsx("span", { style: {
          display: "inline-block",
          width: "14px",
          height: "14px",
          border: "2px solid currentColor",
          borderTopColor: "transparent",
          borderRadius: "50%",
          animation: "rampos-spin 0.6s linear infinite"
        } }),
        l
      ]
    }
  );
}, St = ({
  label: e,
  error: t,
  helpText: n,
  style: r,
  id: l,
  ...o
}) => {
  const i = l || `rampos-input-${e == null ? void 0 : e.toLowerCase().replace(/\s+/g, "-")}`;
  return /* @__PURE__ */ a.jsxs("div", { style: { marginBottom: "12px" }, children: [
    e && /* @__PURE__ */ a.jsx(
      "label",
      {
        htmlFor: i,
        style: {
          display: "block",
          fontSize: "14px",
          fontWeight: 500,
          marginBottom: "4px",
          color: "#374151"
        },
        children: e
      }
    ),
    /* @__PURE__ */ a.jsx(
      "input",
      {
        id: i,
        style: {
          width: "100%",
          padding: "8px 12px",
          border: `1px solid ${t ? "#ef4444" : "#d1d5db"}`,
          borderRadius: "6px",
          fontSize: "14px",
          outline: "none",
          boxSizing: "border-box",
          transition: "border-color 0.2s",
          ...r
        },
        ...o
      }
    ),
    t && /* @__PURE__ */ a.jsx("div", { style: { color: "#ef4444", fontSize: "12px", marginTop: "4px" }, children: t }),
    n && !t && /* @__PURE__ */ a.jsx("div", { style: { color: "#9ca3af", fontSize: "12px", marginTop: "4px" }, children: n })
  ] });
}, su = [
  { value: "USDC", label: "USDC", network: "polygon" },
  { value: "USDT", label: "USDT", network: "polygon" },
  { value: "ETH", label: "Ethereum", network: "arbitrum" },
  { value: "MATIC", label: "MATIC", network: "polygon" },
  { value: "VND_TOKEN", label: "VND Token", network: "polygon" }
], uu = [
  { value: "bank_transfer", label: "Bank Transfer" },
  { value: "card", label: "Credit / Debit Card" },
  { value: "mobile_money", label: "Mobile Money (MoMo, ZaloPay)" }
], ep = ({
  apiKey: e,
  amount: t,
  asset: n,
  network: r,
  walletAddress: l,
  theme: o,
  onSuccess: i,
  onError: s,
  onClose: u,
  onReady: f
}) => {
  const m = Ji(o), y = En.getInstance(), [h, g] = I.useState(() => n && t ? "payment-method" : n ? "enter-amount" : "select-asset"), [x, S] = I.useState(n || ""), [O, d] = I.useState(r || ""), [c, p] = I.useState(t || 0), [v, C] = I.useState(l || ""), [j, T] = I.useState(""), [_, A] = I.useState(null), [N, ne] = I.useState(!1);
  I.useEffect(() => {
    y.emit("CHECKOUT_READY"), f == null || f();
  }, []);
  const ue = I.useCallback(() => {
    y.emit("CHECKOUT_CLOSE"), u == null || u();
  }, [y, u]), ae = (L) => {
    S(L);
    const E = su.find((F) => F.value === L);
    E && d(E.network), g("enter-amount");
  }, Ue = () => {
    if (c <= 0) {
      A("Please enter a valid amount");
      return;
    }
    A(null), c > 1e3 && !N ? g("kyc-check") : g("payment-method");
  }, It = () => {
    ne(!0), g("payment-method");
  }, We = (L) => {
    T(L), g("summary");
  }, $e = async () => {
    g("processing"), A(null);
    try {
      await new Promise((E) => setTimeout(E, 2e3));
      const L = {
        transactionId: `tx_${Date.now().toString(36)}_${Math.random().toString(36).substring(2, 8)}`,
        status: "success",
        amount: c,
        asset: x,
        network: O,
        walletAddress: v,
        timestamp: Date.now()
      };
      g("success"), y.emit("CHECKOUT_SUCCESS", L), i == null || i(L);
    } catch (L) {
      const E = L instanceof Error ? L.message : "Transaction failed";
      A(E), g("failed"), y.emit("CHECKOUT_ERROR", { message: E }), s == null || s(L instanceof Error ? L : new Error(E));
    }
  }, k = {
    fontFamily: m.fontFamily,
    padding: "24px",
    borderRadius: m.borderRadius,
    backgroundColor: m.backgroundColor,
    color: m.textColor,
    boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)",
    maxWidth: "420px",
    width: "100%",
    position: "relative"
  }, P = {
    fontSize: "18px",
    fontWeight: 600,
    marginBottom: "20px",
    borderBottom: "1px solid #e5e7eb",
    paddingBottom: "12px",
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center"
  }, R = (L) => ({
    padding: "12px 16px",
    border: `2px solid ${L ? m.primaryColor : "#e5e7eb"}`,
    borderRadius: "8px",
    marginBottom: "8px",
    cursor: "pointer",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    backgroundColor: L ? `${m.primaryColor}08` : "#fff",
    transition: "all 0.15s"
  }), $ = (L) => ({
    ...R(L)
  }), U = {
    display: "flex",
    justifyContent: "space-between",
    marginBottom: "8px",
    fontSize: "14px",
    color: "#4b5563"
  }, Ze = {
    color: m.errorColor,
    fontSize: "13px",
    padding: "8px 12px",
    backgroundColor: "#fee2e2",
    borderRadius: "6px",
    marginBottom: "12px"
  }, xe = () => /* @__PURE__ */ a.jsxs("div", { children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Select an asset to purchase" }),
    su.map((L) => /* @__PURE__ */ a.jsxs(
      "div",
      {
        style: R(x === L.value),
        onClick: () => ae(L.value),
        role: "button",
        tabIndex: 0,
        onKeyDown: (E) => {
          E.key === "Enter" && ae(L.value);
        },
        children: [
          /* @__PURE__ */ a.jsxs("div", { children: [
            /* @__PURE__ */ a.jsx("div", { style: { fontWeight: 600 }, children: L.label }),
            /* @__PURE__ */ a.jsx("div", { style: { fontSize: "12px", color: "#9ca3af" }, children: L.network })
          ] }),
          x === L.value && /* @__PURE__ */ a.jsx("span", { style: { color: m.primaryColor, fontWeight: 700 }, children: "✓" })
        ]
      },
      L.value
    ))
  ] }), Le = () => /* @__PURE__ */ a.jsxs("div", { children: [
    /* @__PURE__ */ a.jsx(
      St,
      {
        label: `Amount (${x})`,
        type: "number",
        value: c || "",
        onChange: (L) => p(parseFloat(L.target.value) || 0),
        placeholder: "0.00",
        min: "0",
        error: _ || void 0
      }
    ),
    /* @__PURE__ */ a.jsx(
      St,
      {
        label: "Wallet Address",
        type: "text",
        value: v,
        onChange: (L) => C(L.target.value),
        placeholder: "0x...",
        helpText: "Your receiving wallet address"
      }
    ),
    /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", gap: "8px", marginTop: "8px" }, children: [
      /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("select-asset"), primaryColor: m.primaryColor, children: "Back" }),
      /* @__PURE__ */ a.jsx(V, { onClick: Ue, primaryColor: m.primaryColor, children: "Continue" })
    ] })
  ] }), ce = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "24px", marginBottom: "12px" }, children: "ID" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Identity Verification Required" }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "Transactions over $1,000 require KYC verification. This is a quick process." }),
    /* @__PURE__ */ a.jsx(V, { onClick: It, primaryColor: m.primaryColor, children: "Complete Verification" }),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "ghost", onClick: () => g("enter-amount"), primaryColor: m.primaryColor, children: "Go Back" }) })
  ] }), Je = () => /* @__PURE__ */ a.jsxs("div", { children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Select payment method" }),
    uu.map((L) => /* @__PURE__ */ a.jsxs(
      "div",
      {
        style: $(j === L.value),
        onClick: () => We(L.value),
        role: "button",
        tabIndex: 0,
        onKeyDown: (E) => {
          E.key === "Enter" && We(L.value);
        },
        children: [
          /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 500 }, children: L.label }),
          j === L.value && /* @__PURE__ */ a.jsx("span", { style: { color: m.primaryColor, fontWeight: 700 }, children: "✓" })
        ]
      },
      L.value
    )),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("enter-amount"), primaryColor: m.primaryColor, children: "Back" }) })
  ] }), Tn = () => {
    var L;
    return /* @__PURE__ */ a.jsxs("div", { children: [
      /* @__PURE__ */ a.jsxs("div", { style: { marginBottom: "16px" }, children: [
        /* @__PURE__ */ a.jsxs("div", { style: U, children: [
          /* @__PURE__ */ a.jsx("span", { children: "Asset" }),
          /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 600 }, children: x })
        ] }),
        /* @__PURE__ */ a.jsxs("div", { style: U, children: [
          /* @__PURE__ */ a.jsx("span", { children: "Network" }),
          /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 600 }, children: O })
        ] }),
        /* @__PURE__ */ a.jsxs("div", { style: U, children: [
          /* @__PURE__ */ a.jsx("span", { children: "Amount" }),
          /* @__PURE__ */ a.jsxs("span", { style: { fontWeight: 600 }, children: [
            c,
            " ",
            x
          ] })
        ] }),
        /* @__PURE__ */ a.jsxs("div", { style: U, children: [
          /* @__PURE__ */ a.jsx("span", { children: "Payment" }),
          /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 600 }, children: (L = uu.find((E) => E.value === j)) == null ? void 0 : L.label })
        ] }),
        v && /* @__PURE__ */ a.jsxs("div", { style: U, children: [
          /* @__PURE__ */ a.jsx("span", { children: "Wallet" }),
          /* @__PURE__ */ a.jsxs("span", { style: { fontWeight: 600, fontSize: "12px", wordBreak: "break-all" }, children: [
            v.substring(0, 6),
            "...",
            v.substring(v.length - 4)
          ] })
        ] }),
        /* @__PURE__ */ a.jsxs("div", { style: { ...U, borderTop: "1px solid #e5e7eb", paddingTop: "8px", marginTop: "8px", fontWeight: 600 }, children: [
          /* @__PURE__ */ a.jsx("span", { children: "Total" }),
          /* @__PURE__ */ a.jsxs("span", { children: [
            c,
            " ",
            x
          ] })
        ] })
      ] }),
      /* @__PURE__ */ a.jsx(V, { onClick: $e, primaryColor: m.primaryColor, children: "Confirm Payment" }),
      /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("payment-method"), primaryColor: m.primaryColor, children: "Back" }) })
    ] });
  }, Pn = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "24px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: {
      width: "44px",
      height: "44px",
      border: `3px solid ${m.primaryColor}`,
      borderTopColor: "transparent",
      borderRadius: "50%",
      margin: "0 auto 16px",
      animation: "rampos-spin 0.8s linear infinite"
    } }),
    /* @__PURE__ */ a.jsx("div", { style: { fontWeight: 500, color: "#374151" }, children: "Processing your transaction..." }),
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "13px", color: "#9ca3af", marginTop: "4px" }, children: "This may take a moment" }),
    /* @__PURE__ */ a.jsx("style", { children: "@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }" })
  ] }), Nn = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { color: m.successColor, fontSize: "48px", marginBottom: "8px" }, children: "✓" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Payment Successful!" }),
    /* @__PURE__ */ a.jsxs("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: [
      "Your ",
      c,
      " ",
      x,
      " purchase has been processed."
    ] }),
    /* @__PURE__ */ a.jsx(V, { onClick: ue, primaryColor: m.primaryColor, children: "Done" })
  ] }), Rn = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { color: m.errorColor, fontSize: "48px", marginBottom: "8px" }, children: "✗" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Payment Failed" }),
    _ && /* @__PURE__ */ a.jsx("div", { style: Ze, children: _ }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "Something went wrong. Please try again." }),
    /* @__PURE__ */ a.jsx(V, { onClick: () => g("summary"), primaryColor: m.primaryColor, children: "Try Again" }),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "ghost", onClick: ue, primaryColor: m.primaryColor, children: "Cancel" }) })
  ] });
  return /* @__PURE__ */ a.jsxs("div", { style: k, "data-testid": "rampos-checkout", children: [
    /* @__PURE__ */ a.jsxs("div", { style: P, children: [
      /* @__PURE__ */ a.jsx("span", { children: "RampOS Checkout" }),
      /* @__PURE__ */ a.jsx(
        "button",
        {
          onClick: ue,
          style: { background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#9ca3af" },
          "aria-label": "Close",
          children: "x"
        }
      )
    ] }),
    h === "select-asset" && xe(),
    h === "enter-amount" && Le(),
    h === "kyc-check" && ce(),
    h === "payment-method" && Je(),
    h === "summary" && Tn(),
    h === "processing" && Pn(),
    h === "success" && Nn(),
    h === "failed" && Rn(),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "20px", textAlign: "center", fontSize: "11px", color: "#9ca3af" }, children: "Powered by RampOS" })
  ] });
};
class tp extends HTMLElement {
  constructor() {
    super();
    Oe(this, "root", null);
    Oe(this, "mountPoint");
    this.attachShadow({ mode: "open" }), this.mountPoint = document.createElement("div"), this.shadowRoot.appendChild(this.mountPoint);
  }
  static get observedAttributes() {
    return [
      "api-key",
      "amount",
      "asset",
      "network",
      "wallet-address",
      "fiat-currency",
      "environment",
      "theme-primary",
      "theme-bg",
      "theme-text",
      "theme-radius",
      "theme-font"
    ];
  }
  connectedCallback() {
    this.renderComponent();
  }
  attributeChangedCallback() {
    this.renderComponent();
  }
  disconnectedCallback() {
    this.root && (this.root.unmount(), this.root = null);
  }
  getTheme() {
    return {
      primaryColor: this.getAttribute("theme-primary") || void 0,
      backgroundColor: this.getAttribute("theme-bg") || void 0,
      textColor: this.getAttribute("theme-text") || void 0,
      borderRadius: this.getAttribute("theme-radius") || void 0,
      fontFamily: this.getAttribute("theme-font") || void 0
    };
  }
  renderComponent() {
    const n = this.getAttribute("api-key");
    if (!n) {
      console.error("[RampOS] api-key attribute is required for <rampos-checkout>");
      return;
    }
    const r = this.getAttribute("amount"), l = {
      apiKey: n,
      amount: r ? parseFloat(r) : void 0,
      asset: this.getAttribute("asset") || void 0,
      network: this.getAttribute("network") || void 0,
      walletAddress: this.getAttribute("wallet-address") || void 0,
      fiatCurrency: this.getAttribute("fiat-currency") || void 0,
      environment: this.getAttribute("environment") || "sandbox",
      theme: this.getTheme(),
      onSuccess: (o) => {
        this.dispatchEvent(new CustomEvent("rampos-success", { detail: o, bubbles: !0, composed: !0 }));
      },
      onError: (o) => {
        this.dispatchEvent(new CustomEvent("rampos-error", { detail: o, bubbles: !0, composed: !0 }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent("rampos-close", { bubbles: !0, composed: !0 }));
      },
      onReady: () => {
        this.dispatchEvent(new CustomEvent("rampos-ready", { bubbles: !0, composed: !0 }));
      }
    };
    this.root || (this.root = bn.createRoot(this.mountPoint)), this.root.render(ii.createElement(ep, l));
  }
}
typeof customElements < "u" && !customElements.get("rampos-checkout") && customElements.define("rampos-checkout", tp);
const np = ({
  apiKey: e,
  userId: t,
  level: n = "basic",
  theme: r,
  onSubmitted: l,
  onApproved: o,
  onRejected: i,
  onError: s,
  onClose: u,
  onReady: f
}) => {
  const m = Ji(r), y = En.getInstance(), [h, g] = I.useState("intro"), [x, S] = I.useState(""), [O, d] = I.useState(""), [c, p] = I.useState(""), [v, C] = I.useState(""), [j, T] = I.useState("national_id"), [_, A] = I.useState(!1), [N, ne] = I.useState(!1), [ue, ae] = I.useState(null);
  I.useEffect(() => {
    y.emit("KYC_READY"), f == null || f();
  }, []);
  const Ue = I.useCallback(() => {
    y.emit("KYC_CLOSE"), u == null || u();
  }, [y, u]), It = () => {
    if (!x || !O || !c) {
      ae("Please fill in all required fields");
      return;
    }
    ae(null), g("document-upload");
  }, We = () => {
    A(!0), g(n === "basic" ? "review" : "selfie");
  }, $e = () => {
    ne(!0), g("review");
  }, k = async () => {
    g("submitting"), ae(null);
    try {
      await new Promise((we) => setTimeout(we, 2e3));
      const B = {
        userId: t || `user_${Date.now().toString(36)}`,
        status: "pending",
        level: n,
        verifiedAt: void 0
      };
      g("submitted"), y.emit("KYC_SUBMITTED", B), l == null || l(B), setTimeout(() => {
        const we = {
          ...B,
          status: "approved",
          verifiedAt: Date.now(),
          expiresAt: Date.now() + 31536e6
        };
        g("approved"), y.emit("KYC_APPROVED", we), o == null || o(we);
      }, 3e3);
    } catch (B) {
      const we = B instanceof Error ? B.message : "KYC submission failed";
      ae(we), g("intro"), y.emit("KYC_ERROR", { message: we }), s == null || s(B instanceof Error ? B : new Error(we));
    }
  }, P = {
    fontFamily: m.fontFamily,
    padding: "24px",
    borderRadius: m.borderRadius,
    backgroundColor: m.backgroundColor,
    color: m.textColor,
    boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
    maxWidth: "420px",
    width: "100%"
  }, R = {
    fontSize: "18px",
    fontWeight: 600,
    marginBottom: "20px",
    borderBottom: "1px solid #e5e7eb",
    paddingBottom: "12px",
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center"
  }, $ = {
    color: m.errorColor,
    fontSize: "13px",
    padding: "8px 12px",
    backgroundColor: "#fee2e2",
    borderRadius: "6px",
    marginBottom: "12px"
  }, U = {
    display: "flex",
    gap: "4px",
    marginBottom: "20px"
  }, Ze = n === "basic" ? ["Info", "Document", "Review"] : ["Info", "Document", "Selfie", "Review"], xe = (() => {
    switch (h) {
      case "intro":
        return -1;
      case "personal-info":
        return 0;
      case "document-upload":
        return 1;
      case "selfie":
        return 2;
      case "review":
        return n === "basic" ? 2 : 3;
      default:
        return -1;
    }
  })(), Le = () => /* @__PURE__ */ a.jsx("div", { style: U, children: Ze.map((B, we) => /* @__PURE__ */ a.jsxs("div", { style: { flex: 1, textAlign: "center" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: {
      height: "4px",
      borderRadius: "2px",
      backgroundColor: we <= xe ? m.primaryColor : "#e5e7eb",
      marginBottom: "4px",
      transition: "background-color 0.2s"
    } }),
    /* @__PURE__ */ a.jsx("span", { style: { fontSize: "11px", color: we <= xe ? m.primaryColor : "#9ca3af" }, children: B })
  ] }, B)) }), ce = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "32px", marginBottom: "12px", color: m.primaryColor }, children: "ID" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Identity Verification" }),
    /* @__PURE__ */ a.jsxs("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "8px" }, children: [
      "Level: ",
      /* @__PURE__ */ a.jsx("strong", { children: n.charAt(0).toUpperCase() + n.slice(1) })
    ] }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "13px", marginBottom: "24px", lineHeight: "1.5" }, children: "We need to verify your identity to comply with regulations. This usually takes a few minutes." }),
    /* @__PURE__ */ a.jsx(V, { onClick: () => g("personal-info"), primaryColor: m.primaryColor, children: "Start Verification" })
  ] }), Je = () => /* @__PURE__ */ a.jsxs("div", { children: [
    Le(),
    /* @__PURE__ */ a.jsx(St, { label: "First Name *", value: x, onChange: (B) => S(B.target.value), placeholder: "John" }),
    /* @__PURE__ */ a.jsx(St, { label: "Last Name *", value: O, onChange: (B) => d(B.target.value), placeholder: "Doe" }),
    /* @__PURE__ */ a.jsx(St, { label: "Date of Birth *", type: "date", value: c, onChange: (B) => p(B.target.value) }),
    /* @__PURE__ */ a.jsx(St, { label: "Nationality", value: v, onChange: (B) => C(B.target.value), placeholder: "Vietnamese" }),
    ue && /* @__PURE__ */ a.jsx("div", { style: $, children: ue }),
    /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", gap: "8px", marginTop: "8px" }, children: [
      /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("intro"), primaryColor: m.primaryColor, children: "Back" }),
      /* @__PURE__ */ a.jsx(V, { onClick: It, primaryColor: m.primaryColor, children: "Next" })
    ] })
  ] }), Tn = () => /* @__PURE__ */ a.jsxs("div", { children: [
    Le(),
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Upload Identity Document" }),
    /* @__PURE__ */ a.jsxs("div", { style: { marginBottom: "16px" }, children: [
      /* @__PURE__ */ a.jsx("label", { style: { fontSize: "13px", fontWeight: 500, color: "#374151", display: "block", marginBottom: "8px" }, children: "Document Type" }),
      /* @__PURE__ */ a.jsxs(
        "select",
        {
          value: j,
          onChange: (B) => T(B.target.value),
          style: {
            width: "100%",
            padding: "8px 12px",
            border: "1px solid #d1d5db",
            borderRadius: "6px",
            fontSize: "14px",
            backgroundColor: "#fff"
          },
          children: [
            /* @__PURE__ */ a.jsx("option", { value: "national_id", children: "National ID Card" }),
            /* @__PURE__ */ a.jsx("option", { value: "passport", children: "Passport" }),
            /* @__PURE__ */ a.jsx("option", { value: "drivers_license", children: "Driver's License" })
          ]
        }
      )
    ] }),
    /* @__PURE__ */ a.jsxs(
      "div",
      {
        onClick: We,
        style: {
          border: `2px dashed ${_ ? m.successColor : "#d1d5db"}`,
          borderRadius: "8px",
          padding: "32px",
          textAlign: "center",
          cursor: "pointer",
          backgroundColor: _ ? "#f0fdf4" : "#fafafa",
          transition: "all 0.2s",
          marginBottom: "16px"
        },
        role: "button",
        tabIndex: 0,
        onKeyDown: (B) => {
          B.key === "Enter" && We();
        },
        children: [
          /* @__PURE__ */ a.jsx("div", { style: { fontSize: "24px", marginBottom: "8px" }, children: _ ? "&#10003;" : "+" }),
          /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, color: _ ? m.successColor : "#6b7280" }, children: _ ? "Document uploaded" : "Click to upload front of document" }),
          /* @__PURE__ */ a.jsx("div", { style: { fontSize: "12px", color: "#9ca3af", marginTop: "4px" }, children: "PNG, JPG up to 10MB" })
        ]
      }
    ),
    /* @__PURE__ */ a.jsx("div", { style: { display: "flex", gap: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("personal-info"), primaryColor: m.primaryColor, children: "Back" }) })
  ] }), Pn = () => /* @__PURE__ */ a.jsxs("div", { children: [
    Le(),
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Take a Selfie" }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "13px", marginBottom: "16px" }, children: "Please take a clear photo of your face. Make sure your face is well-lit and fully visible." }),
    /* @__PURE__ */ a.jsxs(
      "div",
      {
        onClick: $e,
        style: {
          border: `2px dashed ${N ? m.successColor : "#d1d5db"}`,
          borderRadius: "8px",
          padding: "32px",
          textAlign: "center",
          cursor: "pointer",
          backgroundColor: N ? "#f0fdf4" : "#fafafa",
          marginBottom: "16px"
        },
        role: "button",
        tabIndex: 0,
        onKeyDown: (B) => {
          B.key === "Enter" && $e();
        },
        children: [
          /* @__PURE__ */ a.jsx("div", { style: { fontSize: "24px", marginBottom: "8px" }, children: N ? "&#10003;" : "+" }),
          /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, color: N ? m.successColor : "#6b7280" }, children: N ? "Selfie captured" : "Click to take selfie" })
        ]
      }
    ),
    /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("document-upload"), primaryColor: m.primaryColor, children: "Back" })
  ] }), Nn = () => /* @__PURE__ */ a.jsxs("div", { children: [
    Le(),
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "16px", color: "#374151" }, children: "Review Your Information" }),
    /* @__PURE__ */ a.jsxs("div", { style: { backgroundColor: "#f9fafb", borderRadius: "8px", padding: "16px", marginBottom: "16px", fontSize: "14px" }, children: [
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "8px" }, children: [
        /* @__PURE__ */ a.jsx("span", { style: { color: "#6b7280" }, children: "Name" }),
        /* @__PURE__ */ a.jsxs("span", { style: { fontWeight: 500 }, children: [
          x,
          " ",
          O
        ] })
      ] }),
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "8px" }, children: [
        /* @__PURE__ */ a.jsx("span", { style: { color: "#6b7280" }, children: "Date of Birth" }),
        /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 500 }, children: c })
      ] }),
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "8px" }, children: [
        /* @__PURE__ */ a.jsx("span", { style: { color: "#6b7280" }, children: "Document" }),
        /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 500 }, children: j.replace("_", " ") })
      ] }),
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", justifyContent: "space-between" }, children: [
        /* @__PURE__ */ a.jsx("span", { style: { color: "#6b7280" }, children: "Level" }),
        /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 500 }, children: n })
      ] })
    ] }),
    /* @__PURE__ */ a.jsx(V, { onClick: k, primaryColor: m.primaryColor, children: "Submit for Verification" }),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => g("document-upload"), primaryColor: m.primaryColor, children: "Back" }) })
  ] }), Rn = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "24px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: {
      width: "44px",
      height: "44px",
      border: `3px solid ${m.primaryColor}`,
      borderTopColor: "transparent",
      borderRadius: "50%",
      margin: "0 auto 16px",
      animation: "rampos-spin 0.8s linear infinite"
    } }),
    /* @__PURE__ */ a.jsx("div", { style: { fontWeight: 500, color: "#374151" }, children: "Submitting your documents..." }),
    /* @__PURE__ */ a.jsx("style", { children: "@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }" })
  ] }), L = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: {
      width: "44px",
      height: "44px",
      border: `3px solid ${m.primaryColor}`,
      borderTopColor: "transparent",
      borderRadius: "50%",
      margin: "0 auto 16px",
      animation: "rampos-spin 0.8s linear infinite"
    } }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Verification In Progress" }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "14px" }, children: "Your documents are being reviewed. This usually takes a few minutes." }),
    /* @__PURE__ */ a.jsx("style", { children: "@keyframes rampos-spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }" })
  ] }), E = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { color: m.successColor, fontSize: "48px", marginBottom: "8px" }, children: "✓" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Verification Complete" }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "Your identity has been verified successfully." }),
    /* @__PURE__ */ a.jsx(V, { onClick: Ue, primaryColor: m.primaryColor, children: "Done" })
  ] }), F = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "16px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { color: m.errorColor, fontSize: "48px", marginBottom: "8px" }, children: "✗" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Verification Failed" }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "20px" }, children: "We were unable to verify your identity. Please try again with clearer documents." }),
    /* @__PURE__ */ a.jsx(V, { onClick: () => g("intro"), primaryColor: m.primaryColor, children: "Try Again" }),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "ghost", onClick: Ue, primaryColor: m.primaryColor, children: "Close" }) })
  ] });
  return /* @__PURE__ */ a.jsxs("div", { style: P, "data-testid": "rampos-kyc", children: [
    /* @__PURE__ */ a.jsxs("div", { style: R, children: [
      /* @__PURE__ */ a.jsx("span", { children: "RampOS KYC" }),
      /* @__PURE__ */ a.jsx(
        "button",
        {
          onClick: Ue,
          style: { background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#9ca3af" },
          "aria-label": "Close",
          children: "x"
        }
      )
    ] }),
    h === "intro" && ce(),
    h === "personal-info" && Je(),
    h === "document-upload" && Tn(),
    h === "selfie" && Pn(),
    h === "review" && Nn(),
    h === "submitting" && Rn(),
    h === "submitted" && L(),
    h === "approved" && E(),
    h === "rejected" && F(),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "20px", textAlign: "center", fontSize: "11px", color: "#9ca3af" }, children: "Powered by RampOS" })
  ] });
};
class rp extends HTMLElement {
  constructor() {
    super();
    Oe(this, "root", null);
    Oe(this, "mountPoint");
    this.attachShadow({ mode: "open" }), this.mountPoint = document.createElement("div"), this.shadowRoot.appendChild(this.mountPoint);
  }
  static get observedAttributes() {
    return [
      "api-key",
      "user-id",
      "level",
      "environment",
      "theme-primary",
      "theme-bg",
      "theme-text",
      "theme-radius",
      "theme-font"
    ];
  }
  connectedCallback() {
    this.renderComponent();
  }
  attributeChangedCallback() {
    this.renderComponent();
  }
  disconnectedCallback() {
    this.root && (this.root.unmount(), this.root = null);
  }
  getTheme() {
    return {
      primaryColor: this.getAttribute("theme-primary") || void 0,
      backgroundColor: this.getAttribute("theme-bg") || void 0,
      textColor: this.getAttribute("theme-text") || void 0,
      borderRadius: this.getAttribute("theme-radius") || void 0,
      fontFamily: this.getAttribute("theme-font") || void 0
    };
  }
  renderComponent() {
    const n = this.getAttribute("api-key");
    if (!n) {
      console.error("[RampOS] api-key attribute is required for <rampos-kyc>");
      return;
    }
    const r = {
      apiKey: n,
      userId: this.getAttribute("user-id") || void 0,
      level: this.getAttribute("level") || "basic",
      environment: this.getAttribute("environment") || "sandbox",
      theme: this.getTheme(),
      onSubmitted: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-kyc-submitted", { detail: l, bubbles: !0, composed: !0 }));
      },
      onApproved: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-kyc-approved", { detail: l, bubbles: !0, composed: !0 }));
      },
      onRejected: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-kyc-rejected", { detail: l, bubbles: !0, composed: !0 }));
      },
      onError: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-kyc-error", { detail: l, bubbles: !0, composed: !0 }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent("rampos-kyc-close", { bubbles: !0, composed: !0 }));
      },
      onReady: () => {
        this.dispatchEvent(new CustomEvent("rampos-kyc-ready", { bubbles: !0, composed: !0 }));
      }
    };
    this.root || (this.root = bn.createRoot(this.mountPoint)), this.root.render(ii.createElement(np, r));
  }
}
typeof customElements < "u" && !customElements.get("rampos-kyc") && customElements.define("rampos-kyc", rp);
const au = [
  { asset: "USDC", balance: "1,250.00", decimals: 6, usdValue: 1250 },
  { asset: "ETH", balance: "0.5432", decimals: 18, usdValue: 1358 },
  { asset: "MATIC", balance: "500.00", decimals: 18, usdValue: 450 },
  { asset: "VND_TOKEN", balance: "25,000,000", decimals: 18, usdValue: 1e3 }
], lp = [
  { id: "tx1", type: "receive", asset: "USDC", amount: "500", from: "0xabc...def", to: "0x123...456", status: "confirmed", timestamp: Date.now() - 864e5, txHash: "0xfeed..." },
  { id: "tx2", type: "send", asset: "ETH", amount: "0.1", from: "0x123...456", to: "0xdef...abc", status: "confirmed", timestamp: Date.now() - 1728e5, txHash: "0xbeef..." },
  { id: "tx3", type: "receive", asset: "MATIC", amount: "200", from: "0xabc...789", to: "0x123...456", status: "pending", timestamp: Date.now() - 36e5 }
], op = [
  { value: "polygon", label: "Polygon" },
  { value: "arbitrum", label: "Arbitrum" },
  { value: "optimism", label: "Optimism" },
  { value: "ethereum", label: "Ethereum" },
  { value: "base", label: "Base" }
], ip = ({
  apiKey: e,
  userId: t,
  defaultNetwork: n = "polygon",
  theme: r,
  showBalance: l = !0,
  allowSend: o = !0,
  allowReceive: i = !0,
  onConnected: s,
  onDisconnected: u,
  onTransactionSent: f,
  onTransactionConfirmed: m,
  onError: y,
  onClose: h,
  onReady: g
}) => {
  const x = Ji(r), S = En.getInstance(), [O, d] = I.useState("connect"), [c, p] = I.useState(n), [v, C] = I.useState(""), [j, T] = I.useState([]), [_, A] = I.useState([]), [N, ne] = I.useState(""), [ue, ae] = I.useState(""), [Ue, It] = I.useState("USDC"), [We, $e] = I.useState(!1), [k, P] = I.useState(null);
  I.useEffect(() => {
    S.emit("WALLET_READY"), g == null || g();
  }, []);
  const R = I.useCallback(() => {
    S.emit("WALLET_CLOSE"), h == null || h();
  }, [S, h]), $ = async () => {
    try {
      await new Promise((B) => setTimeout(B, 1e3));
      const E = "0x" + Array.from({ length: 40 }, () => Math.floor(Math.random() * 16).toString(16)).join("");
      C(E), T(au), A(lp), d("dashboard");
      const F = {
        address: E,
        network: c,
        balances: au
      };
      S.emit("WALLET_CONNECTED", F), s == null || s(F);
    } catch (E) {
      const F = E instanceof Error ? E.message : "Connection failed";
      P(F), S.emit("WALLET_ERROR", { message: F }), y == null || y(E instanceof Error ? E : new Error(F));
    }
  }, U = () => {
    C(""), T([]), A([]), d("connect"), S.emit("WALLET_DISCONNECTED"), u == null || u();
  }, Ze = async () => {
    if (!N || !ue) {
      P("Please fill in all fields");
      return;
    }
    $e(!0), P(null);
    try {
      await new Promise((F) => setTimeout(F, 2e3));
      const E = {
        id: `tx_${Date.now().toString(36)}`,
        type: "send",
        asset: Ue,
        amount: ue,
        from: v,
        to: N,
        status: "pending",
        timestamp: Date.now(),
        txHash: "0x" + Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join("")
      };
      A((F) => [E, ...F]), S.emit("WALLET_TX_SENT", E), f == null || f(E), setTimeout(() => {
        const F = { ...E, status: "confirmed" };
        A((B) => B.map((we) => we.id === E.id ? F : we)), S.emit("WALLET_TX_CONFIRMED", F), m == null || m(F);
      }, 3e3), ne(""), ae(""), d("dashboard");
    } catch (E) {
      const F = E instanceof Error ? E.message : "Transaction failed";
      P(F), S.emit("WALLET_ERROR", { message: F }), y == null || y(E instanceof Error ? E : new Error(F));
    } finally {
      $e(!1);
    }
  }, xe = {
    fontFamily: x.fontFamily,
    padding: "24px",
    borderRadius: x.borderRadius,
    backgroundColor: x.backgroundColor,
    color: x.textColor,
    boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1)",
    maxWidth: "420px",
    width: "100%"
  }, Le = {
    fontSize: "18px",
    fontWeight: 600,
    marginBottom: "20px",
    borderBottom: "1px solid #e5e7eb",
    paddingBottom: "12px",
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center"
  }, ce = (E) => ({
    padding: "8px 16px",
    fontSize: "13px",
    fontWeight: E ? 600 : 400,
    color: E ? x.primaryColor : "#6b7280",
    borderBottom: E ? `2px solid ${x.primaryColor}` : "2px solid transparent",
    cursor: "pointer",
    background: "none",
    border: "none",
    transition: "all 0.15s"
  }), Je = {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "12px 0",
    borderBottom: "1px solid #f3f4f6"
  }, Tn = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", padding: "24px 0" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "32px", marginBottom: "12px", color: x.primaryColor }, children: "W" }),
    /* @__PURE__ */ a.jsx("h3", { style: { margin: "0 0 8px", color: "#111827" }, children: "Connect Wallet" }),
    /* @__PURE__ */ a.jsx("p", { style: { color: "#6b7280", fontSize: "14px", marginBottom: "16px" }, children: "Connect your RampOS wallet to view balances and send transactions." }),
    /* @__PURE__ */ a.jsxs("div", { style: { marginBottom: "16px" }, children: [
      /* @__PURE__ */ a.jsx("label", { style: { fontSize: "13px", fontWeight: 500, color: "#374151", display: "block", marginBottom: "6px" }, children: "Network" }),
      /* @__PURE__ */ a.jsx(
        "select",
        {
          value: c,
          onChange: (E) => p(E.target.value),
          style: {
            width: "100%",
            padding: "8px 12px",
            border: "1px solid #d1d5db",
            borderRadius: "6px",
            fontSize: "14px",
            backgroundColor: "#fff"
          },
          children: op.map((E) => /* @__PURE__ */ a.jsx("option", { value: E.value, children: E.label }, E.value))
        }
      )
    ] }),
    k && /* @__PURE__ */ a.jsx("div", { style: { color: x.errorColor, fontSize: "13px", padding: "8px 12px", backgroundColor: "#fee2e2", borderRadius: "6px", marginBottom: "12px" }, children: k }),
    /* @__PURE__ */ a.jsx(V, { onClick: $, primaryColor: x.primaryColor, children: "Connect Wallet" })
  ] }), Pn = () => {
    const E = j.reduce((F, B) => F + (B.usdValue || 0), 0);
    return /* @__PURE__ */ a.jsxs("div", { children: [
      /* @__PURE__ */ a.jsxs("div", { style: { backgroundColor: "#f9fafb", borderRadius: "8px", padding: "12px", marginBottom: "16px", fontSize: "13px" }, children: [
        /* @__PURE__ */ a.jsx("div", { style: { color: "#6b7280", marginBottom: "4px" }, children: "Wallet Address" }),
        /* @__PURE__ */ a.jsxs("div", { style: { fontWeight: 500, wordBreak: "break-all" }, children: [
          v.substring(0, 10),
          "...",
          v.substring(v.length - 8)
        ] }),
        /* @__PURE__ */ a.jsxs("div", { style: { color: "#9ca3af", fontSize: "12px", marginTop: "4px" }, children: [
          "Network: ",
          c
        ] })
      ] }),
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", borderBottom: "1px solid #e5e7eb", marginBottom: "16px" }, children: [
        /* @__PURE__ */ a.jsx("button", { style: ce(O === "dashboard"), onClick: () => d("dashboard"), children: "Balances" }),
        /* @__PURE__ */ a.jsx("button", { style: ce(O === "history"), onClick: () => d("history"), children: "History" })
      ] }),
      l && /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center", marginBottom: "16px" }, children: [
        /* @__PURE__ */ a.jsx("div", { style: { color: "#6b7280", fontSize: "13px" }, children: "Total Balance" }),
        /* @__PURE__ */ a.jsxs("div", { style: { fontSize: "28px", fontWeight: 700, color: "#111827" }, children: [
          "$",
          E.toLocaleString("en-US", { minimumFractionDigits: 2 })
        ] })
      ] }),
      /* @__PURE__ */ a.jsx("div", { style: { marginBottom: "16px" }, children: j.map((F) => /* @__PURE__ */ a.jsxs("div", { style: Je, children: [
        /* @__PURE__ */ a.jsxs("div", { children: [
          /* @__PURE__ */ a.jsx("div", { style: { fontWeight: 600 }, children: F.asset }),
          /* @__PURE__ */ a.jsx("div", { style: { fontSize: "12px", color: "#9ca3af" }, children: F.balance })
        ] }),
        F.usdValue !== void 0 && /* @__PURE__ */ a.jsxs("div", { style: { fontWeight: 500, color: "#374151" }, children: [
          "$",
          F.usdValue.toLocaleString("en-US", { minimumFractionDigits: 2 })
        ] })
      ] }, F.asset)) }),
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", gap: "8px" }, children: [
        o && /* @__PURE__ */ a.jsx(V, { onClick: () => d("send"), primaryColor: x.primaryColor, children: "Send" }),
        i && /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => d("receive"), primaryColor: x.primaryColor, children: "Receive" })
      ] }),
      /* @__PURE__ */ a.jsx("div", { style: { marginTop: "12px", textAlign: "center" }, children: /* @__PURE__ */ a.jsx(
        "button",
        {
          onClick: U,
          style: { background: "none", border: "none", fontSize: "13px", color: "#ef4444", cursor: "pointer" },
          children: "Disconnect"
        }
      ) })
    ] });
  }, Nn = () => /* @__PURE__ */ a.jsxs("div", { children: [
    /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", borderBottom: "1px solid #e5e7eb", marginBottom: "16px" }, children: [
      /* @__PURE__ */ a.jsx("button", { style: ce(O === "dashboard"), onClick: () => d("dashboard"), children: "Balances" }),
      /* @__PURE__ */ a.jsx("button", { style: ce(O === "history"), onClick: () => d("history"), children: "History" })
    ] }),
    _.length === 0 ? /* @__PURE__ */ a.jsx("div", { style: { textAlign: "center", padding: "24px 0", color: "#9ca3af", fontSize: "14px" }, children: "No transactions yet" }) : _.map((E) => /* @__PURE__ */ a.jsxs("div", { style: { padding: "12px 0", borderBottom: "1px solid #f3f4f6", fontSize: "14px" }, children: [
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", justifyContent: "space-between", marginBottom: "4px" }, children: [
        /* @__PURE__ */ a.jsx("span", { style: { fontWeight: 500, textTransform: "capitalize" }, children: E.type }),
        /* @__PURE__ */ a.jsxs("span", { style: { fontWeight: 600, color: E.type === "receive" ? x.successColor : "#374151" }, children: [
          E.type === "receive" ? "+" : "-",
          E.amount,
          " ",
          E.asset
        ] })
      ] }),
      /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", justifyContent: "space-between", fontSize: "12px", color: "#9ca3af" }, children: [
        /* @__PURE__ */ a.jsx("span", { children: new Date(E.timestamp).toLocaleDateString() }),
        /* @__PURE__ */ a.jsx("span", { style: {
          padding: "2px 6px",
          borderRadius: "4px",
          fontSize: "11px",
          backgroundColor: E.status === "confirmed" ? "#f0fdf4" : E.status === "pending" ? "#fefce8" : "#fee2e2",
          color: E.status === "confirmed" ? "#16a34a" : E.status === "pending" ? "#ca8a04" : "#dc2626"
        }, children: E.status })
      ] })
    ] }, E.id))
  ] }), Rn = () => /* @__PURE__ */ a.jsxs("div", { children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "12px", color: "#374151" }, children: "Send Tokens" }),
    /* @__PURE__ */ a.jsxs("div", { style: { marginBottom: "12px" }, children: [
      /* @__PURE__ */ a.jsx("label", { style: { fontSize: "13px", fontWeight: 500, color: "#374151", display: "block", marginBottom: "6px" }, children: "Asset" }),
      /* @__PURE__ */ a.jsx(
        "select",
        {
          value: Ue,
          onChange: (E) => It(E.target.value),
          style: {
            width: "100%",
            padding: "8px 12px",
            border: "1px solid #d1d5db",
            borderRadius: "6px",
            fontSize: "14px",
            backgroundColor: "#fff"
          },
          children: j.map((E) => /* @__PURE__ */ a.jsxs("option", { value: E.asset, children: [
            E.asset,
            " (",
            E.balance,
            ")"
          ] }, E.asset))
        }
      )
    ] }),
    /* @__PURE__ */ a.jsx(St, { label: "Recipient Address", value: N, onChange: (E) => ne(E.target.value), placeholder: "0x..." }),
    /* @__PURE__ */ a.jsx(St, { label: "Amount", type: "number", value: ue, onChange: (E) => ae(E.target.value), placeholder: "0.00", min: "0" }),
    k && /* @__PURE__ */ a.jsx("div", { style: { color: x.errorColor, fontSize: "13px", padding: "8px 12px", backgroundColor: "#fee2e2", borderRadius: "6px", marginBottom: "12px" }, children: k }),
    /* @__PURE__ */ a.jsxs("div", { style: { display: "flex", gap: "8px" }, children: [
      /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => d("dashboard"), primaryColor: x.primaryColor, children: "Cancel" }),
      /* @__PURE__ */ a.jsx(V, { onClick: Ze, loading: We, primaryColor: x.primaryColor, children: "Send" })
    ] })
  ] }), L = () => /* @__PURE__ */ a.jsxs("div", { style: { textAlign: "center" }, children: [
    /* @__PURE__ */ a.jsx("div", { style: { fontSize: "14px", fontWeight: 500, marginBottom: "16px", color: "#374151" }, children: "Receive Tokens" }),
    /* @__PURE__ */ a.jsxs("div", { style: {
      backgroundColor: "#f9fafb",
      borderRadius: "8px",
      padding: "20px",
      marginBottom: "16px",
      wordBreak: "break-all"
    }, children: [
      /* @__PURE__ */ a.jsxs("div", { style: { fontSize: "12px", color: "#6b7280", marginBottom: "8px" }, children: [
        "Your Address (",
        c,
        ")"
      ] }),
      /* @__PURE__ */ a.jsx("div", { style: { fontWeight: 500, fontSize: "14px", color: "#111827", fontFamily: "monospace" }, children: v })
    ] }),
    /* @__PURE__ */ a.jsxs("p", { style: { color: "#6b7280", fontSize: "13px", marginBottom: "16px" }, children: [
      "Send tokens to the address above on the ",
      c,
      " network."
    ] }),
    /* @__PURE__ */ a.jsx(
      V,
      {
        onClick: () => {
          typeof navigator < "u" && navigator.clipboard && navigator.clipboard.writeText(v);
        },
        primaryColor: x.primaryColor,
        children: "Copy Address"
      }
    ),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "8px" }, children: /* @__PURE__ */ a.jsx(V, { variant: "secondary", onClick: () => d("dashboard"), primaryColor: x.primaryColor, children: "Back" }) })
  ] });
  return /* @__PURE__ */ a.jsxs("div", { style: xe, "data-testid": "rampos-wallet", children: [
    /* @__PURE__ */ a.jsxs("div", { style: Le, children: [
      /* @__PURE__ */ a.jsx("span", { children: "RampOS Wallet" }),
      /* @__PURE__ */ a.jsx(
        "button",
        {
          onClick: R,
          style: { background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#9ca3af" },
          "aria-label": "Close",
          children: "x"
        }
      )
    ] }),
    O === "connect" && Tn(),
    O === "dashboard" && Pn(),
    O === "history" && Nn(),
    O === "send" && Rn(),
    O === "receive" && L(),
    /* @__PURE__ */ a.jsx("div", { style: { marginTop: "20px", textAlign: "center", fontSize: "11px", color: "#9ca3af" }, children: "Powered by RampOS" })
  ] });
};
class sp extends HTMLElement {
  constructor() {
    super();
    Oe(this, "root", null);
    Oe(this, "mountPoint");
    this.attachShadow({ mode: "open" }), this.mountPoint = document.createElement("div"), this.shadowRoot.appendChild(this.mountPoint);
  }
  static get observedAttributes() {
    return [
      "api-key",
      "user-id",
      "default-network",
      "environment",
      "show-balance",
      "allow-send",
      "allow-receive",
      "theme-primary",
      "theme-bg",
      "theme-text",
      "theme-radius",
      "theme-font"
    ];
  }
  connectedCallback() {
    this.renderComponent();
  }
  attributeChangedCallback() {
    this.renderComponent();
  }
  disconnectedCallback() {
    this.root && (this.root.unmount(), this.root = null);
  }
  getTheme() {
    return {
      primaryColor: this.getAttribute("theme-primary") || void 0,
      backgroundColor: this.getAttribute("theme-bg") || void 0,
      textColor: this.getAttribute("theme-text") || void 0,
      borderRadius: this.getAttribute("theme-radius") || void 0,
      fontFamily: this.getAttribute("theme-font") || void 0
    };
  }
  renderComponent() {
    const n = this.getAttribute("api-key");
    if (!n) {
      console.error("[RampOS] api-key attribute is required for <rampos-wallet>");
      return;
    }
    const r = {
      apiKey: n,
      userId: this.getAttribute("user-id") || void 0,
      defaultNetwork: this.getAttribute("default-network") || "polygon",
      environment: this.getAttribute("environment") || "sandbox",
      showBalance: this.getAttribute("show-balance") !== "false",
      allowSend: this.getAttribute("allow-send") !== "false",
      allowReceive: this.getAttribute("allow-receive") !== "false",
      theme: this.getTheme(),
      onConnected: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-connected", { detail: l, bubbles: !0, composed: !0 }));
      },
      onDisconnected: () => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-disconnected", { bubbles: !0, composed: !0 }));
      },
      onTransactionSent: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-tx-sent", { detail: l, bubbles: !0, composed: !0 }));
      },
      onTransactionConfirmed: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-tx-confirmed", { detail: l, bubbles: !0, composed: !0 }));
      },
      onError: (l) => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-error", { detail: l, bubbles: !0, composed: !0 }));
      },
      onClose: () => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-close", { bubbles: !0, composed: !0 }));
      },
      onReady: () => {
        this.dispatchEvent(new CustomEvent("rampos-wallet-ready", { bubbles: !0, composed: !0 }));
      }
    };
    this.root || (this.root = bn.createRoot(this.mountPoint)), this.root.render(ii.createElement(ip, r));
  }
}
typeof customElements < "u" && !customElements.get("rampos-wallet") && customElements.define("rampos-wallet", sp);
const up = {
  sandbox: "https://sandbox-api.rampos.io/v1",
  production: "https://api.rampos.io/v1"
};
class cu {
  constructor(t) {
    Oe(this, "apiKey");
    Oe(this, "baseUrl");
    this.apiKey = t.apiKey, this.baseUrl = t.baseUrl ?? up[t.environment ?? "sandbox"];
  }
  async request(t, n = {}) {
    const r = `${this.baseUrl}${t}`, l = {
      "Content-Type": "application/json",
      "X-API-Key": this.apiKey,
      ...n.headers || {}
    }, o = await fetch(r, {
      ...n,
      headers: l
    });
    if (!o.ok) {
      const i = await o.text();
      throw new Error(`RampOS API error (${o.status}): ${i}`);
    }
    return o.json();
  }
  // ----- Checkout -----
  async createCheckout(t) {
    return this.request("/checkout", {
      method: "POST",
      body: JSON.stringify(t)
    });
  }
  async confirmCheckout(t) {
    return this.request(`/checkout/${encodeURIComponent(t)}/confirm`, {
      method: "POST"
    });
  }
  async getCheckoutStatus(t) {
    return this.request(`/checkout/${encodeURIComponent(t)}`);
  }
  // ----- KYC -----
  async submitKYC(t) {
    return this.request("/kyc/submit", {
      method: "POST",
      body: JSON.stringify(t)
    });
  }
  async getKYCStatus(t) {
    return this.request(`/kyc/status/${encodeURIComponent(t)}`);
  }
  // ----- Wallet -----
  async getWallet(t, n) {
    const r = n ? `?network=${encodeURIComponent(n)}` : "";
    return this.request(`/wallet/${encodeURIComponent(t)}${r}`);
  }
  async getBalances(t, n) {
    return this.request(`/wallet/${encodeURIComponent(t)}/balances?network=${encodeURIComponent(n)}`);
  }
  async sendTransaction(t) {
    return this.request("/wallet/send", {
      method: "POST",
      body: JSON.stringify(t)
    });
  }
  async getTransactionHistory(t, n) {
    return this.request(
      `/wallet/${encodeURIComponent(t)}/transactions?network=${encodeURIComponent(n)}`
    );
  }
}
const ap = {
  version: "1.0.0",
  EventEmitter: En,
  ApiClient: cu,
  onMessage: bf,
  init(e) {
    return console.log("[RampOS Widget] Initialized", { version: "1.0.0", environment: e.environment || "sandbox" }), new cu({
      apiKey: e.apiKey,
      environment: e.environment
    });
  }
};
window.RampOSWidget = ap;
export {
  ap as default
};
