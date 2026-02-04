import "server-only";
import { createHmac, randomUUID, timingSafeEqual } from "crypto";

export const ADMIN_SESSION_COOKIE = "rampos_admin_session";

export function constantTimeEqual(a: string, b: string): boolean {
  const aBuf = Buffer.from(a);
  const bBuf = Buffer.from(b);
  const maxLen = Math.max(aBuf.length, bBuf.length);
  const paddedA = Buffer.concat([aBuf, Buffer.alloc(maxLen - aBuf.length)]);
  const paddedB = Buffer.concat([bBuf, Buffer.alloc(maxLen - bBuf.length)]);
  const matches = timingSafeEqual(paddedA, paddedB);
  return matches && aBuf.length === bBuf.length;
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
