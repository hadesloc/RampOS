import { describe, it, expect } from "vitest";
import enMessages from "../../../messages/en.json";
import viMessages from "../../../messages/vi.json";

/**
 * Recursively extract all keys from a nested JSON object.
 * Returns flat paths like "Dashboard.title", "Portal.auth.login.title"
 */
function extractKeys(obj: Record<string, any>, prefix = ""): string[] {
  const keys: string[] = [];
  for (const key of Object.keys(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof obj[key] === "object" && obj[key] !== null && !Array.isArray(obj[key])) {
      keys.push(...extractKeys(obj[key], fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

/**
 * Extract top-level section names from messages
 */
function extractSections(obj: Record<string, any>): string[] {
  return Object.keys(obj);
}

describe("i18n Completeness", () => {
  const enKeys = extractKeys(enMessages);
  const viKeys = extractKeys(viMessages);

  it("en.json and vi.json should have the same top-level sections", () => {
    const enSections = extractSections(enMessages).sort();
    const viSections = extractSections(viMessages).sort();
    expect(enSections).toEqual(viSections);
  });

  it("en.json and vi.json should have the same keys", () => {
    const enSet = new Set(enKeys);
    const viSet = new Set(viKeys);

    const missingInVi = enKeys.filter((k) => !viSet.has(k));
    const missingInEn = viKeys.filter((k) => !enSet.has(k));

    if (missingInVi.length > 0) {
      console.warn("Keys in en.json missing from vi.json:", missingInVi);
    }
    if (missingInEn.length > 0) {
      console.warn("Keys in vi.json missing from en.json:", missingInEn);
    }

    expect(missingInVi).toEqual([]);
    expect(missingInEn).toEqual([]);
  });

  it("should have all required sections", () => {
    const requiredSections = [
      "Index",
      "Dashboard",
      "Auth",
      "Navigation",
      "Common",
      "Users",
      "Compliance",
      "Intents",
      "Ledger",
      "Swap",
      "Settings",
      "Onboarding",
      "ChainAbstraction",
      "Offramp",
      "AdminOfframp",
      "Portal",
    ];

    const enSections = extractSections(enMessages);
    const viSections = extractSections(viMessages);

    for (const section of requiredSections) {
      expect(enSections).toContain(section);
      expect(viSections).toContain(section);
    }
  });

  it("should have navigation keys for all pages", () => {
    const navKeys = [
      "dashboard",
      "users",
      "kyc",
      "compliance",
      "wallet",
      "transactions",
      "settings",
      "logout",
      "deposit",
      "withdraw",
      "assets",
      "intents",
      "ledger",
      "swap",
      "bridge",
      "yield",
      "offramp",
    ];

    const enNav = enMessages.Navigation as Record<string, string>;
    const viNav = viMessages.Navigation as Record<string, string>;

    for (const key of navKeys) {
      expect(enNav).toHaveProperty(key);
      expect(viNav).toHaveProperty(key);
    }
  });

  it("should have common action keys", () => {
    const commonKeys = [
      "loading",
      "error",
      "success",
      "cancel",
      "save",
      "delete",
      "edit",
      "view",
      "submit",
      "close",
      "confirm",
      "approve",
      "reject",
      "refresh",
      "search",
      "pending",
      "completed",
      "failed",
      "cancelled",
    ];

    const enCommon = enMessages.Common as Record<string, string>;
    const viCommon = viMessages.Common as Record<string, string>;

    for (const key of commonKeys) {
      expect(enCommon).toHaveProperty(key);
      expect(viCommon).toHaveProperty(key);
    }
  });

  it("should have ChainAbstraction keys", () => {
    const chainKeys = [
      "title",
      "source",
      "destination",
      "source_chain",
      "dest_chain",
      "select_chain",
      "select_token",
      "preview_route",
      "execute_intent",
      "route_preview",
      "est_time",
      "total_fees",
    ];

    const enChain = enMessages.ChainAbstraction as Record<string, string>;
    const viChain = viMessages.ChainAbstraction as Record<string, string>;

    for (const key of chainKeys) {
      expect(enChain).toHaveProperty(key);
      expect(viChain).toHaveProperty(key);
    }
  });

  it("should have Offramp keys", () => {
    const offrampKeys = [
      "title",
      "description",
      "crypto_currency",
      "network_fee",
      "service_fee",
      "total_fee",
      "you_receive",
      "bank_account",
      "convert_to_vnd",
      "transaction_status",
      "transaction_history",
      "status_pending",
      "status_processing",
      "status_completed",
      "status_failed",
    ];

    const enOfframp = enMessages.Offramp as Record<string, string>;
    const viOfframp = viMessages.Offramp as Record<string, string>;

    for (const key of offrampKeys) {
      expect(enOfframp).toHaveProperty(key);
      expect(viOfframp).toHaveProperty(key);
    }
  });

  it("should have AdminOfframp keys", () => {
    const adminKeys = [
      "title",
      "description",
      "total_intents",
      "pending_review",
      "success_rate",
      "intent_detail",
      "intent_info",
      "transaction_details",
      "status_timeline",
      "bank_transfer_details",
      "rejection_reason",
      "confirm_reject",
      "intent_approved",
      "intent_rejected",
    ];

    const enAdmin = enMessages.AdminOfframp as Record<string, string>;
    const viAdmin = viMessages.AdminOfframp as Record<string, string>;

    for (const key of adminKeys) {
      expect(enAdmin).toHaveProperty(key);
      expect(viAdmin).toHaveProperty(key);
    }
  });

  it("should have Portal subsections", () => {
    const portalSubsections = [
      "dashboard",
      "wallet",
      "assets",
      "deposit",
      "withdraw",
      "transactions",
      "settings",
      "auth",
    ];

    const enPortal = enMessages.Portal as Record<string, any>;
    const viPortal = viMessages.Portal as Record<string, any>;

    for (const section of portalSubsections) {
      expect(enPortal).toHaveProperty(section);
      expect(viPortal).toHaveProperty(section);
    }
  });

  it("no translation values should be empty strings", () => {
    const emptyEnKeys = enKeys.filter((k) => {
      const parts = k.split(".");
      let val: any = enMessages;
      for (const p of parts) val = val?.[p];
      return val === "";
    });
    const emptyViKeys = viKeys.filter((k) => {
      const parts = k.split(".");
      let val: any = viMessages;
      for (const p of parts) val = val?.[p];
      return val === "";
    });

    expect(emptyEnKeys).toEqual([]);
    expect(emptyViKeys).toEqual([]);
  });

  it("Vietnamese translations should differ from English (not just copied)", () => {
    let sameCount = 0;
    let totalCount = 0;
    const exceptions = new Set([
      "Portal.auth.login.email_placeholder",
      "Portal.auth.register.email_label",
      "Portal.auth.login.email_input_label",
      "Navigation.kyc",
      "Navigation.webhooks",
      "Settings.webhook_url",
      "Portal.settings.passkeys",
      "Navigation.offramp",
      "AdminOfframp.id",
      "AdminOfframp.crypto",
      "AdminOfframp.vnd",
    ]);

    for (const key of enKeys) {
      if (exceptions.has(key)) continue;
      const parts = key.split(".");
      let enVal: any = enMessages;
      let viVal: any = viMessages;
      for (const p of parts) {
        enVal = enVal?.[p];
        viVal = viVal?.[p];
      }
      if (typeof enVal === "string" && typeof viVal === "string") {
        totalCount++;
        if (enVal === viVal) sameCount++;
      }
    }

    // Allow at most 5% identical strings (some technical terms may stay the same)
    const samePercentage = (sameCount / totalCount) * 100;
    expect(samePercentage).toBeLessThan(5);
  });
});
