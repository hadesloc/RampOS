import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { PasskeyWalletService } from '../services/passkey.service';

describe('PasskeyWalletService', () => {
  let mock: MockAdapter;
  let service: PasskeyWalletService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new PasskeyWalletService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  describe('createWallet', () => {
    it('should create a passkey wallet', async () => {
      const params = {
        userId: 'user-1',
        credentialId: 'cred-1',
        publicKeyX: '0xabc',
        publicKeyY: '0xdef',
        displayName: 'My Passkey',
      };

      const responseData = {
        credentialId: 'cred-1',
        smartAccountAddress: '0xwallet123',
        publicKeyX: '0xabc',
        publicKeyY: '0xdef',
        isDeployed: true,
        createdAt: '2024-01-01T00:00:00Z',
      };

      mock.onPost('/aa/passkey/wallets').reply(200, responseData);

      const result = await service.createWallet(params);
      expect(result.smartAccountAddress).toBe('0xwallet123');
      expect(result.isDeployed).toBe(true);
    });
  });

  describe('getCounterfactualAddress', () => {
    it('should return the deterministic address', async () => {
      const params = {
        publicKeyX: '0xabc',
        publicKeyY: '0xdef',
      };

      const responseData = {
        address: '0xcounterfactual',
        isDeployed: false,
      };

      mock.onPost('/aa/passkey/address').reply(200, responseData);

      const result = await service.getCounterfactualAddress(params);
      expect(result.address).toBe('0xcounterfactual');
      expect(result.isDeployed).toBe(false);
    });
  });

  describe('signTransaction', () => {
    it('should sign and submit a user operation', async () => {
      const params = {
        userId: 'user-1',
        credentialId: 'cred-1',
        userOperation: {
          sender: '0xsender',
          nonce: '1',
          callData: '0xcalldata',
        },
        assertion: {
          authenticatorData: '0xauth',
          clientDataJSON: '0xclient',
          signature: { r: '0xabc', s: '0xdef' },
          credentialId: 'cred-1',
        },
      };

      const responseData = {
        userOpHash: '0xsubmitted',
        sender: '0xsender',
        nonce: '1',
        signature: '0xsig',
        status: 'PENDING',
      };

      mock.onPost('/aa/passkey/sign').reply(200, responseData);

      const result = await service.signTransaction(params);
      expect(result.userOpHash).toBe('0xsubmitted');
    });
  });

  describe('registerCredential', () => {
    it('should register a passkey credential', async () => {
      const params = {
        userId: 'user-1',
        credentialId: 'cred-new',
        publicKeyX: '0xabc',
        publicKeyY: '0xdef',
        displayName: 'New Key',
      };

      const responseData = {
        credentialId: 'cred-new',
        createdAt: '2024-01-01T00:00:00Z',
      };

      mock.onPost('/aa/passkey/credentials').reply(200, responseData);

      const result = await service.registerCredential(params);
      expect(result.credentialId).toBe('cred-new');
    });
  });

  describe('getCredentials', () => {
    it('should get all credentials for a user', async () => {
      const userId = 'user-1';
      const responseData = [
        {
          credentialId: 'cred-1',
          userId,
          publicKeyX: '0xabc',
          publicKeyY: '0xdef',
          displayName: 'Key 1',
          isActive: true,
          createdAt: '2024-01-01T00:00:00Z',
        },
      ];

      mock.onGet(`/aa/passkey/credentials/${userId}`).reply(200, responseData);

      const result = await service.getCredentials(userId);
      expect(result).toHaveLength(1);
      expect(result[0].credentialId).toBe('cred-1');
    });
  });

  describe('linkSmartAccount', () => {
    it('should link a credential to a smart account', async () => {
      const params = {
        userId: 'user-1',
        credentialId: 'cred-1',
        smartAccountAddress: '0xwallet',
      };

      mock.onPost('/aa/passkey/link').reply(200);

      await expect(service.linkSmartAccount(params)).resolves.not.toThrow();
    });
  });

  describe('deactivateCredential', () => {
    it('should deactivate a credential', async () => {
      mock.onDelete('/aa/passkey/credentials/user-1/cred-1').reply(200);

      await expect(service.deactivateCredential('user-1', 'cred-1')).resolves.not.toThrow();
    });
  });
});
