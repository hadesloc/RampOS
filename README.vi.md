<p align="center">
  <h1 align="center">RampOS</h1>
  <p align="center">
    <strong>Bring Your Own Rails (BYOR) — Hạ Tầng Sàn Giao Dịch Crypto/Tiền Pháp Định</strong>
  </p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square" alt="Phiên bản Rust">
  <img src="https://img.shields.io/badge/solidity-0.8.24-purple.svg?style=flat-square" alt="Phiên bản Solidity">
  <img src="https://img.shields.io/badge/node-18%2B-green.svg?style=flat-square" alt="Phiên bản Node">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-red.svg?style=flat-square" alt="Giấy phép">
</p>

<p align="center">
  <a href="#tính-năng">Tính năng</a> |
  <a href="#ảnh-chụp-màn-hình">Ảnh chụp</a> |
  <a href="#kiến-trúc">Kiến trúc</a> |
  <a href="#khởi-động-nhanh">Khởi động</a> |
  <a href="#tổng-quan-api">API</a> |
  <a href="#hợp-đồng-thông-minh">Contracts</a> |
  <a href="#sdk">SDK</a>
</p>

> 🇬🇧 [English README](README.md)

---

## Tổng quan

RampOS là một **lớp điều phối cấp production** cho các sàn giao dịch crypto/tiền pháp định. Hệ thống xử lý toàn bộ vòng đời giao dịch — từ nạp tiền pháp định, giao dịch crypto, đến rút tiền pháp định — với tính năng compliance tích hợp, account abstraction và phân tách multi-tenant.

Được xây dựng bằng **Rust** để đảm bảo hiệu năng và an toàn bộ nhớ, **Solidity** cho logic on-chain, và **Next.js** cho dashboard quản trị.

### Nguyên tắc cốt lõi

- **BYOR (Bring Your Own Rails)** — Giữ quan hệ ngân hàng của bạn, kết nối với bất kỳ ngân hàng/PSP nào
- **Zero Liability** — RampOS không bao giờ giữ tiền của khách hàng
- **Compliance-First** — Tuân thủ FATF Travel Rule & Luật AML Việt Nam 2022
- **Intent-Based** — Mọi thao tác đều là intent có chữ ký và có thể kiểm toán
- **Sổ cái kép** — Kế toán chuẩn tài chính với đường mòn kiểm toán đầy đủ

---

## Ảnh chụp màn hình

### Landing Page
> Trang marketing với hero section, feature cards, luồng hướng dẫn và demo API.

<p align="center">
  <img src="docs/screenshots/landing-hero.png" alt="Landing Page Hero" width="800">
</p>
<p align="center">
  <img src="docs/screenshots/landing-features.png" alt="Landing Page Features" width="800">
</p>

### Cổng người dùng (User Portal)
> Cổng tự phục vụ cho người dùng cuối: nạp tiền, rút tiền, quản lý tài sản và lịch sử giao dịch.

<p align="center">
  <img src="docs/screenshots/portal.png" alt="User Portal" width="800">
</p>

### Quản lý Intent
> Tìm kiếm, lọc và quản lý mọi payment intent (nạp, rút, giao dịch) theo loại và trạng thái.

<p align="center">
  <img src="docs/screenshots/intents.png" alt="Quản lý Intent" width="800">
</p>

### Dashboard Tuân thủ (Compliance)
> Quản lý hồ sơ KYC/AML — xem xét giao dịch bị gắn cờ, quản lý các vụ việc compliance.

<p align="center">
  <img src="docs/screenshots/compliance.png" alt="Compliance Dashboard" width="800">
</p>

### Sổ cái kép (Ledger)
> Giao diện kế toán thời gian thực với đường mòn kiểm toán đầy đủ cho mọi giao dịch.

<p align="center">
  <img src="docs/screenshots/ledger.png" alt="Ledger" width="800">
</p>

### Đăng nhập Admin
> Xác thực admin key bảo mật để truy cập dashboard.

<p align="center">
  <img src="docs/screenshots/admin-login.png" alt="Admin Login" width="600">
</p>

---

## Tính năng

### 🎯 Intent Engine (`ramp-core/intents`) — Trái tim của RampOS

RampOS được xây dựng xung quanh **hệ thống Intent khai báo** — người dùng nêu *điều họ muốn làm*, engine tự tìm ra *cách thực hiện tốt nhất*:

```
Intent của user: "Swap 1000 USDC trên Ethereum → USDT trên Arbitrum"
     ↓ IntentSolver đánh giá tất cả các route
     ↓ Route A: Bridge USDC → Arbitrum, rồi Swap (điểm: 0.82)
     ↓ Route B: Swap USDC→USDT trên Ethereum, rồi Bridge (điểm: 0.71)
     ↓ Chọn Route A → tạo ExecutionPlan
     ↓ WorkflowEngine lưu trữ & thực thi từng bước một cách bền vững
```

