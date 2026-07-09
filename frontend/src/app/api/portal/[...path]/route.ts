import { NextRequest, NextResponse } from "next/server";

export const dynamic = "force-dynamic";

function apiUrl(): string {
  return (process.env.API_URL || "http://localhost:8080").replace(/\/$/, "");
}

async function handleRequest(
  request: NextRequest,
  props: { params: Promise<{ path: string[] }> },
) {
  const { path } = await props.params;
  const query = request.nextUrl.searchParams.toString();
  const url = `${apiUrl()}/${path.join("/")}${query ? `?${query}` : ""}`;
  const headers = new Headers();
  const contentType = request.headers.get("content-type");
  const accept = request.headers.get("accept");
  const cookie = request.headers.get("cookie");
  if (contentType) headers.set("content-type", contentType);
  if (accept) headers.set("accept", accept);
  if (cookie) headers.set("cookie", cookie);

  try {
    const response = await fetch(url, {
      method: request.method,
      headers,
      body:
        request.method === "GET" || request.method === "HEAD"
          ? undefined
          : await request.text(),
      cache: "no-store",
    });

    return new NextResponse(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  } catch (error) {
    console.error("Portal proxy error:", error);
    return NextResponse.json(
      { error: { code: "UPSTREAM_UNAVAILABLE", message: "Portal API is unavailable" } },
      { status: 502 },
    );
  }
}

export async function GET(request: NextRequest, props: any) {
  return handleRequest(request, props);
}

export async function POST(request: NextRequest, props: any) {
  return handleRequest(request, props);
}

export async function PUT(request: NextRequest, props: any) {
  return handleRequest(request, props);
}

export async function DELETE(request: NextRequest, props: any) {
  return handleRequest(request, props);
}

