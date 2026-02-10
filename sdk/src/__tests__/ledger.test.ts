import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { LedgerService } from '../services/ledger.service';

describe('LedgerService', () => {
  let mock: MockAdapter;
  let service: LedgerService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new LedgerService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  describe('getEntries', () => {
    it('should get ledger entries as array', async () => {
      const entries = [
        {
          id: 'entry-1',
          tenantId: 'tenant-1',
          intentId: 'intent-1',
          transactionId: 'tx-1',
          accountType: 'FIAT',
          direction: 'CREDIT',
          amount: '500000',
          currency: 'VND',
          balanceAfter: '500000',
          sequence: 1,
          createdAt: '2024-01-01T00:00:00Z',
        },
      ];

      mock.onGet('/ledger').reply(200, entries);

      const result = await service.getEntries();
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('entry-1');
      expect(result[0].direction).toBe('CREDIT');
    });

    it('should get ledger entries wrapped in data', async () => {
      const entries = [
        {
          id: 'entry-2',
          tenantId: 'tenant-1',
          intentId: 'intent-2',
          transactionId: 'tx-2',
          accountType: 'CRYPTO',
          direction: 'DEBIT',
          amount: '100',
          currency: 'USDC',
          balanceAfter: '900',
          sequence: 2,
          createdAt: '2024-01-02T00:00:00Z',
        },
      ];

      mock.onGet('/ledger').reply(200, { data: entries });

      const result = await service.getEntries();
      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('entry-2');
    });

    it('should pass filters as query params', async () => {
      mock.onGet('/ledger').reply(200, []);

      await service.getEntries({ intentId: 'intent-1', userId: 'user-1' });

      expect(mock.history.get[0].params).toEqual({
        intentId: 'intent-1',
        userId: 'user-1',
      });
    });

    it('should return empty array for unexpected response', async () => {
      mock.onGet('/ledger').reply(200, { unexpected: 'data' });

      const result = await service.getEntries();
      expect(result).toEqual([]);
    });
  });
});