**4 loại action Intent:**
| Action | Cùng chuỗi | Khác chuỗi | Số bước |
|--------|------------|------------|---------|
| `Swap` | Swap DEX trực tiếp | Bridge+Swap hoặc Swap+Bridge (tự chọn) | 2–5 |
| `Bridge` | — | Across / Stargate (tự chọn provider) | 3 |
| `Send` | Chuyển trực tiếp | Bridge+Chuyển | 1–4 |
| `Stake` | Stake trực tiếp | Bridge+Stake | 2–5 |

**Tối ưu hóa Route thông minh:**
- Ước tính phí gas theo từng chuỗi (Ethereum, Arbitrum, Base, Optimism, Polygon)
- Ước tính thời gian bridge (5 phút L2→L2, 10 phút L1→L2, 1 giờ L2→L1)
- Chấm điểm tổng hợp: 40% tiết kiệm gas + 40% tốc độ + 20% ít bước nhất
- Slippage configurable `max_slippage_bps` (mặc định 0.5%), bảo vệ MEV
- Ràng buộc: max gas USD, max số bước, deadline thực thi

**Workflow Engine hai chế độ:**
- **Chế độ InProcess** (dev/test) — Tokio async tasks + lưu trữ state PostgreSQL tùy chọn để khôi phục sau crash
- **Chế độ Temporal** (production) — Thực thi bền vững hoàn toàn qua Temporal gRPC, tự retry, lịch sử workflow, xử lý signal (như xác nhận ngân hàng thủ công)
- **Fallback tự động** — Nếu Temporal không kết nối được, tự chuyển sang in-process

**Bù đắp & Rollback:**
- Mỗi workflow nhiều bước đều có bước bù đắp để rollback tự động khi thất bại
- Escrow đảm bảo không mất tiền trong khi thực hiện một phần
- `compensation.rs` xử lý rollback theo mô hình saga cho tất cả loại giao dịch

### 🔧 Các service lõi (`ramp-core/service`)

| Service | Mô tả |
|---------|-------|
| **Pay-in** | Vòng đời đầy đủ: khởi tạo → xác nhận ngân hàng → ghi sổ cái → webhook |
| **Pay-out** | Kiểm tra compliance → trừ sổ cái → chuyển qua rails → xác nhận |
| **Trade** | Ghi nhận giao dịch crypto với kế toán kép VND↔crypto |
| **RFQ Auction** | Thị trường đấu giá LP hai chiều cho tỷ giá tốt nhất (USDT↔VND) |
| **Escrow** | Khóa tiền trong escrow khi xử lý; tự giải phóng hoặc rollback |
| **Settlement** | Quyết toán cuối ngày giữa các nhà cung cấp rails |
| **Reconciliation** | Đối soát hàng ngày tự động giữa sổ cái và sao kê ngân hàng |
| **Exchange Rate** | Engine tỷ giá thời gian thực với spread cấu hình được |
| **Withdraw** | Luồng rút tiền đầy đủ với policy engine và giới hạn per-tenant |
| **Withdraw Policy** | Chính sách rút tiền cấu hình được theo từng tenant, từng user |
| **Webhook Delivery** | Giao hàng đảm bảo với retry, ký HMAC, và DLQ |
| **Webhook DLQ** | Dead Letter Queue cho webhook thất bại vĩnh viễn |
| **Passkey Auth** | Xác minh WebAuthn phía server cho tài khoản bảo mật passkey |
| **License** | Quản lý license per-tenant: tier, hết hạn, feature flags |
| **Onboarding** | Onboarding người dùng tối giản với tiến trình tier KYC |
| **Metrics** | Thu thập metrics nội bộ cho Prometheus export |

### 🏦 Compliance Engine (`ramp-compliance`)
- **Phân tầng KYC** — Tier 1/2/3 với giới hạn cấu hình; tích hợp Onfido và eKYC
- **AML Rules Engine** — Kiểm tra velocity, phát hiện structuring, phân tích device anomaly
- **Chấm điểm gian lận** — Trích xuất đặc trưng sẵn sàng ML, chấm điểm rủi ro và decision engine
- **Sàng lọc trừng phạt** — Tích hợp OpenSanctions với các nhà cung cấp cấu hình được
- **Quản lý vụ việc** — Quy trình đầy đủ với ghi chú, theo dõi trạng thái và giải quyết
- **Báo cáo quy định** — Tạo SAR/CTR tự động theo định dạng SBV (Ngân hàng Nhà nước Việt Nam)
- **SBV Scheduler** — Lập lịch báo cáo tự động cho Ngân hàng Nhà nước Việt Nam
- **Fuzz Testing** — Các mục tiêu fuzz chuyên biệt cho edge case của quy tắc compliance



