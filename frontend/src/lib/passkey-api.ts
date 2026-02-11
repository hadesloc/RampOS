/**
 * Passkey API Client
 *
 * Communicates with the backend passkey endpoints for WebAuthn
 * registration, authentication, and credential management.
 */

import {
  type SerializedCredential,
  type RegistrationChallenge,
  type AuthenticationChallenge,
} from './webauthn';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

export class PasskeyApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = 'PasskeyApiError';
  }
}

async function passkeyRequest<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  const headers: HeadersInit = {
    'Content-Type': 'application/json',
    ...options.headers,
  };

  const response = await fetch(url, {
    ...options,
    headers,
    credentials: 'include',
  });

  if (!response.ok) {
    let errorData: { code?: string; message?: string } = {};
    try {
      errorData = await response.json();
    } catch {
      errorData = { message: response.statusText };
    }

    throw new PasskeyApiError(
      response.status,
      errorData.code || 'PASSKEY_ERROR',
      errorData.message || 'Passkey operation failed',
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

export interface PasskeyInfo {
  id: string;
  name: string;
  createdAt: string;
  lastUsedAt: string | null;
}

export interface VerifyRegistrationResponse {
  credentialId: string;
  userId: string;
}

export interface VerifyAuthenticationResponse {
  userId: string;
  token: string;
}

export const passkeyApi = {
  /** Get registration challenge from server */
  getRegistrationChallenge: async (
    displayName: string,
  ): Promise<RegistrationChallenge> => {
    return passkeyRequest<RegistrationChallenge>(
      '/v1/auth/webauthn/register/challenge',
      {
        method: 'POST',
        body: JSON.stringify({ displayName }),
      },
    );
  },

  /** Verify registration credential with server */
  verifyRegistration: async (
    credential: SerializedCredential,
  ): Promise<VerifyRegistrationResponse> => {
    return passkeyRequest<VerifyRegistrationResponse>(
      '/v1/auth/webauthn/register/complete',
      {
        method: 'POST',
        body: JSON.stringify({ credential }),
      },
    );
  },

  /** Get authentication challenge from server */
  getAuthenticationChallenge: async (): Promise<AuthenticationChallenge> => {
    return passkeyRequest<AuthenticationChallenge>(
      '/v1/auth/webauthn/login/challenge',
      {
        method: 'POST',
      },
    );
  },

  /** Verify authentication credential with server */
  verifyAuthentication: async (
    credential: SerializedCredential,
  ): Promise<VerifyAuthenticationResponse> => {
    return passkeyRequest<VerifyAuthenticationResponse>(
      '/v1/auth/webauthn/login/complete',
      {
        method: 'POST',
        body: JSON.stringify({ credential }),
      },
    );
  },

  /** List all passkeys for the current user */
  listPasskeys: async (): Promise<PasskeyInfo[]> => {
    return passkeyRequest<PasskeyInfo[]>('/v1/portal/passkeys');
  },

  /** Delete a passkey by credential ID */
  deletePasskey: async (credentialId: string): Promise<void> => {
    return passkeyRequest<void>(
      `/v1/portal/passkeys/${encodeURIComponent(credentialId)}`,
      {
        method: 'DELETE',
      },
    );
  },

  /** Rename a passkey */
  renamePasskey: async (
    credentialId: string,
    name: string,
  ): Promise<PasskeyInfo> => {
    return passkeyRequest<PasskeyInfo>(
      `/v1/portal/passkeys/${encodeURIComponent(credentialId)}`,
      {
        method: 'PATCH',
        body: JSON.stringify({ name }),
      },
    );
  },
};
