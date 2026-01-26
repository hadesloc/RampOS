import { AAService } from '../src/services/aa.service';
import { RampOSClient } from '../src/client';
import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import {
  SmartAccount,
  SessionKey,
  UserOperation,
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
      const params = { owner: 'user-123' };
      const mockResponse: SmartAccount = {
        address: '0x123...',
        owner: 'user-123',
        factoryAddress: '0xabc...',
        salt: '0x1',
        deployed: false,
      };

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
        owner: 'user-123',
        factoryAddress: '0xabc...',
        salt: '0x1',
        deployed: true,
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
        target: '0xTarget...',
        value: '0',
        data: '0x',
        sponsored: true,
      };

      const mockResponse: UserOpReceipt = {
        userOpHash: '0xhash...',
        success: true
      };

      mock.onPost('/aa/bundler/user-op', params).reply(200, mockResponse);

      const result = await aaService.sendUserOperation(params);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('estimateGas', () => {
    it('should estimate gas', async () => {
      const params: UserOperationParams = {
        target: '0xTarget...',
        value: '0',
        data: '0x',
      };
      const mockResponse: GasEstimate = {
        preVerificationGas: '1000',
        verificationGas: '2000',
        callGasLimit: '3000',
      };

      mock.onPost('/aa/bundler/estimate-gas', params).reply(200, mockResponse);

      const result = await aaService.estimateGas(params);
      expect(result).toEqual(mockResponse);
    });
  });
});