| Hợp đồng | Mô tả |
|----------|-------------|
| `RampOSAccount.sol` | Smart Account ERC-4337 — chủ sở hữu ECDSA, thực thi batch, nâng cấp UUPS |
| `RampOSAccountFactory.sol` | Triển khai tài khoản xác định với CREATE2 |
| `RampOSPaymaster.sol` | Tài trợ gas cho giao dịch gasless |
| `VNDToken.sol` | Token ổn định cho đại diện VND on-chain |
| `PasskeySigner.sol` | Xác minh chữ ký WebAuthn/Passkey on-chain |
| `PasskeyAccountFactory.sol` | Factory tài khoản với xác thực passkey |
| `EIP7702Auth.sol` | Ủy quyền EIP-7702 cho delegation EOA |
| `EIP7702Delegation.sol` | Delegation hợp đồng thông minh cho EOA |
| `ZkKycRegistry.sol` | Registry trạng thái KYC Zero-Knowledge |
| `ZkKycVerifier.sol` | Xác minh ZK-proof cho compliance bảo vệ quyền riêng tư |

### 🌐 Hỗ trợ đa chuỗi (`ramp-core/chain`)
- **Chuỗi EVM** — Ethereum, Polygon, Arbitrum, Base, BSC
- **Solana** — SOL gốc và hỗ trợ SPL token
- **TON** — Tích hợp The Open Network
- **Cross-Chain** — Hỗ trợ bridge qua Across và Stargate
- **Tổng hợp DEX** — Định tuyến swap qua nhiều DEX
- **Tích hợp Oracle** — Chainlink price feeds với fallback provider

### 🔐 Custody & Quản lý khóa (`ramp-core/custody`)
- **Ký MPC** — Tạo khóa Multi-Party Computation và ký giao dịch
- **Policy Engine** — Chính sách phê duyệt cấu hình được theo loại thao tác
- **Xoay khóa** — Quản lý vòng đời khóa tự động

### 💰 Thanh toán & Đo lường (`ramp-core/billing`)
- **Đo lường sử dụng** — Theo dõi lời gọi API, khối lượng giao dịch per tenant
- **Tích hợp Stripe** — Thanh toán tự động dựa trên sử dụng đo lường

### 🖥️ Ứng dụng Frontend

#### Dashboard Admin (Next.js 15 + React)
- Dashboard thời gian thực với cập nhật WebSocket và biểu đồ Recharts
- Quản lý intent với tìm kiếm, lọc và theo dõi trạng thái
- Quản lý người dùng với tổng quan trạng thái KYC
- Xem xét và giải quyết vụ việc compliance
- Khám phá sổ cái kép
- Cài đặt hệ thống: thương hiệu, domain, API key, roles
- **Đa ngôn ngữ** — Tiếng Anh và Tiếng Việt (next-intl)
- **Kiểm thử E2E** — Bộ kiểm thử Playwright

#### Cổng người dùng
- Tự phục vụ nạp và rút tiền
- Tổng quan danh mục tài sản
- Lịch sử giao dịch
- Cài đặt tài khoản

#### Widget nhúng
- Widget on-ramp/off-ramp nhúng được cho bất kỳ dApp nào
- Phân phối sẵn sàng CDN

---

## Kiến trúc

### 1. Kiến trúc hệ thống tổng thể

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                              Hệ sinh thái RampOS                                  │
│                                                                                   │
│  ┌────────────────┐  REST/WS  ┌──────────────────────────────────────────────┐  │
│  │  Dashboard     │◄─────────►│              ramp-api (Axum)                 │  │
│  │  Admin         │           │   Xác thực · Rate Limit · Idempotency · OTel │  │
│  │  (Next.js 15)  │           └────────────────────┬─────────────────────────┘  │
│  └────────────────┘                                │                             │
│                                                     │ gọi xuống                  │
│  ┌────────────────┐  REST/WS  ┌─────────────────────▼─────────────────────────┐  │
│  │  Cổng người dùng│◄─────────►│             ramp-core (Rust)                  │  │
│  │  (Next.js 15)  │           │                                                │  │
│  └────────────────┘           │  ┌──────────┐  ┌──────────┐  ┌────────────┐  │  │
│                               │  │  Intent  │  │ Workflow │  │  Service   │  │  │
│  ┌────────────────┐  iframe  │  │  Engine  │  │  Engine  │  │  Layer     │  │  │
│  │  Widget nhúng  │◄────────  │  │ (Solver) │  │(Temporal)│  │ (15 svcs) │  │  │
│  └────────────────┘           │  └────┬─────┘  └────┬─────┘  └─────┬──────┘  │  │
│                               │       └──────────────┴──────────────┘         │  │
│  ┌────────────────┐  SDK/API │               │                                 │  │
│  │  Sàn giao dịch│◄──────────┤  ┌────────────▼────────────┐                  │  │
│  │  (Tenant)      │           │  │       ramp-ledger        │                  │  │
│  └────────────────┘           │  │  Sổ cái kép · Nguyên tử │                  │  │
│                               │  └────────────┬────────────┘                  │  │
│                               └───────────────┼────────────────────────────────┘  │
│                                               │                                   │
│          ┌──────────────┬────────────────────┼───────────────────────────────┐   │
│          ▼              ▼                    ▼                               ▼   │
│  ┌──────────────┐ ┌──────────┐ ┌──────────────────┐            ┌──────────────┐  │
│  │PostgreSQL 16 │ │ Redis 7  │ │ NATS JetStream    │            │ ClickHouse   │  │
│  │ (Primary+HA) │ │(Cache/RL)│ │ (Event Stream)    │            │ (Analytics)  │  │
│  └──────────────┘ └──────────┘ └──────────────────┘            └──────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────┘
         │                          │                          │
         ▼                          ▼                          ▼
