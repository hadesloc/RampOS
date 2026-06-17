import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { ADMIN_SESSION_COOKIE, readAdminSessionToken } from "@/lib/admin-auth";

/**
 * GET /api/admin/session
 *
 * Returns the identity of the currently signed-in admin (email / name / role)
 * for display in the UI. The admin session lives in a `server-only`, httpOnly
 * cookie, so client components cannot read it directly — this route surfaces
 * just the non-sensitive identity fields from the HMAC-verified session.
 */
export async function GET() {
  const secret = process.env.RAMPOS_ADMIN_JWT_SECRET?.trim();
  if (!secret) {
    return NextResponse.json({ authenticated: false }, { status: 200 });
  }

  const cookieStore = await cookies();
  const token = cookieStore.get(ADMIN_SESSION_COOKIE)?.value;
  const session = readAdminSessionToken(token, secret);

  if (!session) {
    return NextResponse.json({ authenticated: false }, { status: 200 });
  }

  return NextResponse.json({
    authenticated: true,
    email: session.admin?.email ?? null,
    displayName: session.admin?.displayName ?? null,
    role: session.admin?.role ?? null,
  });
}
