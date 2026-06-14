import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { SettlementService } from '../services/settlement.service';

describe('SettlementService', () => {
  let mock: MockAdapter;
  let service: SettlementService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new SettlementService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  it('applies a completed settlement outcome and preserves linkage fields', async () => {
    const request = {
      outcome: 'COMPLETED' as const,
    };
    const responseData = {
      settlementId: 'set_789',
      offrampIntentId: 'ofr_123',
      status: 'COMPLETED',
      rfqId: 'rfq_123',
      lpId: 'lp_456',
      finalRate: '25100',
    };

    mock.onPost('/admin/settlement/set_789/outcome').reply(200, responseData);

    const result = await service.applyOutcome('set_789', request);

    expect(result).toEqual(responseData);
    expect(result.rfqId).toBe('rfq_123');
    expect(result.lpId).toBe('lp_456');
    expect(result.finalRate).toBe('25100');
    expect(mock.history.post[0].url).toBe('/admin/settlement/set_789/outcome');
    expect(mock.history.post[0].data).toBe(JSON.stringify(request));
  });

  it('applies a failed settlement outcome with errorMessage', async () => {
    const request = {
      outcome: 'FAILED' as const,
      errorMessage: 'bank transfer failed',
    };
    const responseData = {
      settlementId: 'set_789',
      offrampIntentId: 'ofr_123',
      status: 'FAILED',
      rfqId: 'rfq_123',
      lpId: 'lp_456',
      finalRate: '25100',
    };

    mock.onPost('/admin/settlement/set_789/outcome').reply(200, responseData);

    const result = await service.applyOutcome('set_789', request);

    expect(result.status).toBe('FAILED');
    expect(mock.history.post[0].data).toBe(JSON.stringify(request));
  });
});