┌──────────────────┐   ┌──────────────────────┐   ┌───────────────────────────────┐
│  Ngân hàng / PSP │   │  Blockchain Networks  │   │  Nhà cung cấp Compliance      │
│  VCB · MB · ...  │   │  EVM · Solana · TON   │   │  Onfido · Chainalysis         │
│  (Rails)         │   │  Bridge: Across/Gate  │   │  OpenSanctions · SBV          │
└──────────────────┘   └──────────────────────┘   └───────────────────────────────┘
```

---

### 2. Vòng đời Intent — Từ yêu cầu đến thực thi

```
  Tenant / Người dùng
      │
      │  POST /v1/intents/...
      ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                       ramp-api: Request Pipeline                                 │
│                                                                                  │
│  ┌─────────────┐   ┌─────────────────┐   ┌──────────────┐   ┌───────────────┐  │
│  │ JWT Auth    │──►│ Idempotency     │──►│ Rate Limiter │──►│ Validator     │  │
│  │ (tenant_id) │   │ (kiểm tra Redis)│   │ (per tenant) │   │ (số tiền/KYC) │  │
│  └─────────────┘   └─────────────────┘   └──────────────┘   └───────────────┘  │
└────────────────────────────────────────────────────────┬────────────────────────┘
                                                         │
                                                         ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              IntentSolver                                        │
│                                                                                  │
│  IntentSpec { action: Swap/Bridge/Send/Stake, from, to, amount, constraints }   │
│                                                                                  │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  Route Builder → đánh giá TẤT CẢ các route khả thi                       │  │
│  │                                                                           │  │
│  │  Route A: [Approve] → [Swap] → [Bridge] → [Chờ] → [Transfer]             │  │
│  │           gas: $2.1    t/g: 8 phút   bước: 5    điểm: 0.82 ✅            │  │
│  │                                                                           │  │
│  │  Route B: [Bridge] → [Chờ] → [Approve] → [Swap]                          │  │
│  │           gas: $4.7    t/g: 25 phút  bước: 4    điểm: 0.71              │  │
│  │                                                                           │  │
│  │  Công thức: 40% × (1/gas) + 40% × (1/t/g) + 20% × (1/số bước)          │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│                                      │                                           │
│                          Chọn route tốt nhất                                     │
│                                      │                                           │
│                                      ▼                                           │
│  ExecutionPlan { steps[], gas_cost, est_time, min_output (sau slippage) }       │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                            WorkflowEngine                                        │
│                                                                                  │
│  ┌─────────────────────────────┐      ┌──────────────────────────────────────┐  │
│  │  InProcess (dev/test)       │  OR  │  Temporal (production)               │  │
│  │                             │      │                                      │  │
│  │  Tokio async tasks          │      │  Thực thi bền vững (gRPC)            │  │
│  │  Lưu state vào PostgreSQL   │      │  Tự retry khi thất bại               │  │
│  │  Khôi phục sau crash        │      │  Signal: xác nhận ngân hàng          │  │
│  │                             │      │  Lịch sử workflow đầy đủ             │  │
│  └─────────────────────────────┘      └──────────────────────────────────────┘  │
│                                                                                  │
│                  Tự động fallback về in-process nếu Temporal mất kết nối        │
└──────────────────────────────────────┬──────────────────────────────────────────┘
                                       │
                         thực thi từng bước tuần tự
                                       │
                    ┌──────────────────┴────────────────────┐
                    ▼                                        ▼
          ┌──────────────────────┐             ┌────────────────────────────┐
          │  Bước on-chain       │             │  Compensation Workflow     │
          │  Approve · Swap      │             │  (chạy khi có thất bại)    │
          │  Bridge · Stake      │             │                            │
          │  Transfer · Wait     │             │  Bước N thất bại?          │
          └──────────────────────┘             │  → Chạy bù đắp N-1        │
                                               │  → Chạy bù đắp N-2        │
                                               │  → Giải phóng escrow       │
                                               └────────────────────────────┘
```

---

### 3. Luồng Pay-in (Tiền pháp định → Crypto)

```
  Người dùng (trên sàn)       RampOS                        Ngân hàng / Blockchain
       │                         │                                   │
       │  1. Tạo Pay-in Intent   │                                   │
       │────────────────────────►│                                   │
       │                         │  2. Kiểm tra AML                  │
       │                         │  3. Khóa tiền vào Escrow          │
       │                         │  4. Tạo mã tham chiếu ngân hàng   │
       │◄────────────────────────│                                   │
       │  5. Hiện mã VA / QR     │                                   │
       │                         │                                   │
       │  6. Người dùng chuyển VND                                   │
       │──────────────────────────────────────────────────────────► │
       │                         │                                   │
       │                         │  7. Webhook / polling ngân hàng  │
       │                         │◄──────────────────────────────────│
       │                         │                                   │
       │                         │  8. Signal: BankConfirmation      │
       │                         │  (WorkflowEngine nhận tín hiệu)   │
       │                         │                                   │
       │                         │  9.  Kiểm tra AML sau giao dịch   │
       │                         │  10. Ghi sổ cái kép               │
       │                         │      DR: bank_clearing            │
       │                         │      CR: user_balance             │
       │                         │  11. Giải phóng Escrow            │
       │                         │  12. Ghi có crypto cho user       │
       │                         │──────────────────────────────────►│
       │                         │  13. Gửi webhook đến tenant        │
       │◄────────────────────────│                                   │
       │  14. UI cập nhật (WebSocket)                                │
