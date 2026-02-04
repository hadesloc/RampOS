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
    path: "/",
  });
  return NextResponse.json({ token });
}
