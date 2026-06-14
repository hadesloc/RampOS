import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { OfframpService } from '../services/offramp.service';

describe('OfframpService', () => {
  let mock: MockAdapter;
  let service: OfframpService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new OfframpService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  it('gets off-ramp status with RFQ and settlement linkage fields', async () => {
    const responseData = {
      id: 'ofr_123',
      state: 'VND_TRANSFERRING',
      cryptoAsset: 'USDT',
      cryptoAmount: '100',
      exchangeRate: '25000',
      netVndAmount: '2475000',
      grossVndAmount: '2500000',
      depositAddress: '0x123',
      chainId: 137,
      txHash: '0xtx',
      bankReference: 'RAMP-ABC123',
      linkedRfqId: 'rfq_123',
      winningLpId: 'lp_456',
      matchedRate: '25100',
      settlementId: 'set_789',
      createdAt: '2026-06-13T00:00:00Z',
      updatedAt: '2026-06-13T00:02:00Z',
    };

    mock.onGet('/portal/offramp/ofr_123/status').reply(200, responseData);

    const result = await service.status('ofr_123');

    expect(result.linkedRfqId).toBe('rfq_123');
    expect(result.winningLpId).toBe('lp_456');
    expect(result.matchedRate).toBe('25100');
    expect(result.settlementId).toBe('set_789');
    expect(mock.history.get[0].url).toBe('/portal/offramp/ofr_123/status');
  });

  it('records portal crypto receipt with current camelCase chain fields', async () => {
    const request = {
      txHash: '0xtx',
      chainId: 137,
      fromAddress: '0xfrom',
      toAddress: '0xto',
      blockNumber: 123,
      confirmations: 12,
      rawPayload: { source: 'test' },
    };
    const responseData = {
      id: 'ofr_123',
      state: 'CRYPTO_RECEIVED',
      cryptoAsset: 'USDT',
      cryptoAmount: '100',
      exchangeRate: '25000',
      netVndAmount: '2475000',
      grossVndAmount: '2500000',
      depositAddress: '0xto',
      chainId: 137,
      txHash: '0xtx',
      createdAt: '2026-06-13T00:00:00Z',
      updatedAt: '2026-06-13T00:02:00Z',
    };

    mock.onPost('/portal/offramp/ofr_123/crypto-received').reply(200, responseData);

    const result = await service.cryptoReceived('ofr_123', request);

    expect(result.txHash).toBe('0xtx');
    expect(mock.history.post[0].url).toBe('/portal/offramp/ofr_123/crypto-received');
    expect(mock.history.post[0].data).toBe(JSON.stringify(request));
  });

  it('lists pending admin off-ramps with linked settlement fields', async () => {
    const responseData = {
      data: [
        {
          id: 'ofr_123',
          userId: 'user_123',
          state: 'CRYPTO_RECEIVED',
          cryptoAsset: 'USDT',
          cryptoAmount: '100',
          exchangeRate: '25000',
          netVndAmount: '2475000',
          grossVndAmount: '2500000',
          linkedRfqId: 'rfq_123',
          winningLpId: 'lp_456',
          matchedRate: '25100',
          settlementId: 'set_789',
          createdAt: '2026-06-13T00:00:00Z',
          updatedAt: '2026-06-13T00:02:00Z',
        },
      ],
      total: 1,
      limit: 20,
      offset: 0,
    };

    mock.onGet('/admin/offramp/pending').reply(200, responseData);

    const result = await service.admin.listPending({ limit: 20, offset: 0 });

    expect(result.data[0].linkedRfqId).toBe('rfq_123');
    expect(result.data[0].settlementId).toBe('set_789');
    expect(mock.history.get[0].params).toEqual({ limit: 20, offset: 0 });
  });
});
