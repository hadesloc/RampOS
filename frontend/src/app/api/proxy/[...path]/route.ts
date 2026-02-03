import { NextRequest, NextResponse } from 'next/server';
import { cookies } from 'next/headers';
import { ADMIN_SESSION_COOKIE, isAdminSessionTokenValid } from '@/lib/admin-auth';

const API_URL = process.env.API_URL || 'http://localhost:8080';
const API_KEY = process.env.API_KEY || '';
const ADMIN_KEY = process.env.RAMPOS_ADMIN_KEY || '';

async function handleRequest(req: NextRequest, props: { params: Promise<{ path: string[] }> }) {
  const cookieStore = await cookies();
  const token = cookieStore.get(ADMIN_SESSION_COOKIE)?.value;
  if (!isAdminSessionTokenValid(token, ADMIN_KEY)) {
    return NextResponse.json({ message: 'Unauthorized' }, { status: 401 });
  }
  if (!API_KEY || !ADMIN_KEY) {
    return NextResponse.json({ message: 'Server configuration error' }, { status: 500 });
  }

  const params = await props.params;
  const path = params.path.join('/');
  const searchParams = req.nextUrl.searchParams.toString();
  // Ensure we don't double slash if API_URL has trailing slash
  const cleanApiUrl = API_URL.replace(/\/$/, '');
  const url = `${cleanApiUrl}/${path}${searchParams ? `?${searchParams}` : ''}`;

  const headers = new Headers(req.headers);
  headers.set('Authorization', `Bearer ${API_KEY}`);
  headers.set('X-Admin-Key', ADMIN_KEY);

  // Clean up headers that might cause issues
  headers.delete('host');
  headers.delete('content-length');
  headers.delete('connection');

  try {
    const body = req.body;
    const options: RequestInit = {
      method: req.method,
      headers,
      body: (req.method === 'GET' || req.method === 'HEAD') ? undefined : body,
      // @ts-expect-error - duplex is needed for streaming body in fetch
      duplex: 'half'
    };

    const response = await fetch(url, options);

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