```

---

### 4. Luồng Pay-out (Crypto → Tiền pháp định) với Cổng Compliance

```
  Người dùng (yêu cầu rút tiền)
       │
       │  POST /v1/intents/payout
       ▼
  ┌────────────────────────────────────────────────────────────────────────────────┐
  │                       Cổng Compliance (BẮT BUỘC)                               │
  │                                                                                 │
  │  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌────────────┐  │
  │  │ Kiểm tra KYC   │  │ AML Velocity   │  │ Sàng lọc       │  │ Chấm điểm │  │
  │  │ Tier ≥ yêu cầu │  │ 24h/7d/30d     │  │ trừng phạt     │  │ gian lận  │  │
  │  │ cho số tiền    │  │ giới hạn       │  │ OFAC·UN·OpenS  │  │ ML score  │  │
  │  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘  └──────┬────┘  │
  │          └───────────────────┴───────────────────┴────────────────────┘        │
  │                                         │                                       │
  │                         Tất cả kiểm tra ĐẠT?                                   │
  │                      KHÔNG ─────────────────── CÓ                              │
  │                        │                         │                              │
  │               ┌─────────▼──────┐        ┌────────▼──────────────────────────┐  │
  │               │ Tạo vụ việc    │        │  Tiến hành thực thi               │  │
  │               │ compliance để  │        └───────────────────────────────────┘  │
  │               │ xem xét thủ   │                                               │
  │               │ công           │                                               │
  │               └────────────────┘                                               │
  └────────────────────────────────────────────────────────────────────────────────┘
                                              │
                                              ▼
  ┌────────────────────────────────────────────────────────────────────────────────┐
  │                         Thực thi Payout                                        │
  │                                                                                │
  │  1. Kiểm tra Withdraw Policy (giới hạn per-tenant, cooldown, blacklist)        │
  │  2. Trừ số dư (Sổ cái kép: DR user_balance, CR bank_settling)                 │
  │  3. Khóa vào Escrow cho đến khi ngân hàng xác nhận                            │
  │  4. Gửi lệnh chuyển tiền đến ngân hàng/rails                                  │
  │  5. Chờ xác nhận rails (polling / webhook)                                     │
  │  6. Thành công → Giải phóng escrow, ghi sổ cái cuối                          │
  │  7. Thất bại → Bù đắp: hoàn đảo sổ cái, hoàn tiền về user                    │
  │  8. Gửi webhook về tenant                                                      │
  └────────────────────────────────────────────────────────────────────────────────┘
```

---

### 5. Hạ tầng Production

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                       Kubernetes Cluster Production                              │
│                                                                                  │
│  ┌───────────┐    ┌───────────────────────────────────────────────────────────┐ │
│  │  ArgoCD   │───►│                  ramp-api Pods                            │ │
│  │  GitOps   │    │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │ │
│  │  Deploy   │    │  │  Pod 1   │  │  Pod 2   │  │  Pod 3   │  │  ...     │ │ │
│  └───────────┘    │  │  (Axum)  │  │  (Axum)  │  │  (Axum)  │  │ HPA:3-20 │ │ │
│                   │  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │ │
│  ┌───────────┐    └───────────────────────────────────────────────────────────┘ │
│  │Prometheus │                                                                   │
│  │  Rules    │    ┌───────────────────────────────────────────────────────────┐ │
│  │  Grafana  │    │                    Tầng dữ liệu                           │ │
│  │Dashboards │    │                                                            │ │
│  └───────────┘    │  ┌──────────────────────────┐  ┌──────────────────────┐  │ │
│                   │  │  PostgreSQL 16 HA         │  │  Redis 7 Cluster     │  │ │
│  ┌───────────┐    │  │  Primary ──► Replica      │  │  Cache · Sessions    │  │ │
│  │  S3 Backup│◄───│  │  PgBouncer (pool: 100)    │  │  Rate Limit · Idem.  │  │ │
│  │  Cron Job │    │  │  Tự động failover         │  └──────────────────────┘  │ │
│  │ (hàng ngày│    │  └──────────────────────────┘                             │ │
│  └───────────┘    │  ┌──────────────────────────┐  ┌──────────────────────┐  │ │
│                   │  │  NATS JetStream           │  │  ClickHouse          │  │ │
│                   │  │  Event streaming          │  │  Analytics · Báo cáo │  │ │
│                   │  │  Durable messages         │  │  Dữ liệu báo cáo SBV │  │ │
│                   │  └──────────────────────────┘  └──────────────────────┘  │ │
│                   └───────────────────────────────────────────────────────────┘ │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │  Network Policy cấp pod · Sẵn sàng mTLS · HPA + PDB được cấu hình          ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────────┘
         │              │               │                    │
         ▼              ▼               ▼                    ▼
   OpenTelemetry   Prometheus       Grafana            AlertManager
   (traces/logs)   (metrics)       (dashboards)       (PagerDuty/Slack)
```



