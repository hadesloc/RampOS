import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { AAService } from '../src/services/aa.service';
import {
  CreateAccountResponse,
  EstimateGasRequest,
  GasEstimate,
  SendUserOperationRequest,
  SendUserOperationResponse,
  SmartAccount,
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
      const params = {
        tenantId: 'tenant-1',
        userId: 'user-123',
        ownerAddress: '0xowner...',
      };
      const mockResponse: CreateAccountResponse = {
        address: '0x123...',
        owner: '0xowner...',
        accountType: 'simple',
        isDeployed: false,
        chainId: 1,
        entryPoint: '0xentry...',
      };

      mock.onPost('/aa/accounts', params).reply(200, mockResponse);

      const result = await aaService.createSmartAccount(params);
      expect(result).toEqual({
        address: expect.any(String),
        owner: expect.any(String),
        accountType: expect.any(String),
        isDeployed: expect.any(Boolean),
        chainId: expect.any(Number),
        entryPoint: expect.any(String),
      });
    });
  });

  describe('getSmartAccount', () => {
    it('should get smart account info', async () => {
      const address = '0x123...';
      const mockResponse: SmartAccount = {
        address: '0x123...',
        owner: '0xowner...',
        isDeployed: true,
        chainId: 1,
        entryPoint: '0xentry...',
        accountType: 'simple',
      };

      mock.onGet(`/aa/accounts/${address}`).reply(200, mockResponse);

      const result = await aaService.getSmartAccount(address);
      expect(result).toEqual({
        address: expect.any(String),
        owner: expect.any(String),
        accountType: expect.any(String),
        isDeployed: expect.any(Boolean),
        chainId: expect.any(Number),
        entryPoint: expect.any(String),
      });
    });
  });


  describe('sendUserOperation', () => {
    it('should send a user operation', async () => {
      const params: SendUserOperationRequest = {
        tenantId: 'tenant-1',
        userOperation: {
          sender: '0xsender...',
          nonce: '1',
          callData: '0xcall...',
          callGasLimit: '100000',
          verificationGasLimit: '200000',
          preVerificationGas: '30000',
          maxFeePerGas: '100',
          maxPriorityFeePerGas: '2',
        },
      };

      const mockResponse: SendUserOperationResponse = {
        userOpHash: '0xhash...',
        sender: params.userOperation.sender,
        nonce: params.userOperation.nonce,
        status: 'PENDING',
        sponsored: false,
      };

      mock.onPost('/aa/user-operations', params).reply(200, mockResponse);

      const result = await aaService.sendUserOperation(params);
      expect(result).toEqual(mockResponse);
    });
  });

  describe('estimateGas', () => {
    it('should estimate gas', async () => {
      const params: EstimateGasRequest = {
        tenantId: 'tenant-1',
        userOperation: {
          sender: '0xsender...',
          nonce: '1',
          callData: '0xcall...',
          callGasLimit: '100000',
          verificationGasLimit: '200000',
          preVerificationGas: '30000',
          maxFeePerGas: '100',
          maxPriorityFeePerGas: '2',
        },
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
