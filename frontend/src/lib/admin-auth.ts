import "server-only";
import { createHmac, randomUUID, timingSafeEqual } from "crypto";

export const ADMIN_SESSION_COOKIE = "rampos_admin_session";

export function constantTimeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  return timingSafeEqual(Buffer.from(a), Buffer.from(b));
}

export function createAdminSessionToken(
  secret: string,
  ttlSeconds = 60 * 60 * 8
): string {
  const nonce = randomUUID();
  const expiresAt = Math.floor(Date.now() / 1000) + ttlSeconds;
  const payload = `${nonce}.${expiresAt}`;
  const sig = createHmac("sha256", secret).update(payload).digest("hex");
  return `${payload}.${sig}`;
}

export function isAdminSessionTokenValid(
  token: string | undefined,
  secret: string
): boolean {
  if (!token) return false;
  const parts = token.split(".");
  if (parts.length !== 3) return false;
  const [nonce, expiresAtStr, sig] = parts;
  const expiresAt = Number(expiresAtStr);
  if (!Number.isFinite(expiresAt) || expiresAt <= Math.floor(Date.now() / 1000)) {
    return false;
  }
  const payload = `${nonce}.${expiresAtStr}`;
  const expected = createHmac("sha256", secret).update(payload).digest("hex");
  if (sig.length !== expected.length) return false;
  return timingSafeEqual(Buffer.from(sig), Buffer.from(expected));
}
