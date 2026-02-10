import axios from 'axios';
import MockAdapter from 'axios-mock-adapter';
import { UserService } from '../services/user.service';
import { KycStatus } from '../types/user';

describe('UserService', () => {
  let mock: MockAdapter;
  let service: UserService;
  const httpClient = axios.create();

  beforeEach(() => {
    mock = new MockAdapter(httpClient);
    service = new UserService(httpClient);
  });

  afterEach(() => {
    mock.restore();
  });

  describe('getBalances', () => {
    it('should get user balances', async () => {
      const userId = 'user-1';
      const responseData = {
        balances: [
          {
            accountType: 'FIAT',
            currency: 'VND',
            balance: '1000000'
          },
          {
            accountType: 'CRYPTO',
            currency: 'USDC',
            balance: '100.5'
          }
        ]
      };

      mock.onGet(`/balance/${userId}`).reply(200, responseData);

      const result = await service.getBalances(userId);
      expect(result).toHaveLength(2);
      expect(result[0].currency).toBe('VND');
      expect(result[1].currency).toBe('USDC');
    });
  });

  describe('getKycStatus', () => {
    it('should get user KYC status', async () => {
      const tenantId = 'tenant-1';
      const userId = 'user-1';
      const responseData = {
        userId: userId,
        status: KycStatus.VERIFIED,
        updatedAt: '2024-01-01T00:00:00Z'
      };

      mock.onGet(`/tenants/${tenantId}/users/${userId}/kyc`).reply(200, responseData);

      const result = await service.getKycStatus(tenantId, userId);
      expect(result.status).toBe(KycStatus.VERIFIED);
      expect(result.userId).toBe(userId);
    });
  });
});
