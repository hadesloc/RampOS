import { createHmac } from 'crypto';
import { NextRequest, NextResponse } from 'next/server';
import { cookies } from 'next/headers';
import {
  ADMIN_AUTHORIZATION_HEADER,
  ADMIN_SESSION_COOKIE,
  constantTimeEqual,
  createAdminSessionToken,
  readAdminSessionToken,
} from '@/lib/admin-auth';

export const dynamic = 'force-dynamic';

function isProductionRuntime(): boolean {
  return process.env.NODE_ENV?.trim().toLowerCase() === 'production';
}

function requiredServerEnv(name: string): string {
  const value = process.env[name]?.trim();
  if (value) {
    return value;
  }
  if (isProductionRuntime()) {
    throw new Error(`Missing required production environment variable: ${name}`);
  }
  return '';
}

function getApiUrl(): string {
  return requiredServerEnv('API_URL') || 'http://localhost:8080';
}
function getApiKey(): string {
  return requiredServerEnv('API_KEY');
}
function getApiSecret(): string {
  return requiredServerEnv('API_SECRET');
}
function getAdminSessionSecret(): string {
  return requiredServerEnv('RAMPOS_ADMIN_JWT_SECRET');
}

async function handleRequest(req: NextRequest, props: { params: Promise<{ path: string[] }> }) {
  const cookieStore = await cookies();
  const csrfCookie = cookieStore.get('rampos_csrf')?.value;
  const csrfHeader = req.headers.get('x-csrf-token');
  if (!csrfCookie || !csrfHeader || !constantTimeEqual(csrfCookie, csrfHeader)) {
    return NextResponse.json({ message: 'CSRF check failed' }, { status: 403 });
  }

  const API_URL = getApiUrl();
  const API_KEY = getApiKey();
  const API_SECRET = getApiSecret();
  const ADMIN_SESSION_SECRET = getAdminSessionSecret();

  const token = cookieStore.get(ADMIN_SESSION_COOKIE)?.value;
  const session = readAdminSessionToken(token, ADMIN_SESSION_SECRET);
  if (!session) {
    return NextResponse.json({ message: 'Unauthorized' }, { status: 401 });
  }
  if (!API_KEY || !API_SECRET || !ADMIN_SESSION_SECRET) {
    return NextResponse.json({ message: 'Server configuration error' }, { status: 500 });
  }

  const params = await props.params;
  const path = params.path.join('/');
  const searchParams = req.nextUrl.searchParams.toString();
  // Ensure we don't double slash if API_URL has trailing slash
  const cleanApiUrl = API_URL.replace(/\/$/, '');
  const url = `${cleanApiUrl}/${path}${searchParams ? `?${searchParams}` : ''}`;
  const backendPath = `/${path}`;
  const bodyText =
    req.method === 'GET' || req.method === 'HEAD' ? '' : await req.text();
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const signature = createHmac('sha256', API_SECRET)
    .update(`${req.method}\n${backendPath}\n${timestamp}\n${bodyText}`)
    .digest('hex');

  const headers = new Headers();
  headers.set('Authorization', `Bearer ${API_KEY}`);
  headers.set(ADMIN_AUTHORIZATION_HEADER, `Bearer ${session.accessToken}`);
  headers.set('X-Timestamp', timestamp);
  headers.set('X-Signature', signature);
  // Only forward content-type, not all request headers
  const contentType = req.headers.get('content-type');
  if (contentType) {
    headers.set('Content-Type', contentType);
  }
  const accept = req.headers.get('accept');
  if (accept) {
    headers.set('Accept', accept);
  }

  try {
    const options: RequestInit = {
      method: req.method,
      headers,
      body: req.method === 'GET' || req.method === 'HEAD' ? undefined : bodyText,
      // @ts-expect-error - duplex is needed for streaming body in fetch
      duplex: 'half'
    };

    let response = await fetch(url, options);

    if (response.status === 401 && session.refreshToken) {
      const refreshResponse = await fetch(`${cleanApiUrl}/v1/admin/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify({ refresh_token: session.refreshToken }),
      });

      if (refreshResponse.ok) {
        const refreshPayload = await refreshResponse.json().catch(() => ({}));
        const refreshedSession = {
          ...session,
          accessToken: String(refreshPayload.accessToken || ''),
          accessTokenExpiresAt:
            Math.floor(Date.now() / 1000) + Number(refreshPayload.expiresIn || 0),
        };

        if (refreshedSession.accessToken) {
          cookieStore.set({
            name: ADMIN_SESSION_COOKIE,
            value: createAdminSessionToken(ADMIN_SESSION_SECRET, refreshedSession),
            httpOnly: true,
            sameSite: 'strict',
            secure: process.env.NODE_ENV === 'production',
            path: '/',
            maxAge: 60 * 60 * 24 * 7,
          });

          headers.set(ADMIN_AUTHORIZATION_HEADER, `Bearer ${refreshedSession.accessToken}`);
          response = await fetch(url, options);
        }
      }
    }

    return new NextResponse(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  } catch (error) {
    console.error('Proxy error:', error);
    return NextResponse.json({ message: 'Internal Server Error' }, { status: 500 });
  }
}

export async function GET(req: NextRequest, props: any) {
  return handleRequest(req, props);
}

export async function POST(req: NextRequest, props: any) {
  return handleRequest(req, props);
}

export async function PUT(req: NextRequest, props: any) {
  return handleRequest(req, props);
}

export async function DELETE(req: NextRequest, props: any) {
  return handleRequest(req, props);
}

export async function PATCH(req: NextRequest, props: any) {
  return handleRequest(req, props);
}
