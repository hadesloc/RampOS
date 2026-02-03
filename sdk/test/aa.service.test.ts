import { AAService } from '../src/services/aa.service';
import { RampOSClient } from '../src/client';
import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import {
  SmartAccount,
  SessionKey,
  GasEstimate,
  UserOpReceipt,
  UserOperationParams,
} from '../src/types/aa';

describe('AAService', () => {
  let mock: MockAdapter;
  let aaService: AAService;

  const mockBaseUrl = 'https://api.rampos.io/v1';

  beforeEach(() => {
    mock = new MockAdapter(axios);
    const axiosInstance = axios.create({ baseURL: mockBaseUrl });
    mock = new MockAdapter(axiosInstance);
    aaService = new AAService(axiosInstance);
  });

  afterEach(() => {
    mock.reset();
  });

  describe('createSmartAccount', () => {
    it('should create a smart account', async () => {
      const params = { userId: 'user-123', ownerAddress: '0xowner...', chainId: 1 };
      const mockAccount: SmartAccount = {
        address: '0x123...',
        tenantId: 'tenant-1',
        userId: 'user-123',
        ownerAddress: '0xowner...',
        chainId: 1,
        factoryAddress: '0xabc...',
        entryPoint: '0xentry...',
        accountType: 'simple',
        isDeployed: false,
        createdAt: '2026-01-01T00:00:00Z',
        updatedAt: '2026-01-01T00:00:00Z',
      };
      const mockResponse = { account: mockAccount };

      mock.onPost('/aa/accounts', params).reply(200, mockResponse);

      const result = await aaService.createSmartAccount(params);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('getSmartAccount', () => {
    it('should get smart account info', async () => {
      const address = '0x123...';
      const mockResponse: SmartAccount = {
        address: '0x123...',
        tenantId: 'tenant-1',
        userId: 'user-123',
        ownerAddress: '0xowner...',
        chainId: 1,
        factoryAddress: '0xabc...',
        entryPoint: '0xentry...',
        accountType: 'simple',
        isDeployed: true,
        createdAt: '2026-01-01T00:00:00Z',
        updatedAt: '2026-01-01T00:00:00Z',
      };

      mock.onGet(`/aa/accounts/${address}`).reply(200, mockResponse);

      const result = await aaService.getSmartAccount(address);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('addSessionKey', () => {
    it('should add a session key', async () => {
      const accountAddress = '0x123...';
      const sessionKey: SessionKey = {
        publicKey: '0xpub...',
        permissions: ['mock-permission'],
        validUntil: 1234567890,
      };

      mock.onPost(`/aa/accounts/${accountAddress}/sessions`, sessionKey).reply(200);

      await aaService.addSessionKey({ accountAddress, sessionKey });
      // No assertion needed, just ensure no error thrown
    });
  });

  describe('removeSessionKey', () => {
    it('should remove a session key', async () => {
      const accountAddress = '0x123...';
      const keyId = 'session-1';

      mock.onDelete(`/aa/accounts/${accountAddress}/sessions/${keyId}`).reply(200);

      await aaService.removeSessionKey({ accountAddress, keyId });
      // No assertion needed, just ensure no error thrown
    });
  });

  describe('sendUserOperation', () => {
    it('should send a user operation', async () => {
      const params: UserOperationParams = {
        sender: '0xsender...',
        chainId: 1,
        callData: '0xcall...',
      };

      const mockResponse: UserOpReceipt = {
        userOperation: {
          id: 'op_123',
          sender: params.sender,
          nonce: '0x1',
          initCode: '0x',
          callData: params.callData,
          callGasLimit: '100000',
          verificationGasLimit: '200000',
          preVerificationGas: '30000',
          maxFeePerGas: '100',
          maxPriorityFeePerGas: '2',
          paymasterAndData: '0x',
          signature: '0xsig',
          status: 'PENDING',
          chainId: params.chainId,
          createdAt: '2026-01-01T00:00:00Z',
          updatedAt: '2026-01-01T00:00:00Z',
        },
        userOpHash: '0xhash...',
      };

      mock.onPost('/aa/user-operations', params).reply(200, mockResponse);

      const result = await aaService.sendUserOperation(params);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('estimateGas', () => {
    it('should estimate gas', async () => {
      const params: UserOperationParams = {
        sender: '0xsender...',
        chainId: 1,
        callData: '0xcall...',
      };
      const mockResponse: GasEstimate = {
        callGasLimit: '3000',
        verificationGasLimit: '2000',
        preVerificationGas: '1000',
        maxFeePerGas: '100',
        maxPriorityFeePerGas: '2',
      };

      mock.onPost('/aa/user-operations/estimate', params).reply(200, mockResponse);

      const result = await aaService.estimateGas(params);
      expect(result).toEqual(mockResponse);
    });
  });
});
