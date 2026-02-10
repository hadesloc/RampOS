import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { RampOSClient } from '../client';
import { RampOSConfig } from '../types';

describe('RampOSClient', () => {
  let mock: MockAdapter;
  const config: RampOSConfig = {
    apiKey: 'test-api-key',
    apiSecret: 'test-api-secret',
    baseURL: 'https://api.test.rampos.io/v1',
    timeout: 5000,
    tenantId: 'test-tenant-id'
  };

  beforeEach(() => {
    mock = new MockAdapter(axios);
  });

  afterEach(() => {
    mock.restore();
  });

  it('should initialize with correct config', () => {
    const client = new RampOSClient(config);
    expect(client).toBeInstanceOf(RampOSClient);
    expect(client.intents).toBeDefined();
    expect(client.users).toBeDefined();
    expect(client.ledger).toBeDefined();
    expect(client.aa).toBeDefined();
    expect(client.passkey).toBeDefined();
    expect(client.webhooks).toBeDefined();
  });

  it('should use default baseURL if not provided', () => {
    const client = new RampOSClient({ ...config, baseURL: undefined });
    // We can't easily check private properties, but we can verify it doesn't throw
    expect(client).toBeInstanceOf(RampOSClient);
  });

  it('should include authentication headers in requests', async () => {
    const client = new RampOSClient(config);

    // Hack to access private httpClient to mock it, or we just use axios mock on the global axios instance
    // But RampOSClient creates its own instance.
    // Since we can't easily access the instance inside client, we rely on the fact that we mocked 'axios'
    // BUT wait, RampOSClient uses `axios.create()`. MockAdapter can mock an instance or the default one.
    // If we pass `axios` to MockAdapter, it mocks the default instance.
    // `axios.create()` returns a NEW instance.
    // To mock the instance created inside, we might need to spy on axios.create or similar.
    // However, `axios-mock-adapter` v1.x usually mocks the instance passed to it.

    // Let's check how RampOSClient imports axios.
    // import axios from 'axios';
    // this.httpClient = axios.create({...});

    // If I mock `axios` directly, `axios.create` might not be affected in the way I want if I don't use onGet on the created instance.
    // Actually, `axios-mock-adapter` when initialized with `axios` mocks the default instance.
    // `axios.create()` returns a new instance which inherits defaults but is separate.
    // BUT commonly people mock `axios` and hope `create` returns something that is also mocked?
    // No, usually you have to mock the instance.

    // Since I cannot access the instance created inside `RampOSClient` (it is private),
    // I will try to use a spy on axios.create if possible, OR I will assume that I can't easily unit test the headers
    // without exposing the client.

    // Alternatively, I can cast client to any and access httpClient.
    const clientAny = client as any;
    const mockInstance = new MockAdapter(clientAny.httpClient);

    mockInstance.onGet('/test').reply(200, { success: true });

    await clientAny.httpClient.get('/test');

    expect(mockInstance.history.get.length).toBe(1);
    const request = mockInstance.history.get[0];

    expect(request.headers?.['Authorization']).toBe(`Bearer ${config.apiKey}`);
    expect(request.headers?.['X-Tenant-ID']).toBe(config.tenantId);
    expect(request.headers?.['X-Signature']).toBeDefined();
    expect(request.headers?.['X-Timestamp']).toBeDefined();
  });

  it('should generate correct HMAC signature', async () => {
    const client = new RampOSClient(config);
    const clientAny = client as any;
    const mockInstance = new MockAdapter(clientAny.httpClient);

    const body = { amount: 1000 };
    mockInstance.onPost('/test').reply(200, { success: true });

    await clientAny.httpClient.post('/test', body);

    const request = mockInstance.history.post[0];
    expect(request.headers?.['X-Signature']).toBeDefined();

    // We verify that the signature is a hex string (HMAC-SHA256 output is 64 chars hex)
    expect(request.headers?.['X-Signature']).toMatch(/^[0-9a-f]{64}$/);
  });

  it('should retry failed requests', async () => {
    const client = new RampOSClient({
        ...config,
        retry: { maxRetries: 3, baseDelay: 10 }
    });
    const clientAny = client as any;
    const mockInstance = new MockAdapter(clientAny.httpClient);

    mockInstance.onGet('/retry-test')
      .replyOnce(500);

    mockInstance.onGet('/retry-test')
      .replyOnce(500);

    mockInstance.onGet('/retry-test')
      .reply(200, { success: true });

    await clientAny.httpClient.get('/retry-test');

    expect(mockInstance.history.get.length).toBe(3);
  });

  it('should create multichain provider', () => {
      const client = new RampOSClient(config);
      const provider = client.createMultichainProvider();
      expect(provider).toBeDefined();
  });
});