### Workspace Rust (7 crates)

| Crate | Mô tả | Phụ thuộc chính |
|-------|-------------|-----------------|
| `ramp-api` | API Gateway REST | Axum 0.7, Tower, OpenTelemetry |
| `ramp-core` | Logic nghiệp vụ, state machine, 119 modules | Tokio, SQLx, async-nats |
| `ramp-ledger` | Kế toán sổ cái kép | rust_decimal |
| `ramp-compliance` | KYC/AML/KYT, 64 modules | Fuzz testing, tạo báo cáo |
| `ramp-aa` | Account Abstraction (ERC-4337) | Alloy |
| `ramp-adapter` | SDK tích hợp Ngân hàng/PSP | Pluggable provider trait |
| `ramp-common` | Kiểu dùng chung & lỗi | serde, thiserror |

### Cấu trúc dự án

```
rampos/
├── crates/                # 7 Rust workspace crates
│   ├── ramp-api/           # HTTP API (Axum) — 101 files
│   ├── ramp-core/          # Logic nghiệp vụ — 126 files
│   │   ├── billing/         # Đo lường, Stripe
│   │   ├── bridge/          # Across, Stargate
│   │   ├── chain/           # EVM, Solana, TON, swaps
│   │   ├── crosschain/      # Executor, relayer
│   │   ├── custody/         # Khóa MPC, ký, policies
│   │   ├── intents/         # Solver, thực thi, số dư thống nhất
│   │   ├── oracle/          # Chainlink, fallback
│   │   └── ...
│   ├── ramp-compliance/    # KYC/AML engine — 75 files
│   │   ├── aml/             # Phát hiện device anomaly
│   │   ├── fraud/           # Chấm điểm, phân tích, đặc trưng
│   │   ├── kyc/             # Onfido, eKYC, phân tầng
│   │   ├── kyt/             # Tích hợp Chainalysis
│   │   ├── sanctions/       # OpenSanctions
│   │   └── reports/         # SAR/CTR, định dạng SBV
│   ├── ramp-ledger/        # Sổ cái kép
│   ├── ramp-aa/            # Account Abstraction
│   ├── ramp-adapter/       # SDK Rails adapter
│   └── ramp-common/        # Kiểu dùng chung
├── contracts/              # 10 hợp đồng Solidity (Foundry)
│   ├── src/
│   │   ├── passkey/         # WebAuthn on-chain
│   │   ├── eip7702/         # Delegation EOA
│   │   └── zk/             # Zero-Knowledge KYC
│   ├── test/               # 18 file kiểm thử
│   └── script/             # 8 script triển khai
├── sdk/                    # TypeScript SDK
├── sdk-go/                 # Go SDK
├── sdk-python/             # Python SDK
├── packages/widget/        # Widget nhúng
├── frontend/               # Dashboard Admin (Next.js 15)
├── frontend-landing/       # Trang marketing
├── migrations/             # 67 migration PostgreSQL
├── k8s/                    # Kubernetes (Kustomize)
│   ├── base/               # Manifests lõi, Postgres HA, PgBouncer
│   ├── jobs/               # Backup jobs (Postgres, Redis, NATS → S3)
│   ├── monitoring/         # Prometheus, Grafana
│   └── overlays/           # Cấu hình Staging/Production
├── monitoring/             # Grafana dashboards, Prometheus rules
├── argocd/                 # Triển khai GitOps
└── docs/                   # Tài liệu
```

---

## Khởi động nhanh

### Yêu cầu

| Thành phần | Phiên bản | Mục đích |
|-----------|---------|---------|
| Rust | 1.75+ | Backend API |
| PostgreSQL | 16+ | Cơ sở dữ liệu chính |
| Redis | 7+ | Cache, rate limiting, idempotency |
| NATS | 2.10+ | Event streaming |
| Node.js | 18+ | Frontend, SDKs |
| Foundry | Mới nhất | Hợp đồng thông minh |

### Cài đặt

```bash
# Clone repository
git clone https://github.com/hadesloc/RampOS.git
cd RampOS

# Sao chép cấu hình môi trường
cp .env.example .env
# Chỉnh sửa .env — điền mật khẩu (xem comment để tạo)

# Khởi động hạ tầng
docker-compose up -d postgres redis nats

# Chạy migration cơ sở dữ liệu
cargo install sqlx-cli
sqlx migrate run

# Build và chạy
cargo build --release
cargo run --release --package ramp-api
```

API server sẽ chạy tại `http://localhost:8080`.

### Sử dụng Docker

