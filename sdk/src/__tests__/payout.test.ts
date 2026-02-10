import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { IntentService } from '../services/intent.service';
import { CreatePayoutRequest } from '../types/intent';

describe('PayoutService (via IntentService)', () => {
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

    it('should create a payout successfully', async () => {
      const request: CreatePayoutRequest = {
        tenantId: 'test-tenant',
        userId: 'user-123',
        amountVnd: 200000,
        railsProvider: 'NAPAS247',
        bankAccount: {
            bankCode: 'VCB',
            accountNumber: '123456789',
            accountName: 'TEST USER'
        }
      };

      const response = {
        intentId: 'intent-456',
        status: 'PENDING'
      };

      mock.onPost('/intents/payout').reply(200, response);

      const result = await service.createPayOut(request);
      expect(result.intentId).toBe('intent-456');
    });

    it('should fail with invalid bank account', async () => {
        const request: CreatePayoutRequest = {
          tenantId: 'test-tenant',
          userId: 'user-123',
          amountVnd: 200000,
          railsProvider: 'NAPAS247',
          bankAccount: {
              bankCode: 'XYZ', // Invalid bank
              accountNumber: '123456789',
              accountName: 'TEST USER'
          }
        };

        mock.onPost('/intents/payout').reply(400, { code: 'INVALID_BANK' });

        await expect(service.createPayOut(request)).rejects.toThrow();
    });
  });
