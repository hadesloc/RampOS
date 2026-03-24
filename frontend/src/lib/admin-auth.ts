import "server-only";
import { createHmac, randomUUID, timingSafeEqual } from "crypto";

export const ADMIN_SESSION_COOKIE = "rampos_admin_session";
export const ADMIN_AUTHORIZATION_HEADER = "X-Admin-Authorization";

export type AdminSession = {
  accessToken: string;
  refreshToken: string;
  accessTokenExpiresAt: number;
  refreshTokenExpiresAt?: number | null;
  admin?: {
    id?: string;
    email?: string;
    role?: string;
  };
};

export function constantTimeEqual(a: string, b: string): boolean {
  const aBuf = Buffer.from(a);
  const bBuf = Buffer.from(b);
  const maxLen = Math.max(aBuf.length, bBuf.length);
  const paddedA = Buffer.concat([aBuf, Buffer.alloc(maxLen - aBuf.length)]);
  const paddedB = Buffer.concat([bBuf, Buffer.alloc(maxLen - bBuf.length)]);
  const matches = timingSafeEqual(paddedA, paddedB);
  return matches && aBuf.length === bBuf.length;
}

// Backward-compatible helper for existing tests:
// - `number` input keeps the legacy nonce.expiry.signature shape
// - `AdminSession` input stores a signed admin session payload
export function createAdminSessionToken(
  secret: string,
  value: number | AdminSession = 60 * 60 * 8
): string {
  if (typeof value === "number") {
    const nonce = randomUUID();
    const expiresAt = Math.floor(Date.now() / 1000) + value;
    const payload = `${nonce}.${expiresAt}`;
    const sig = createHmac("sha256", secret).update(payload).digest("hex");
    return `${payload}.${sig}`;
  }

  const expiresAt = value.accessTokenExpiresAt;
  const payload = Buffer.from(JSON.stringify(value), "utf8").toString("base64url");
  const sig = createHmac("sha256", secret)
    .update(`${payload}.${expiresAt}`)
    .digest("hex");
  return `${payload}.${expiresAt}.${sig}`;
}

export function readAdminSessionToken(
  token: string | undefined,
  secret: string
): AdminSession | null {
  if (!token) return null;
  const parts = token.split(".");
  if (parts.length !== 3) return null;

  const [payloadPart, expiresAtStr, sig] = parts;
  const payload = `${payloadPart}.${expiresAtStr}`;
  const expected = createHmac("sha256", secret).update(payload).digest("hex");
  if (!constantTimeEqual(sig, expected)) return null;

  const expiresAt = Number(expiresAtStr);
  if (!Number.isFinite(expiresAt)) return null;

  try {
    const decoded = JSON.parse(
      Buffer.from(payloadPart, "base64url").toString("utf8")
    ) as AdminSession;
    if (
      typeof decoded?.accessToken !== "string" ||
      typeof decoded?.refreshToken !== "string" ||
      typeof decoded?.accessTokenExpiresAt !== "number"
    ) {
      return null;
    }
    return decoded;
  } catch {
    // Legacy token format intentionally falls through.
    return {
      accessToken: "",
      refreshToken: "",
      accessTokenExpiresAt: expiresAt,
      refreshTokenExpiresAt: null,
    };
  }
}

export function isAdminSessionTokenValid(
  token: string | undefined,
  secret: string
): boolean {
  const session = readAdminSessionToken(token, secret);
  if (!session) return false;

  const now = Math.floor(Date.now() / 1000);
  if (session.accessTokenExpiresAt > now) {
    return true;
  }

  return Boolean(session.refreshToken);
}