```bash
# Full stack
docker-compose up --build

# Hoặc chỉ hạ tầng
docker-compose up -d postgres redis nats clickhouse
docker-compose up ramp-api
```

### Frontend (Dashboard Admin)

```bash
cd frontend
cp .env.local.example .env.local
npm install
npm run dev
# → http://localhost:3000
```

---

## Tổng quan API

### Thị trường Đấu giá RFQ — Khám phá giá hai chiều

Lớp **RFQ (Request For Quote)** tạo ra thị trường đấu giá LP cạnh tranh, nơi các Nhà cung cấp Thanh khoản thi đua để đưa ra tỷ giá tốt nhất:

```
OFF-RAMP (USDT → VND):  User tạo RFQ → LP cạnh tranh mua USDT → ai trả nhiều VND nhất thắng
ON-RAMP  (VND → USDT):  User tạo RFQ → LP cạnh tranh bán USDT → ai bán rẻ VND nhất thắng
```

| Phương thức | Endpoint | Auth | Mô tả |
|-------------|----------|------|-------|
| `POST` | `/v1/portal/rfq` | Portal JWT | Tạo RFQ (OFFRAMP hoặc ONRAMP) |
| `GET`  | `/v1/portal/rfq/:id` | Portal JWT | Xem RFQ + danh sách bid + tỷ giá tốt nhất |
| `POST` | `/v1/portal/rfq/:id/accept` | Portal JWT | Chấp nhận bid tốt nhất → MATCHED |
| `POST` | `/v1/portal/rfq/:id/cancel` | Portal JWT | Huỷ RFQ đang mở |
| `POST` | `/v1/lp/rfq/:rfq_id/bid` | X-LP-Key | LP đặt giá |
| `GET`  | `/v1/admin/rfq/open` | Admin Key | Liệt kê tất cả phiên đấu giá đang mở |
| `POST` | `/v1/admin/rfq/:id/finalize` | Admin Key | Kích hoạt ghép lệnh thủ công |

### Sơ đồ Luồng Đấu giá RFQ

```
  ┌───────────────────────────────────────────────────────────────────────────────────┐
  │                    Kiến trúc RFQ Auction Market                                     │
  │                                                                                  │
  │   ┌──────────────┐  1. Tạo RFQ  ┌──────────────────────────────────────────┐ │
  │   │  Cổng người   │ ──────────────►  │          RFQ Request (state: OPEN)          │ │
  │   │  dùng (Portal) │                │  ┌───────────────────────────────────┐  │ │
  │   │               │                │  │  direction: OFFRAMP | ONRAMP        │  │ │
  │   │               │                │  │  crypto_amount: 100 USDT            │  │ │
  │   │               │                │  │  expires_at: +5 phút               │  │ │
  │   └──────────────┘                │  └───────────────────────────────────┘  │ │
  │          │                         └──────────────────────────────────────────┘ │
  │          │                                             │                         │
  │          │                         2. Event NATS "rfq.created" gửi cho LP       │
  │          │                                             │                         │
  │          │                         ┌───────────────────┼───────────────────┐    │
  │          │                         ▼                   ▼                   ▼    │
  │          │                  ┌──────────┐     ┌──────────┐     ┌──────────┐│
  │          │   3. Đặt giá  │  LP Acme   │     │ LP FastEx  │     │ LP VietFX  ││
  │          │   ◄───────────  └─────┬─────┘     └─────┬─────┘     └──────────┘│
  │          │            26.000 VND/U   │  25.800 VND/U   │                        │
  │          │                           ▼                 ▼                        │
  │          │                 ┌──────────────────────────────────────────┐  │
  │          │                 │              Bảng rfq_bids                      │  │
  │   4. Xem │                 │  ┌───────────────────────────────────────┐ │  │
  │  tỷ giá  │                 │  │ bid#1 lp_acme  rate=26000 ← Tốt ✅  │ │  │
  │  tốt nhất│ ◄────────────── │  │ bid#2 lp_fastex rate=25800            │ │  │
  │          │                 │  └───────────────────────────────────────┘ │  │
  │          │                 └──────────────────────────────────────────┘  │
  │          │                                                                        │
  │          │  5. POST /accept ──────────────────────────────────────────►            │
  │          │                         state → MATCHED | event "rfq.matched" → NATS │
  │          │                                                                        │
  │          │  6. final_rate = 26.000 VND/USDT ◄──────────────────────────────   │
  └───────────────────────────────────────────────────────────────────────────────────┘

  OFFRAMP: chọn rate cao nhất (user bán USDT → được nhiều VND nhất)
  ONRAMP:  chọn rate thấp nhất (user mua USDT → trả ít VND nhất)

  Job tự expire: chạy mỗi 60s → OPEN + expires_at < NOW() → EXPIRED
```

| Phương thức | Endpoint | Mô tả |
|--------|----------|-------------|
| `POST` | `/v1/intents/payin` | Tạo intent nạp tiền pháp định |
| `POST` | `/v1/intents/payin/confirm` | Xác nhận nạp tiền từ ngân hàng |
| `POST` | `/v1/intents/payout` | Tạo intent rút tiền pháp định |
| `POST` | `/v1/events/trade-executed` | Ghi nhận giao dịch crypto |
| `GET` | `/v1/intents/{id}` | Lấy trạng thái intent |

