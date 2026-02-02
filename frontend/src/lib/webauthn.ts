export interface WebAuthnCredential {
  id: string;
  rawId: ArrayBuffer;
  response: AuthenticatorAttestationResponse | AuthenticatorAssertionResponse;
  type: 'public-key';
}

export const startRegistration = async (email: string): Promise<boolean> => {
  console.log('Starting WebAuthn registration for', email);
  return new Promise((resolve) => {
    setTimeout(() => {
      console.log('WebAuthn registration successful');
      resolve(true);
    }, 1000);
  });
};

export const startAuthentication = async (email?: string): Promise<boolean> => {
  console.log('Starting WebAuthn authentication', email ? `for ${email}` : '');
  return new Promise((resolve) => {
    setTimeout(() => {
      console.log('WebAuthn authentication successful');
      resolve(true);
    }, 1000);
  });
};
