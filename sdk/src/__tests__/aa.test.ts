import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { AAService } from '../services/aa.service';

describe('AAService', () => {
  let mock: MockAdapter;
  let service: AAService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new AAService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  describe('createSmartAccount', () => {
    it('should create a smart account', async () => {
      const params = {
        userId: 'user-1',
        chainId: 1,
      };

      const responseData = {
        address: '0x1234567890abcdef',
        userId: 'user-1',
        chainId: 1,
        deployed: true,
      };

      mock.onPost('/aa/accounts').reply(200, responseData);

      const result = await service.createSmartAccount(params);
      expect(result.address).toBe('0x1234567890abcdef');
      expect(result.deployed).toBe(true);
    });
  });

  describe('getSmartAccount', () => {
    it('should get smart account by address', async () => {
      const address = '0x1234567890abcdef';
      const responseData = {
        address,
        userId: 'user-1',
        chainId: 1,
        deployed: true,
        nonce: 0,
      };

      mock.onGet(`/aa/accounts/${address}`).reply(200, responseData);

      const result = await service.getSmartAccount(address);
      expect(result.address).toBe(address);
    });
  });

  describe('sendUserOperation', () => {
    it('should send a user operation', async () => {
      const params = {
        sender: '0x1234',
        callData: '0xabcd',
        chainId: 1,
      };

      const responseData = {
        userOpHash: '0xhash123',
        status: 'PENDING',
      };

      mock.onPost('/aa/user-operations').reply(200, responseData);

      const result = await service.sendUserOperation(params);
      expect(result.userOpHash).toBe('0xhash123');
    });
  });

  describe('estimateGas', () => {
    it('should estimate gas for user operation', async () => {
      const params = {
        sender: '0x1234',
        callData: '0xabcd',
        chainId: 1,
      };

      const responseData = {
        callGasLimit: '100000',
        verificationGasLimit: '50000',
        preVerificationGas: '21000',
        maxFeePerGas: '30000000000',
        maxPriorityFeePerGas: '1000000000',
      };

      mock.onPost('/aa/user-operations/estimate').reply(200, responseData);

      const result = await service.estimateGas(params);
      expect(result.callGasLimit).toBe('100000');
    });
  });

  describe('getUserOperation', () => {
    it('should get user operation by hash', async () => {
      const hash = '0xhash123';
      const responseData = {
        userOpHash: hash,
        sender: '0x1234',
        status: 'COMPLETED',
        transactionHash: '0xtxhash',
      };

      mock.onGet(`/aa/user-operations/${hash}`).reply(200, responseData);

      const result = await service.getUserOperation(hash);
      expect(result.userOpHash).toBe(hash);
    });
  });

  describe('getUserOperationReceipt', () => {
    it('should get user operation receipt', async () => {
      const hash = '0xhash123';
      const responseData = {
        userOpHash: hash,
        success: true,
        actualGasCost: '2100000',
        actualGasUsed: '70000',
      };

      mock.onGet(`/aa/user-operations/${hash}/receipt`).reply(200, responseData);

      const result = await service.getUserOperationReceipt(hash);
      expect(result.success).toBe(true);
    });
  });
});
