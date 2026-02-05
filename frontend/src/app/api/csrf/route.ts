import { NextResponse } from "next/server";
import { cookies } from "next/headers";
import { randomUUID } from "crypto";

export async function GET() {
  const token = randomUUID();
  (await cookies()).set({
    name: "rampos_csrf",
    value: token,
    httpOnly: false,
    sameSite: "strict",
    secure: process.env.NODE_ENV === "production",
    path: "/",
  });
  return NextResponse.json(
    { token },
    { headers: { "Cache-Control": "no-store" } }
  );
}
