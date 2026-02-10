import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { IntentService } from '../services/intent.service';
import { IntentType, CreatePayinRequest, ConfirmPayinRequest, CreatePayoutRequest } from '../types/intent';

describe('IntentService', () => {
  let mock: MockAdapter;
  let service: IntentService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new IntentService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  describe('createPayIn', () => {
    it('should create a payin intent', async () => {
      const request: CreatePayinRequest = {
        tenantId: 'test-tenant',
        userId: 'user-1',
        amountVnd: 500000,
        railsProvider: 'VIETQR'
      };

      const responseData = {
        intentId: 'intent-1',
        referenceCode: 'REF123',
        expiresAt: '2024-01-01T00:00:00Z',
        status: 'PENDING'
      };

      mock.onPost('/intents/payin').reply(200, responseData);

      const result = await service.createPayIn(request);
      expect(result).toEqual(responseData);
      expect(mock.history.post[0].data).toBe(JSON.stringify(request));
    });
  });

  describe('confirmPayIn', () => {
    it('should confirm a payin intent', async () => {
      const request: ConfirmPayinRequest = {
        tenantId: 'test-tenant',
        referenceCode: 'REF123',
        status: 'SUCCESS',
        bankTxId: 'BANK123',
        amountVnd: 500000,
        settledAt: '2024-01-01T00:00:00Z',
        rawPayloadHash: 'hash'
      };

      const responseData = {
        intentId: 'intent-1',
        status: 'COMPLETED'
      };

      mock.onPost('/intents/payin/confirm').reply(200, responseData);

      const result = await service.confirmPayIn(request);
      expect(result).toEqual(responseData);
    });
  });

  describe('createPayOut', () => {
    it('should create a payout intent', async () => {
      const request: CreatePayoutRequest = {
        tenantId: 'test-tenant',
        userId: 'user-1',
        amountVnd: 1000000,
        railsProvider: 'NAPAS247',
        bankAccount: {
          bankCode: 'VCB',
          accountNumber: '123456',
          accountName: 'Test User'
        }
      };

      const responseData = {
        intentId: 'intent-2',
        status: 'PENDING'
      };

      mock.onPost('/intents/payout').reply(200, responseData);

      const result = await service.createPayOut(request);
      expect(result).toEqual(responseData);
    });
  });

  describe('get', () => {
    it('should get an intent by id', async () => {
      const intentId = 'intent-1';
      const responseData = {
        id: intentId,
        userId: 'user-1',
        intentType: IntentType.PAYIN,
        state: 'COMPLETED',
        amount: '500000',
        currency: 'VND',
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-01T01:00:00Z'
      };

      mock.onGet(`/intents/${intentId}`).reply(200, responseData);

      const result = await service.get(intentId);
      expect(result).toEqual(responseData);
    });
  });

  describe('list', () => {
    it('should list intents', async () => {
        const intents = [
            {
                id: 'intent-1',
                userId: 'user-1',
                intentType: IntentType.PAYIN,
                state: 'COMPLETED',
                amount: '500000',
                currency: 'VND',
                createdAt: '2024-01-01T00:00:00Z',
                updatedAt: '2024-01-01T01:00:00Z'
            }
        ];

        mock.onGet('/intents').reply(200, intents);

        const result = await service.list();
        expect(result).toHaveLength(1);
        expect(result[0].id).toBe('intent-1');
    });

    it('should list intents wrapped in data', async () => {
        const intents = [
            {
                id: 'intent-1',
                userId: 'user-1',
                intentType: IntentType.PAYIN,
                state: 'COMPLETED',
                amount: '500000',
                currency: 'VND',
                createdAt: '2024-01-01T00:00:00Z',
                updatedAt: '2024-01-01T01:00:00Z'
            }
        ];

        mock.onGet('/intents').reply(200, { data: intents });

        const result = await service.list();
        expect(result).toHaveLength(1);
        expect(result[0].id).toBe('intent-1');
    });
  });
});
