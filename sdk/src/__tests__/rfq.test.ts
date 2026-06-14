import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { RfqService } from '../services/rfq.service';
import { CreateRfqRequest } from '../types/rfq';

describe('RfqService', () => {
  let mock: MockAdapter;
  let service: RfqService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new RfqService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  it('creates a linked off-ramp RFQ with offrampId', async () => {
    const request: CreateRfqRequest = {
      direction: 'OFFRAMP',
      cryptoAsset: 'USDT',
      cryptoAmount: '100',
      offrampId: 'ofr_123',
      ttlMinutes: 5,
    };
    const responseData = {
      id: 'rfq_123',
      direction: 'OFFRAMP',
      cryptoAsset: 'USDT',
      cryptoAmount: '100',
      state: 'OPEN',
      expiresAt: '2026-06-13T00:05:00Z',
      winningLpId: null,
      finalRate: null,
      createdAt: '2026-06-13T00:00:00Z',
    };

    mock.onPost('/portal/rfq').reply(200, responseData);

    const result = await service.create(request);

    expect(result).toEqual(responseData);
    expect(mock.history.post[0].url).toBe('/portal/rfq');
    expect(mock.history.post[0].data).toBe(JSON.stringify(request));
  });

  it('finalizes an admin RFQ', async () => {
    const responseData = {
      rfqId: 'rfq_123',
      state: 'MATCHED',
      winningLpId: 'lp_456',
      finalRate: '25000',
    };

    mock.onPost('/admin/rfq/rfq_123/finalize').reply(200, responseData);

    const result = await service.admin.finalize('rfq_123');

    expect(result).toEqual(responseData);
    expect(mock.history.post[0].url).toBe('/admin/rfq/rfq_123/finalize');
  });

  it('submits an LP bid to the exposed LP RFQ route', async () => {
    const request = {
      exchangeRate: '25100',
      vndAmount: '2510000',
      lpName: 'LP One',
      validMinutes: 5,
    };
    const responseData = {
      id: 'bid_123',
      rfqId: 'rfq_123',
      lpId: 'lp_456',
      exchangeRate: '25100',
      vndAmount: '2510000',
      validUntil: '2026-06-13T00:05:00Z',
      state: 'ACTIVE',
    };

    mock.onPost('/lp/rfq/rfq_123/bid').reply(200, responseData);

    const result = await service.lp.submitBid('rfq_123', request);

    expect(result).toEqual(responseData);
    expect(mock.history.post[0].url).toBe('/lp/rfq/rfq_123/bid');
    expect(mock.history.post[0].data).toBe(JSON.stringify(request));
  });
});
