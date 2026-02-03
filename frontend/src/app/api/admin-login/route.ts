import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { ADMIN_SESSION_COOKIE, buildAdminSessionToken } from "@/lib/admin-auth";

export async function POST(req: Request) {
  const adminKey = process.env.RAMPOS_ADMIN_KEY;
  if (!adminKey) {
    return NextResponse.json(
      { message: "Admin key not configured" },
      { status: 500 }
    );
  }

  const body = await req.json().catch(() => ({}));
  const key = typeof body?.key === "string" ? body.key : "";

  if (key !== adminKey) {
    return NextResponse.json({ message: "Invalid admin key" }, { status: 401 });
  }

  const token = buildAdminSessionToken(adminKey);
  (await cookies()).set({
    name: ADMIN_SESSION_COOKIE,
    value: token,
    httpOnly: true,
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: 60 * 60 * 8,
  });

  return NextResponse.json({ ok: true });
}
