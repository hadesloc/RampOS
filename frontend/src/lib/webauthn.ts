/**
 * WebAuthn Utilities
 *
 * Helper functions for WebAuthn/Passkey authentication.
 */

// Check if WebAuthn is supported
export function isWebAuthnSupported(): boolean {
  return (
    typeof window !== "undefined" &&
    !!window.PublicKeyCredential &&
    typeof window.PublicKeyCredential === "function"
  );
}

// Check if platform authenticator is available (Touch ID, Face ID, Windows Hello)
export async function isPlatformAuthenticatorAvailable(): Promise<boolean> {
  if (!isWebAuthnSupported()) return false;

  try {
    return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return false;
  }
}

// Check if conditional mediation (autofill) is supported
export async function isConditionalMediationAvailable(): Promise<boolean> {
  if (!isWebAuthnSupported()) return false;

  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const pkc = PublicKeyCredential as any;
    return await pkc.isConditionalMediationAvailable?.() ?? false;
  } catch {
    return false;
  }
}

// Base64URL encoding/decoding utilities
export function base64UrlEncode(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let str = "";
  for (let i = 0; i < bytes.length; i++) {
    str += String.fromCharCode(bytes[i]);
  }
  return btoa(str).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

export function base64UrlDecode(str: string): ArrayBuffer {
  const base64 = str.replace(/-/g, "+").replace(/_/g, "/");
  const padding = "=".repeat((4 - (base64.length % 4)) % 4);
  const raw = atob(base64 + padding);
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) {
    bytes[i] = raw.charCodeAt(i);
  }
  return bytes.buffer;
}

// WebAuthn credential types
export interface WebAuthnCredential {
  id: string;
  rawId: ArrayBuffer;
  response: AuthenticatorAttestationResponse | AuthenticatorAssertionResponse;
  type: "public-key";
}

export interface SerializedCredential {
  id: string;
  rawId: string;
  type: "public-key";
  response: {
    clientDataJSON: string;
    attestationObject?: string;
    authenticatorData?: string;
    signature?: string;
    userHandle?: string;
  };
}

// Serialize credential for API transport
export function serializeCredential(
  credential: PublicKeyCredential
): SerializedCredential {
  const response = credential.response;

  const serialized: SerializedCredential = {
    id: credential.id,
    rawId: base64UrlEncode(credential.rawId),
    type: "public-key",
    response: {
      clientDataJSON: base64UrlEncode(response.clientDataJSON),
    },
  };

  if ("attestationObject" in response) {
    // Registration response
    const attestationResponse = response as AuthenticatorAttestationResponse;
    serialized.response.attestationObject = base64UrlEncode(
      attestationResponse.attestationObject
    );
  } else {
    // Authentication response
    const assertionResponse = response as AuthenticatorAssertionResponse;
    serialized.response.authenticatorData = base64UrlEncode(
      assertionResponse.authenticatorData
    );
    serialized.response.signature = base64UrlEncode(assertionResponse.signature);
    if (assertionResponse.userHandle) {
      serialized.response.userHandle = base64UrlEncode(
        assertionResponse.userHandle
      );
    }
  }

  return serialized;
}

// Challenge types from server
export interface RegistrationChallenge {
  challenge: string;
  rpId: string;
  rpName: string;
  userId: string;
  userName: string;
  userDisplayName: string;
  timeout: number;
  attestation: AttestationConveyancePreference;
  authenticatorSelection?: AuthenticatorSelectionCriteria;
  pubKeyCredParams: PublicKeyCredentialParameters[];
  excludeCredentials?: Array<{
    id: string;
    type: "public-key";
    transports?: AuthenticatorTransport[];
  }>;
}

export interface AuthenticationChallenge {
  challenge: string;
  rpId: string;
  timeout: number;
  userVerification?: UserVerificationRequirement;
  allowCredentials?: Array<{
    id: string;
    type: "public-key";
    transports?: AuthenticatorTransport[];
  }>;
}

// Convert server challenge to browser options
export function toRegistrationOptions(
  challenge: RegistrationChallenge
): PublicKeyCredentialCreationOptions {
  return {
    challenge: base64UrlDecode(challenge.challenge),
    rp: {
      id: challenge.rpId,
      name: challenge.rpName,
    },
    user: {
      id: base64UrlDecode(challenge.userId),
      name: challenge.userName,
      displayName: challenge.userDisplayName,
    },
    pubKeyCredParams: challenge.pubKeyCredParams,
    timeout: challenge.timeout,
    attestation: challenge.attestation,
    authenticatorSelection: challenge.authenticatorSelection,
    excludeCredentials: challenge.excludeCredentials?.map((cred) => ({
      id: base64UrlDecode(cred.id),
      type: cred.type,
      transports: cred.transports,
    })),
  };
}

export function toAuthenticationOptions(
  challenge: AuthenticationChallenge
): PublicKeyCredentialRequestOptions {
  return {
    challenge: base64UrlDecode(challenge.challenge),
    rpId: challenge.rpId,
    timeout: challenge.timeout,
    userVerification: challenge.userVerification || "preferred",
    allowCredentials: challenge.allowCredentials?.map((cred) => ({
      id: base64UrlDecode(cred.id),
      type: cred.type,
      transports: cred.transports,
    })),
  };
}

// High-level registration function
export async function startRegistration(
  options: PublicKeyCredentialCreationOptions
): Promise<SerializedCredential> {
  if (!isWebAuthnSupported()) {
    throw new Error("WebAuthn is not supported in this browser");
  }

  const credential = (await navigator.credentials.create({
    publicKey: options,
  })) as PublicKeyCredential | null;

  if (!credential) {
    throw new Error("Failed to create credential");
  }

  return serializeCredential(credential);
}

// High-level authentication function
export async function startAuthentication(
  options: PublicKeyCredentialRequestOptions
): Promise<SerializedCredential> {
  if (!isWebAuthnSupported()) {
    throw new Error("WebAuthn is not supported in this browser");
  }

  const credential = (await navigator.credentials.get({
    publicKey: options,
  })) as PublicKeyCredential | null;

  if (!credential) {
    throw new Error("Failed to get credential");
  }

  return serializeCredential(credential);
}

// Abort controller for canceling WebAuthn operations
let currentAbortController: AbortController | null = null;

export function abortWebAuthn(): void {
  if (currentAbortController) {
    currentAbortController.abort();
    currentAbortController = null;
  }
}

// Start registration with abort support
export async function startRegistrationWithAbort(
  options: PublicKeyCredentialCreationOptions
): Promise<SerializedCredential> {
  abortWebAuthn();
  currentAbortController = new AbortController();

  try {
    const credential = (await navigator.credentials.create({
      publicKey: options,
      signal: currentAbortController.signal,
    })) as PublicKeyCredential | null;

    if (!credential) {
      throw new Error("Failed to create credential");
    }

    return serializeCredential(credential);
  } finally {
    currentAbortController = null;
  }
}

// Start authentication with abort support
export async function startAuthenticationWithAbort(
  options: PublicKeyCredentialRequestOptions
): Promise<SerializedCredential> {
  abortWebAuthn();
  currentAbortController = new AbortController();

  try {
    const credential = (await navigator.credentials.get({
      publicKey: options,
      signal: currentAbortController.signal,
    })) as PublicKeyCredential | null;

    if (!credential) {
      throw new Error("Failed to get credential");
    }

    return serializeCredential(credential);
  } finally {
    currentAbortController = null;
  }
}
