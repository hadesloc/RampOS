import { AxiosInstance } from 'axios';
import {
  PasskeyCredential,
  PasskeyCredentialSchema,
  RegisterPasskeyParams,
  RegisterPasskeyResponse,
  RegisterPasskeyResponseSchema,
  CreatePasskeyWalletParams,
  CreatePasskeyWalletResponse,
  CreatePasskeyWalletResponseSchema,
  LinkSmartAccountParams,
  SignTransactionParams,
  SignTransactionResponse,
  SignTransactionResponseSchema,
  GetCounterfactualAddressParams,
  GetCounterfactualAddressResponse,
  GetCounterfactualAddressResponseSchema,
} from '../types/passkey';

/**
 * PasskeyWalletService - Manages passkey-native smart accounts.
 *
 * Provides methods for:
 * - Creating passkey wallets (registers credential + deploys smart account)
 * - Signing ERC-4337 UserOperations with WebAuthn P256 signatures
 * - Managing passkey credentials (register, list, deactivate)
 * - Linking passkeys to smart accounts
 * - Computing counterfactual (CREATE2) addresses
 */
export class PasskeyWalletService {
  constructor(private readonly httpClient: AxiosInstance) {}

  // ==========================================================================
  // Wallet Lifecycle
  // ==========================================================================

  /**
   * Create a passkey wallet: registers the credential and deploys a smart
   * account with the passkey set as a signer.
   *
   * @param params - Passkey public key coordinates + user info
   * @returns Deployed smart account address and credential info
   */
  async createWallet(params: CreatePasskeyWalletParams): Promise<CreatePasskeyWalletResponse> {
    const response = await this.httpClient.post('/aa/passkey/wallets', params);
    return CreatePasskeyWalletResponseSchema.parse(response.data);
  }

  /**
   * Get the counterfactual (CREATE2) address for a passkey wallet before
   * deployment. Useful for pre-funding the account.
   *
   * @param params - Passkey public key coordinates + optional salt
   * @returns The deterministic address and deployment status
   */
  async getCounterfactualAddress(
    params: GetCounterfactualAddressParams
  ): Promise<GetCounterfactualAddressResponse> {
    const response = await this.httpClient.post('/aa/passkey/address', params);
    return GetCounterfactualAddressResponseSchema.parse(response.data);
  }

  // ==========================================================================
  // Transaction Signing
  // ==========================================================================

  /**
   * Sign and submit an ERC-4337 UserOperation using a passkey (WebAuthn P256).
   *
   * The client is responsible for:
   * 1. Calling `navigator.credentials.get()` to obtain the WebAuthn assertion
   * 2. Extracting the P256 signature (r, s) from the assertion
   * 3. Passing the assertion data to this method
   *
   * The backend will:
   * 1. Encode the signature with the passkey type byte (0x01)
   * 2. Submit the UserOperation to the bundler
   *
   * @param params - UserOperation + WebAuthn assertion with P256 signature
   * @returns Submitted UserOperation hash and status
   */
  async signTransaction(params: SignTransactionParams): Promise<SignTransactionResponse> {
    const response = await this.httpClient.post('/aa/passkey/sign', params);
    return SignTransactionResponseSchema.parse(response.data);
  }

  // ==========================================================================
  // Credential Management
  // ==========================================================================

  /**
   * Register a new passkey credential for a user.
   *
   * This stores the P256 public key coordinates from a WebAuthn registration
   * ceremony. The credential can later be linked to a smart account.
   *
   * @param params - Credential ID + P256 public key (x, y) + display name
   * @returns Registered credential info
   */
  async registerCredential(params: RegisterPasskeyParams): Promise<RegisterPasskeyResponse> {
    const response = await this.httpClient.post('/aa/passkey/credentials', params);
    return RegisterPasskeyResponseSchema.parse(response.data);
  }

  /**
   * Get all passkey credentials for a user.
   *
   * @param userId - The user ID to fetch credentials for
   * @returns Array of passkey credentials (only active ones)
   */
  async getCredentials(userId: string): Promise<PasskeyCredential[]> {
    const response = await this.httpClient.get(`/aa/passkey/credentials/${userId}`);
    return PasskeyCredentialSchema.array().parse(response.data);
  }

  /**
   * Get a specific passkey credential by credential ID.
   *
   * @param userId - The user ID
   * @param credentialId - The credential ID from WebAuthn registration
   * @returns The passkey credential
   */
  async getCredential(userId: string, credentialId: string): Promise<PasskeyCredential> {
    const response = await this.httpClient.get(
      `/aa/passkey/credentials/${userId}/${credentialId}`
    );
    return PasskeyCredentialSchema.parse(response.data);
  }

  /**
   * Link a passkey credential to an existing smart account address.
   *
   * @param params - User ID + credential ID + smart account address
   */
  async linkSmartAccount(params: LinkSmartAccountParams): Promise<void> {
    await this.httpClient.post('/aa/passkey/link', params);
  }

  /**
   * Deactivate a passkey credential.
   *
   * The credential will no longer be usable for signing but is kept
   * for audit purposes.
   *
   * @param userId - The user ID
   * @param credentialId - The credential ID to deactivate
   */
  async deactivateCredential(userId: string, credentialId: string): Promise<void> {
    await this.httpClient.delete(`/aa/passkey/credentials/${userId}/${credentialId}`);
  }
}