### Ví dụ: Tạo Pay-in

```bash
curl -X POST http://localhost:8080/v1/intents/payin \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-123" \
  -d '{
    "user_id": "usr_123",
    "amount_vnd": 1000000,
    "rails_provider": "VIETCOMBANK"
  }'
```

---

## Hợp đồng thông minh

### Triển khai

```bash
cd contracts
forge install
forge script script/Deploy.s.sol --rpc-url sepolia --broadcast
forge verify-contract <ADDRESS> RampOSAccountFactory --chain sepolia
```

### Điểm nổi bật

| Tính năng | Hợp đồng | Chuẩn |
|---------|----------|----------|
| Smart Account | `RampOSAccount.sol` | ERC-4337 |
| Tài trợ Gas | `RampOSPaymaster.sol` | ERC-4337 |
| Đăng nhập Passkey | `PasskeySigner.sol` | WebAuthn |
| Delegation EOA | `EIP7702Delegation.sol` | EIP-7702 |
| KYC bảo mật | `ZkKycRegistry.sol` | ZK Proofs |
| Stablecoin VND | `VNDToken.sol` | ERC-20 |

---

## SDK

### TypeScript

```typescript
import { RampOSClient } from '@rampos/sdk';

const client = new RampOSClient({
  apiKey: 'your_api_key',
  baseUrl: 'http://localhost:8080'
});

const payin = await client.payins.create({
  userId: 'usr_123',
  amountVnd: 1000000
});
```

### Go

```go
import "github.com/hadesloc/rampos-go"

client := rampos.NewClient("your_api_key")
payin, err := client.Payins.Create(ctx, &rampos.CreatePayinRequest{
    UserID:    "usr_123",
    AmountVND: 1000000,
})
```

### Python

```python
from rampos import RampOSClient

client = RampOSClient(api_key="your_api_key")
payin = client.payins.create(user_id="usr_123", amount_vnd=1000000)
```

---

## Công nghệ

| Tầng | Công nghệ |
|-------|------------|
| **Backend** | Rust, Tokio, Axum, SQLx |
| **Cơ sở dữ liệu** | PostgreSQL 16 (35 migrations) |
| **Cache** | Redis 7 |
| **Messaging** | NATS JetStream |
| **Phân tích** | ClickHouse |
| **Hợp đồng thông minh** | Solidity 0.8.24, Foundry |
| **Frontend** | Next.js 15, React, Tailwind CSS, Recharts |
| **Mật mã** | AES-256-GCM, Argon2, HMAC-SHA256, JWT |
| **Hạ tầng** | Kubernetes, ArgoCD, PgBouncer |
| **Quan sát** | OpenTelemetry, Prometheus, Grafana |
| **Kiểm thử** | Playwright (E2E), Vitest, Foundry fuzz |

---

## Hạ tầng

### Kubernetes (Sẵn sàng Production)
- **PostgreSQL HA** — Primary + streaming replica với tự động failover
- **PgBouncer** — Connection pooling cho đồng thời cao
- **Sao lưu tự động** — Postgres, Redis, NATS → S3 với chính sách lưu giữ
- **Network Policies** — Phân tách cấp pod
- **HPA/PDB** — Auto-scaling và disruption budget
- **Kustomize Overlays** — Cấu hình Staging và Production

### Bảo mật
- Mã hóa AES-256-GCM cho dữ liệu nhạy cảm lưu trữ
- Hash mật khẩu Argon2
- Xác minh chữ ký webhook HMAC-SHA256
- Xác thực JWT với phân quyền theo vai trò
- Bảo vệ rate limiting và request timeout
- NetworkPolicies Kubernetes và kiến trúc sẵn sàng mTLS

---

## Đóng góp

Mọi đóng góp đều được chào đón! Xem [CONTRIBUTING.md](CONTRIBUTING.md) để biết hướng dẫn.

```bash
git checkout -b feature/tinh-nang-moi
git commit -m 'feat: thêm tính năng mới'
git push origin feature/tinh-nang-moi
# Mở Pull Request
```

---

## Giấy phép

Dự án này được cấp phép theo **GNU Affero General Public License v3.0 (AGPL-3.0)**.

Nghĩa là:
- ✅ Bạn có thể xem, sửa đổi và sử dụng code này cho **mục đích cá nhân và giáo dục**
- ✅ Bạn có thể đóng góp trở lại dự án này
- ⚠️ Nếu bạn dùng phần mềm này để cung cấp **dịch vụ mạng (SaaS)**, bạn **phải** công bố mã nguồn đầy đủ theo AGPL-3.0
- ❌ Bạn **không thể** sử dụng trong sản phẩm thương mại độc quyền/đóng mà không công khai toàn bộ codebase

Xem [LICENSE](LICENSE) để biết nội dung giấy phép đầy đủ.

---

<p align="center">
  Xây dựng với Rust 🦀 | Mã nguồn mở
</p>
