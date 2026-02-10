import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { IntentService } from '../services/intent.service';
import { CreatePayinRequest, CreatePayoutRequest } from '../types/intent';

describe('PayinService (via IntentService)', () => {
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

  it('should create a payin successfully', async () => {
    const request: CreatePayinRequest = {
      tenantId: 'test-tenant',
      userId: 'user-123',
      amountVnd: 500000,
      railsProvider: 'VIETQR'
    };

    const response = {
      intentId: 'intent-123',
      referenceCode: 'REF123',
      expiresAt: '2024-01-01T00:00:00Z',
      status: 'PENDING'
    };

    mock.onPost('/intents/payin').reply(200, response);

    const result = await service.createPayIn(request);
    expect(result.intentId).toBe('intent-123');
    expect(result.status).toBe('PENDING');
  });

  it('should handle payin errors gracefully', async () => {
    const request: CreatePayinRequest = {
      tenantId: 'test-tenant',
      userId: 'user-123',
      amountVnd: -100, // Invalid amount
      railsProvider: 'VIETQR'
    };

    mock.onPost('/intents/payin').reply(400, {
      code: 'INVALID_AMOUNT',
      message: 'Amount must be positive'
    });

    await expect(service.createPayIn(request)).rejects.toThrow();
  });
});
