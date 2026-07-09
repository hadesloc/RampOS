import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { randomUUID } from "crypto";
import { shouldUseSecureCookies } from "@/lib/admin-auth";

export async function GET() {
  const token = randomUUID();
  (await cookies()).set({
    name: "rampos_csrf",
    value: token,
    httpOnly: false,
    sameSite: "strict",
    secure: shouldUseSecureCookies(),
    path: "/",
  });
  return NextResponse.json(
    { token },
    { headers: { "Cache-Control": "no-store" } }
  );
}
