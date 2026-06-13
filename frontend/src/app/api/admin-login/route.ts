import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import {
  ADMIN_SESSION_COOKIE,
  AdminSession,
  constantTimeEqual,
  createAdminSessionToken,
} from "@/lib/admin-auth";

function isProductionRuntime(): boolean {
  return process.env.NODE_ENV?.trim().toLowerCase() === "production";
}

function requiredServerEnv(name: string): string {
  const value = process.env[name]?.trim();
  if (value) {
    return value;
  }
  if (isProductionRuntime()) {
    throw new Error(`Missing required production environment variable: ${name}`);
  }
  return "";
}

export async function POST(req: Request) {
  const sessionSecret = requiredServerEnv("RAMPOS_ADMIN_JWT_SECRET");
  const apiUrl = (requiredServerEnv("API_URL") || "http://localhost:8080").replace(/\/$/, "");
  if (!sessionSecret) {
    return NextResponse.json(
      { message: "Admin auth not configured" },
      { status: 500 }
    );
  }

  const cookieStore = await cookies();

  const csrfCookie = cookieStore.get("rampos_csrf")?.value;
  const csrfHeader = req.headers.get("x-csrf-token");
  if (!csrfCookie || !csrfHeader || !constantTimeEqual(csrfCookie, csrfHeader)) {
    return NextResponse.json({ message: "CSRF check failed" }, { status: 403 });
  }

  const body = await req.json().catch(() => ({}));
  const email = typeof body?.email === "string" ? body.email : "";
  const password = typeof body?.password === "string" ? body.password : "";
  if (!email || !password) {
    return NextResponse.json({ message: "Missing credentials" }, { status: 400 });
  }

  const response = await fetch(`${apiUrl}/v1/admin/auth/login`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({ email, password }),
  }).catch(() => null);

  if (!response) {
    return NextResponse.json({ message: "Login failed" }, { status: 502 });
  }

  const payload = await response.json().catch(() => ({}));
  if (!response.ok) {
    return NextResponse.json(
      { message: payload?.message || "Invalid admin credentials" },
      { status: response.status }
    );
  }

  const session: AdminSession = {
    accessToken: String(payload.accessToken || ""),
    refreshToken: String(payload.refreshToken || ""),
    accessTokenExpiresAt:
      Math.floor(Date.now() / 1000) + Number(payload.expiresIn || 0),
    refreshTokenExpiresAt: Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 7,
    admin: payload.admin,
  };

  if (!session.accessToken || !session.refreshToken || !session.accessTokenExpiresAt) {
    return NextResponse.json({ message: "Invalid login response" }, { status: 502 });
  }

  const token = createAdminSessionToken(sessionSecret, session);
  cookieStore.set({
    name: ADMIN_SESSION_COOKIE,
    value: token,
    httpOnly: true,
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    maxAge: 60 * 60 * 24 * 7,
  });

  return NextResponse.json({ ok: true });
}
