import "server-only";
import { createHmac, timingSafeEqual } from "crypto";

export const ADMIN_SESSION_COOKIE = "rampos_admin_session";
const ADMIN_SESSION_SCOPE = "rampos-admin-session";

export function buildAdminSessionToken(secret: string): string {
  return createHmac("sha256", secret).update(ADMIN_SESSION_SCOPE).digest("hex");
}

export function isAdminSessionTokenValid(
  token: string | undefined,
  secret: string
): boolean {
  if (!token) return false;
  const expected = buildAdminSessionToken(secret);
  if (token.length !== expected.length) return false;
  return timingSafeEqual(Buffer.from(token), Buffer.from(expected));
}
