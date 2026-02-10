import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { MultichainService } from '../services/multichain.service';

describe('MultichainService', () => {
  let mock: MockAdapter;
  let service: MultichainService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new MultichainService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  describe('getSupportedChains', () => {
    it('should return all supported chains', () => {
      const chains = service.getSupportedChains();
      expect(chains.length).toBeGreaterThan(0);
    });
  });

  describe('getChain', () => {
    it('should return chain config by ID', () => {
      const chain = service.getChain(1);
      expect(chain).toBeDefined();
      expect(chain?.name).toBeDefined();
    });

    it('should return undefined for unsupported chain', () => {
      const chain = service.getChain(999999);
      expect(chain).toBeUndefined();
    });
  });

  describe('isEvmChain', () => {
    it('should return true for Ethereum', () => {
      expect(service.isEvmChain(1)).toBe(true);
    });
  });

  describe('getPortfolio', () => {
    it('should get multi-chain portfolio', async () => {
      const address = '0x1234';
      const responseData = {
        accounts: [
          {
            address,
            chainId: 1,
            chainName: 'Ethereum',
            accountType: 'EOA',
            isDeployed: true,
            balance: '0.5',
          },
        ],
        totalBalanceUsd: '1000.00',
      };

      mock.onGet(`/multichain/portfolio/${address}`).reply(200, responseData);

      const result = await service.getPortfolio(address);
      expect(result.accounts).toHaveLength(1);
      expect(result.accounts[0].address).toBe(address);
    });
  });

  describe('getTokens', () => {
    it('should get tokens for address on chain', async () => {
      const address = '0x1234';
      const tokens = [
        {
          address: '0xtoken',
          symbol: 'USDC',
          name: 'USD Coin',
          decimals: 6,
          chainId: 1,
        },
      ];

      mock.onGet(`/multichain/tokens/${address}`).reply(200, tokens);

      const result = await service.getTokens(address, 1);
      expect(result).toHaveLength(1);
      expect(result[0].symbol).toBe('USDC');
    });
  });

  describe('createIntent', () => {
    it('should create a cross-chain intent', async () => {
      const intent = {
        sourceChainId: 1,
        targetChainId: 42161,
        type: 'BRIDGE' as const,
        fromAddress: '0xfrom',
        toAddress: '0xto',
        amount: '100',
      };

      const responseData = {
        intentId: 'intent-1',
        status: 'PENDING',
        sourceChainId: 1,
        targetChainId: 42161,
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T00:00:00Z',
      };

      mock.onPost('/multichain/intents').reply(200, responseData);

      const result = await service.createIntent(intent);
      expect(result.intentId).toBe('intent-1');
    });
  });

  describe('getBridgeQuote', () => {
    it('should get bridge quote', async () => {
      const request = {
        sourceChainId: 1,
        targetChainId: 42161,
        tokenAddress: '0xtoken',
        amount: '100',
        fromAddress: '0xfrom',
      };

      const responseData = {
        sourceChainId: 1,
        targetChainId: 42161,
        inputAmount: '100',
        outputAmount: '99.5',
        bridgeFee: '0.3',
        gasFee: '0.2',
        estimatedTimeSeconds: 300,
        bridgeProvider: 'stargate',
        expiresAt: '2024-01-01T01:00:00Z',
      };

      mock.onPost('/multichain/bridge/quote').reply(200, responseData);

      const result = await service.getBridgeQuote(request);
      expect(result.outputAmount).toBe('99.5');
    });
  });

  describe('executeBridge', () => {
    it('should execute a bridge transaction', async () => {
      const responseData = {
        id: 'bridge-tx-1',
        status: 'PENDING',
        sourceChainId: 1,
        targetChainId: 42161,
        amount: '100',
        fee: '0.5',
        createdAt: '2024-01-01T00:00:00Z',
      };

      mock.onPost('/multichain/bridge/execute').reply(200, responseData);

      const result = await service.executeBridge('quote-1');
      expect(result.id).toBe('bridge-tx-1');
    });
  });

  describe('createEip7702Authorization', () => {
    it('should create EIP-7702 authorization', async () => {
      const responseData = {
        delegateAddress: '0xdelegate',
        chainId: 1,
        nonce: 0,
        signature: '0xsig',
      };

      mock.onPost('/multichain/eip7702/authorize').reply(200, responseData);

      const result = await service.createEip7702Authorization('0xdelegate', 1);
      expect(result.delegateAddress).toBe('0xdelegate');
    });
  });

  describe('createSessionDelegation', () => {
    it('should create session delegation', async () => {
      const params = {
        delegate: '0xdelegate',
        chainId: 1,
        validUntil: 1700000000,
        permissions: {
          maxValuePerTx: '1000',
        },
      };

      const responseData = {
        sessionId: 'session-1',
        delegator: '0xdelegator',
        delegate: '0xdelegate',
        chainId: 1,
        validAfter: 1699000000,
        validUntil: 1700000000,
      };

      mock.onPost('/multichain/eip7702/session').reply(200, responseData);

      const result = await service.createSessionDelegation(params);
      expect(result.sessionId).toBe('session-1');
    });
  });

  describe('revokeSession', () => {
    it('should revoke a session', async () => {
      mock.onDelete('/multichain/eip7702/sessions/session-1').reply(200);

      await expect(service.revokeSession('session-1')).resolves.not.toThrow();
    });
  });
});
