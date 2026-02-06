# RampOS - Hướng dẫn Test Thực Tế

## Tổng quan

Hệ thống RampOS cho phép:
1. **On-ramp**: Nạp VND (qua bank) → Nhận VND token trên blockchain
2. **Trade**: Đổi VND token ↔ Crypto (USDC, ETH, etc.)
3. **Off-ramp**: Rút crypto → Nhận VND về bank

## Bước 1: Chuẩn bị môi trường

### 1.1. Cài đặt dependencies

```bash
# Backend
cd crates/ramp-api
cargo build

# Frontend
cd frontend
npm install

# Contracts
cd contracts
forge install
```

### 1.2. Tạo ví deployer

```bash
# Tạo ví mới (lưu private key an toàn!)
cast wallet new

# Output:
# Address: 0x...
# Private key: 0x...
```

### 1.3. Lấy testnet ETH (Base Sepolia)

1. Truy cập: https://www.coinbase.com/faucets/base-ethereum-sepolia-faucet
2. Nhập địa chỉ ví deployer
3. Nhận ~0.1 ETH testnet

### 1.4. Cấu hình contracts/.env

```bash
cd contracts
cp .env.example .env
```

Chỉnh sửa `.env`:
```
DEPLOYER_PRIVATE_KEY=0x<your_private_key>
BASESCAN_API_KEY=<optional, for verification>
```

## Bước 2: Deploy Smart Contracts

```bash
cd contracts

# Build
forge build

# Deploy lên Base Sepolia
forge script script/DeployAll.s.sol:DeployAllScript \
  --rpc-url https://sepolia.base.org \
  --broadcast \
  -vvv
```

Sau khi deploy, lưu lại các địa chỉ:
- VND Token: `0x...`
- Account Factory: `0x...`
- Paymaster: `0x...`

## Bước 3: Cấu hình Backend

### 3.1. Cập nhật root .env

```bash
cp .env.example .env
```

Chỉnh sửa:
```env
# Database
POSTGRES_PASSWORD=<generate_secure_password>

# AA Service
BASE_SEPOLIA_RPC_URL=https://sepolia.base.org
PIMLICO_API_KEY=<your_pimlico_key>  # Get from https://dashboard.pimlico.io
BUNDLER_URL=https://api.pimlico.io/v2/84532/rpc?apikey=<your_key>

# Contract addresses (from deployment)
VND_TOKEN_ADDRESS=0x...
ACCOUNT_FACTORY_ADDRESS=0x...
PAYMASTER_ADDRESS=0x...

# Paymaster signer (same as deployer for testing)
PAYMASTER_SIGNER_KEY=<deployer_private_key>
```

### 3.2. Khởi động infrastructure

```bash
docker-compose up -d
```

### 3.3. Chạy backend

```bash
cd crates/ramp-api
cargo run
```

## Bước 4: Chạy Frontend

```bash
cd frontend
npm run dev
```

Truy cập: http://localhost:3000

## Bước 5: Test Flow

### 5.1. Đăng ký tài khoản

1. Vào http://localhost:3000/portal/register
2. Nhập email
3. Đăng ký bằng Passkey hoặc Magic Link

### 5.2. Tạo Smart Account

1. Vào Portal Dashboard
2. Click "Create Wallet"
3. Hệ thống tạo Smart Account (AA) trên Base Sepolia

### 5.3. Nạp VND (giả lập bank)

**Option A: Dùng script**
```bash
./scripts/simulate-bank-webhook.sh 1000000 "YOUR_REFERENCE_CODE"
```

**Option B: Dùng curl**
```bash
curl -X POST http://localhost:8080/v1/webhooks/bank/vietqr \
  -H "Content-Type: application/json" \
  -d '{
    "transactionId": "BANK123456",
    "referenceCode": "YOUR_REFERENCE_CODE",
    "amount": 1000000,
    "currency": "VND",
    "senderBankCode": "VCB",
    "senderAccount": "1234567890",
    "senderName": "NGUYEN VAN A",
    "status": "SUCCESS"
  }'
```

### 5.4. Kiểm tra balance

Vào Portal → Wallet → Xem balance VND

### 5.5. Nạp VND Token (on-chain)

Sau khi có balance VND trong hệ thống:
1. Backend sẽ gọi `VNDToken.mintWithReference()`
2. Token được mint vào Smart Account của user

### 5.6. Test Withdraw

1. Vào Portal → Withdraw
2. Chọn số tiền VND
3. Nhập thông tin bank
4. Confirm

## Bước 6: Test E2E tự động

```bash
./scripts/test-e2e-flow.sh
```

## Troubleshooting

### Lỗi "Smart account service not available"

- Kiểm tra `PAYMASTER_SIGNER_KEY` đã set trong .env
- Kiểm tra AA service được khởi tạo đúng

### Lỗi khi deploy contract

- Kiểm tra có đủ ETH testnet trong ví deployer
- Kiểm tra RPC URL đúng

### Lỗi bundler

- Kiểm tra Pimlico API key
- Thử với bundler khác: Alchemy, StackUp

## Môi trường Production

Khi deploy production:

1. **Contracts**: Deploy lên Base Mainnet
2. **RPC**: Dùng Alchemy/Infura production endpoint
3. **Bundler**: Dùng Pimlico production tier
4. **Bank**: Kết nối với VietQR/Napas thật
5. **Security**:
   - Sử dụng HSM cho private keys
   - Enable rate limiting
   - Configure proper CORS
