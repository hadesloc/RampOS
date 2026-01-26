# RampOS (BYOR) — Whitepaper kỹ thuật v1.0

**Giải pháp Orchestrator + Compliance + Account Abstraction Kit cho sàn giao dịch crypto/VND tại Việt Nam**
**Ngày:** 22/01/2026 (GMT+7)

---

## 0) Tóm tắt điều hành

Việt Nam đang triển khai **thí điểm thị trường tài sản mã hoá** và bắt đầu **tiếp nhận hồ sơ cấp phép dịch vụ “tổ chức thị trường giao dịch tài sản mã hoá” từ 20/01/2026**. ([baochinhphu.vn][1]) Trong khuôn khổ thí điểm, **việc chào bán/phát hành/giao dịch/thanh toán tài sản mã hoá phải thực hiện bằng Đồng Việt Nam (VND)**. ([xaydungchinhsach.chinhphu.vn][2])

Điều này làm nảy sinh một nhu cầu rất “đinh”: các sàn mới (hoặc sàn chuẩn bị xin phép) cần một lớp hạ tầng **nạp/rút VND, đối soát, audit trail, KYC/AML/KYT, quản trị rủi ro, và (tuỳ phạm vi) nạp/rút on-chain đa chuỗi** — nhưng nhiều sàn không muốn (hoặc không thể) tự xây đủ “phần xương sống vận hành” ngay từ ngày đầu.

**RampOS (BYOR – Bring Your Own Rails)** giải bài toán đó bằng cách:

* Sàn **giữ nguyên đối tác ngân hàng/PSP** của họ (rails VND), RampOS **không đụng tiền**, không ôm liability.
* RampOS cung cấp **Orchestrator** (state machine chuẩn hóa + ledger + đối soát + SLA), **Compliance Pack** (KYC/AML/KYT + case management + audit/report), và **AA/Multi-chain Kit** (UX gasless, batch, session policy) dựa trên các chuẩn AA phổ biến như **ERC-4337** ([Ethereum Improvement Proposals][3]) và hướng tương thích tương lai như **EIP-7702** ([Ethereum Improvement Proposals][4]), dùng **EIP-712** cho chữ ký dữ liệu có cấu trúc. ([Ethereum Improvement Proposals][5])

---

## 1) Bối cảnh & ràng buộc (đặc biệt quan trọng cho crypto/VND)

### 1.1 Ràng buộc VND

Trong Nghị quyết thí điểm, nguyên tắc nêu rõ: **“chào bán, phát hành, giao dịch, thanh toán tài sản mã hóa phải được thực hiện bằng Đồng Việt Nam.”** ([xaydungchinhsach.chinhphu.vn][2])
=> “On/off-ramp” theo nghĩa truyền thống (VND ↔ stablecoin) có thể bị thu hẹp, **nhưng ramp theo nghĩa vận hành** (nạp/rút VND, đối soát, kiểm soát rủi ro, audit) lại trở thành “bắt buộc”.

### 1.2 Điều kiện vận hành & tuân thủ

Nghị quyết cũng nhấn mạnh: chỉ tổ chức được cấp phép mới được cung cấp dịch vụ liên quan và hoạt động marketing; và người tham gia phải tuân thủ phòng chống rửa tiền, an ninh mạng, bảo vệ dữ liệu… ([xaydungchinhsach.chinhphu.vn][2])
=> Sàn muốn sống khoẻ phải có: **KYC/AML/KYT + nhật ký kiểm toán + quy trình xử lý sự cố/flag**.

### 1.3 Khung AML/KYT tham chiếu

RampOS thiết kế theo nguyên tắc “risk-based” (quản lý theo rủi ro), bám các chuẩn quốc tế cho tài sản ảo/VASP như hướng dẫn cập nhật của **FATF**. ([FATF][6]) Đồng thời tương thích với trục pháp lý nội địa như **Luật Phòng, chống rửa tiền 2022**. ([THƯ VIỆN PHÁP LUẬT][7])
*(Lưu ý: whitepaper là tài liệu kỹ thuật, không phải tư vấn pháp lý.)*

---

## 2) Mục tiêu thiết kế

1. **BYOR**: sàn tự chọn rails VND (ngân hàng/PSP). RampOS chỉ điều phối + kiểm soát + audit.
2. **Chuẩn hoá state machine**: mọi giao dịch đi qua một “máy trạng thái” thống nhất → dễ tích hợp, dễ vận hành, dễ báo cáo.
3. **Ledger chuẩn tài chính**: ghi sổ hai bút toán (double-entry), bất biến, dễ đối soát.
4. **Security-first**: zero-trust nội bộ, khoá/secret quản trị chặt, API an toàn theo best practices (ví dụ OWASP API Security Top 10). ([OWASP Foundation][8])
5. **Multi-chain “đúng chỗ”**: AA để tăng UX + giảm friction; không trộn lẫn với logic VND settlement.

---

## 3) Tổng quan hệ thống

### 3.1 Các tác nhân

* **Exchange (Sàn)**: UI/UX, trading engine, custody nội bộ, chọn bank/PSP.
* **Rails Provider (Bank/PSP của sàn)**: pay-in/pay-out VND, webhook xác nhận.
* **RampOS Orchestrator**: điều phối intent, chạy policy, quản lý state, ledger, đối soát, audit.
* **Compliance Engine**: KYC tiering, AML rules, KYT hooks, case management.
* **AA & Chain Services (tuỳ chọn)**: bundler/paymaster, chain adapters, tx monitor.

### 3.2 Phân tách Control Plane vs Data Plane

* **Control Plane**: cấu hình rails, chính sách rủi ro, limit, whitelist token/chain, khoá API, SLA.
* **Data Plane**: xử lý giao dịch thực tế (intent → webhook → ledger → trạng thái → webhook trả sàn).

---

## 4) Kiến trúc logic: “Intent → State Machine → Ledger → Reconciliation”

### 4.1 Khái niệm “Intent”

Intent là “ý định nghiệp vụ” do sàn gửi tới RampOS (hoặc user qua sàn), ví dụ:

* `PAYIN_VND`
* `PAYOUT_VND`
* `TRADE_EXECUTED` (crypto/VND)
* (tuỳ chọn) `DEPOSIT_ONCHAIN`, `WITHDRAW_ONCHAIN`

Intent được ký bằng chữ ký dữ liệu cấu trúc theo **EIP-712** để đảm bảo không thể bị sửa mà không bị phát hiện. ([Ethereum Improvement Proposals][5])

> Gợi ý nâng cao: nếu cần “signature dùng chung đa chuỗi”, có thể tham chiếu các đề xuất về crosschain EIP-712 signatures như ERC-7964. ([Ethereum Improvement Proposals][9])

### 4.2 State Machine (cốt lõi để vận hành)

**Pay-in VND**

* `PAYIN_CREATED` → `INSTRUCTION_ISSUED` → `FUNDS_PENDING` → `FUNDS_CONFIRMED` → `VND_CREDITED` → `COMPLETED`
  Nhánh lỗi: `EXPIRED`, `MISMATCHED_AMOUNT`, `SUSPECTED_FRAUD`, `MANUAL_REVIEW`

**Pay-out VND**

* `PAYOUT_CREATED` → `POLICY_APPROVED` → `PAYOUT_SUBMITTED` → `PAYOUT_CONFIRMED` → `COMPLETED`
  Nhánh lỗi: `REJECTED_BY_POLICY`, `BANK_REJECTED`, `TIMEOUT`, `MANUAL_REVIEW`

**Trade event (crypto/VND)**

* `TRADE_RECORDED` → `POST_TRADE_CHECKED` → `SETTLED_LEDGER` → `COMPLETED`

### 4.3 Ledger (double-entry) & bất biến

Mỗi hành động tạo **2 ledger entries**:

* Ví dụ pay-in 10,000,000 VND:

  * Debit: `Clearing:BankPending`
  * Credit: `Liability:UserVND`
* Khi bank confirm:

  * Debit: `Asset:Bank`
  * Credit: `Clearing:BankPending`

Mục tiêu:

* không “mất tiền” do bug trạng thái,
* đối soát dễ,
* audit log rõ ràng.

---

## 5) Giao thức tích hợp (API/Events/Webhooks)

### 5.1 Nguyên tắc kỹ thuật

* **Idempotency**: mọi endpoint ghi trạng thái đều yêu cầu `Idempotency-Key`.
* **At-least-once webhooks**: retry backoff, dedupe bằng `eventId`.
* **Webhook signing**: HMAC + timestamp + replay protection.
* **Outbox pattern**: ghi DB trước, phát event sau để tránh “ghi sổ xong không bắn webhook”.

### 5.2 Orchestrator API (đầu sàn gọi)

#### (A) Tạo pay-in

```http
POST /v1/intents/payin
Idempotency-Key: 0d9b...
X-Signature: t=..., v1=...
Content-Type: application/json
```

```json
{
  "tenantId": "exchange_abc",
  "userId": "u_123",
  "amountVnd": 10000000,
  "railsProvider": "partner_bank_xyz",
  "metadata": {
    "channel": "bank_transfer",
    "note": "topup"
  }
}
```

**Response**

```json
{
  "intentId": "pi_01H...",
  "referenceCode": "ABC123456",
  "virtualAccount": {
    "bank": "XYZ",
    "accountNumber": "1234567890",
    "accountName": "EXCHANGE ABC - VA"
  },
  "expiresAt": "2026-01-22T10:30:00+07:00",
  "status": "INSTRUCTION_ISSUED"
}
```

#### (B) Nhận confirm pay-in (từ adapter của sàn)

```http
POST /v1/intents/payin/confirm
Idempotency-Key: ...
```

```json
{
  "tenantId": "exchange_abc",
  "referenceCode": "ABC123456",
  "status": "FUNDS_CONFIRMED",
  "bankTxId": "BTX_9988",
  "amountVnd": 10000000,
  "settledAt": "2026-01-22T10:02:11+07:00",
  "rawPayloadHash": "sha256:..."
}
```

#### (C) Gửi trade executed (sàn → RampOS)

```http
POST /v1/events/trade-executed
Idempotency-Key: ...
```

```json
{
  "tenantId": "exchange_abc",
  "tradeId": "t_7788",
  "userId": "u_123",
  "symbol": "BTC/VND",
  "price": 1150000000,
  "vndDelta": -10000000,
  "cryptoDelta": 0.0000087,
  "ts": "2026-01-22T10:05:00+07:00"
}
```

### 5.3 Webhook về sàn (RampOS → Exchange)

Các event chính:

* `intent.status.changed`
* `risk.review.required`
* `kyc.flagged`
* `recon.batch.ready`

---

## 6) Rails Adapter (BYOR) — chuẩn hoá để sàn cắm bank/PSP nào cũng được

RampOS cung cấp **Adapter SDK** (TypeScript/Go/Rust) với interface:

* `createPayinInstruction(user, amount, ref)`
* `parsePayinWebhook(payload) -> ConfirmPayin`
* `initiatePayout(userBankToken, amount, ref)`
* `parsePayoutWebhook(payload) -> ConfirmPayout`

Điểm hay:

* sàn thay bank/PSP **không phải sửa lõi RampOS**, chỉ thay adapter.

---

## 7) Compliance Pack (KYC/AML/KYT + Case Management)

### 7.1 KYC tiering & limits

* Tier 0: xem giá/quote
* Tier 1: eKYC cơ bản, hạn mức thấp
* Tier 2: nâng hạn mức (bổ sung thông tin)
* Tier 3: KYB/doanh nghiệp

### 7.2 AML rules (risk-based)

Bám hướng dẫn VASP/virtual assets của FATF ([FATF][6]) và tương thích yêu cầu nội địa theo Luật PCRT 2022. ([THƯ VIỆN PHÁP LUẬT][7])

Rule mẫu:

* Velocity/structuring: nạp nhiều lần nhỏ trong thời gian ngắn
* Unusual payout: rút ngay sau nạp
* Name mismatch (nếu rails cho phép kiểm tra)
* Device/IP anomaly
* Blacklist/PEP/Sanctions screening
* Case workflow: `OPEN → REVIEW → HOLD/RELEASE → REPORT`

### 7.3 KYT (tuỳ chọn theo scope on-chain)

Nếu sàn có nạp/rút on-chain, KYT dùng để chấm điểm rủi ro ví nguồn/đích; các giao dịch rủi ro cao sẽ bị `MANUAL_REVIEW`.

---

## 8) Multi-chain & Account Abstraction (AA) — “xịn” nhưng dùng đúng việc

### 8.1 ERC-4337 (khuyến nghị cho EVM)

ERC-4337 dùng `UserOperation`, alt-mempool, `EntryPoint`, bundler, paymaster để enable smart accounts mà không đổi consensus layer. ([Ethereum Improvement Proposals][3])
Lợi ích:

* gasless onboarding (paymaster) ([ERC4337 Docs][10])
* batch calls (1-click flow) ([ERC4337 Docs][10])
* signature linh hoạt (passkeys/multisig) ([ERC4337 Docs][10])

Về tích hợp hạ tầng bundler/mempool, có thể tham chiếu các đề xuất JSON-RPC cho ERC-4337 như EIP-7769. ([Ethereum Improvement Proposals][11])

### 8.2 EIP-7702 (tương thích tương lai, onboarding “EOA → smart-like”)

EIP-7702 đưa ra transaction type cho phép EOA “set code” theo cơ chế uỷ quyền/delegation, hướng tới tương thích tương lai với AA. ([Ethereum Improvement Proposals][4])
Ý nghĩa sản phẩm:

* giảm friction khi người dùng đang dùng EOA truyền thống
* hỗ trợ dần smart-account feature mà không đổi địa chỉ (tuỳ roadmap)

### 8.3 Passkeys/WebAuthn cho đăng nhập & ký

Passkeys dựa trên FIDO/WebAuthn, hướng tới đăng nhập “phishing-resistant” (kháng lừa đảo) và UX tốt hơn. ([FIDO Alliance][12])
RampOS khuyến nghị:

* user auth/login: passkeys
* signing intent nội bộ: EIP-712 ([Ethereum Improvement Proposals][5])
* signing AA: theo smart account module

---

## 9) Security Model (Threat model & biện pháp)

### 9.1 Các rủi ro chính

* API abuse / BOLA / Broken auth (OWASP API Top 10) ([OWASP Foundation][13])
* Webhook spoofing / replay
* Ledger tampering
* Insider risk / key leakage
* Payout fraud
* AA paymaster abuse (spam, drain)

### 9.2 Biện pháp cốt lõi

**API & dịch vụ**

* mTLS nội bộ + JWT/OIDC cho admin
* rate limit + WAF
* strict RBAC/ABAC theo tenant
* idempotency & dedupe trên mọi ghi trạng thái

**Identity cho workload (zero-trust)**

* Dùng SPIFFE/SPIRE để cấp identity ngắn hạn (SVID) cho workload ([Spiffe][14])
* mTLS sidecar/proxy (Envoy Gateway hoặc Envoy) cho dịch vụ nội bộ ([Envoy Proxy][15])

**Secrets & Crypto**

* Vault Transit để “cryptography as a service”, ký/verify/HMAC/encrypt ([HashiCorp Developer][16])
* KMS/HSM (cloud) cho master keys; tách quyền operator

**Audit**

* Append-only audit log (WORM storage / immutability policy), hash chain theo batch.

---

## 10) Tech Stack “mới & xịn” (khuyến nghị triển khai production)

### 10.1 Ngôn ngữ & runtime

* **Core Orchestrator**: Rust (Tokio + Axum) *hoặc* Go (high-concurrency, đơn giản vận hành)
* **SDK/Adapters**: TypeScript (Node LTS) cho hệ sinh thái sàn; Go/Rust cho adapter hiệu năng cao
* **Smart contracts**: Solidity + Foundry; audit tooling

### 10.2 Kiến trúc chạy tác vụ dài & retry chuẩn

* Dùng **Temporal** cho durable workflows (pay-in/payout/recon/dispute là “job dài”, cần retry, cần bền vững) ([Temporal Docs][17])

### 10.3 Messaging / Event streaming

* **NATS JetStream** cho message persistence, replay, HA; phù hợp event-driven microservices ([NATS Docs][18])
* (Scale lớn) Kafka làm lựa chọn thay thế nếu sàn đã có Kafka ecosystem

### 10.4 Data layer

* **PostgreSQL** (ledger + intent + policy)
* **Redis** (cache, rate-limit, job queue lightweight)
* **ClickHouse** (analytics, fraud patterns, báo cáo nhanh)
* S3-compatible object store (raw payloads, recon batches, evidence)

### 10.5 Observability

* **OpenTelemetry** cho traces/metrics/logs + OTLP collector ([OpenTelemetry][19])
* Prometheus + Grafana + Loki/Tempo (hoặc vendor APM)

### 10.6 Hạ tầng triển khai

* Kubernetes + GitOps (ArgoCD)
* Service mesh/gateway: Envoy Gateway (policy mTLS/JWT dễ quản trị) ([Envoy Gateway][20])
* IaC: Terraform + Helm
* Secrets: Vault + external KMS

---

## 11) Performance & SLO (mục tiêu thực tế)

* API p95 < 150ms (read) / < 300ms (write) ở tải bình thường
* Webhook delivery: retry trong 24h, at-least-once
* Recon: chốt sổ theo batch (T+0/T+1 tuỳ rails)
* Uptime: 99.9% (core), 99.5% (analytics)

---

## 12) Roadmap triển khai (để ra tiền nhanh)

**Phase 1 (0–30 ngày):** pay-in/payout state machine + ledger + adapter mẫu + webhooks + audit logs
**Phase 2 (31–60 ngày):** Compliance Pack (KYC tiers, AML rules, case mgmt) + recon batch + export báo cáo
**Phase 3 (61–90 ngày):** Multi-tenant hardening + AA kit (ERC-4337) ([Ethereum Improvement Proposals][3]) + optional EIP-7702 readiness ([Ethereum Improvement Proposals][4])

---

## Phụ lục A — Checklist “đúng BYOR” khi pitch cho sàn

* ✅ Bạn không giữ tiền, không làm PSP, không làm bank
* ✅ Sàn giữ quan hệ rails, bạn cung cấp chuẩn adapter
* ✅ Bạn bán: state machine + ledger + compliance + audit + AA kit
* ✅ Có dashboard vận hành: recon, manual review, dispute, SLA

---

## Phụ lục B — Vì sao giải pháp này “đúng” cho crypto/VND

* Vì **giao dịch/thanh toán phải bằng VND** ([xaydungchinhsach.chinhphu.vn][2]) nên “phần VND rails + ledger + audit + compliance” là trọng tâm.
* Vì **đã mở thủ tục cấp phép từ 20/01/2026** ([baochinhphu.vn][1]) nên sàn mới cần một bộ “compliance-ready” để tăng cơ hội đi nhanh.

---

Nếu bạn muốn, mình sẽ viết tiếp **Whitepaper v1.1 dạng “tài liệu bàn giao kỹ thuật”** (siêu chi tiết) gồm:

1. **JSON Schema** đầy đủ cho toàn bộ API + webhook signing spec
2. **DB schema** chi tiết (DDL) cho ledger & recon
3. **Bộ AML rules mặc định** + cách cấu hình per-tenant
4. **AA kit spec**: bundler/paymaster, policy chống abuse, mô hình phí gasless

Bạn muốn “whitepaper kỹ thuật” tập trung nhiều hơn vào **(A) kiến trúc backend & vận hành**, hay **(B) AA/multi-chain & wallet UX**?

[1]: https://baochinhphu.vn/bat-dau-tiep-nhan-ho-so-cap-phep-thi-truong-tai-san-ma-hoa-tu-ngay-20-1-102260120185611897.htm?utm_source=chatgpt.com "Bắt đầu tiếp nhận hồ sơ cấp phép thị trường tài sản mã hóa từ ngày 20/1"
[2]: https://xaydungchinhsach.chinhphu.vn/toan-van-nghi-quyet-so-5-2025-nq-cp-ve-trien-khai-thi-diem-thi-truong-tai-san-ma-hoa-tai-viet-nam-119250909184045221.htm "TOÀN VĂN: Nghị quyết số 05/2025/NQ-CP về triển khai thí điểm thị trường tài sản mã hóa tại Việt Nam"
[3]: https://eips.ethereum.org/EIPS/eip-4337?utm_source=chatgpt.com "ERC-4337: Account Abstraction Using Alt Mempool"
[4]: https://eips.ethereum.org/EIPS/eip-7702?utm_source=chatgpt.com "EIP-7702: Set Code for EOAs"
[5]: https://eips.ethereum.org/EIPS/eip-712?utm_source=chatgpt.com "EIP-712: Typed structured data hashing and signing"
[6]: https://www.fatf-gafi.org/en/publications/Fatfrecommendations/Guidance-rba-virtual-assets-2021.html?utm_source=chatgpt.com "Updated Guidance for a Risk-Based Approach to Virtual ..."
[7]: https://thuvienphapluat.vn/van-ban/Tien-te-Ngan-hang/Luat-14-2022-QH15-Phong-chong-rua-tien-519327.aspx?utm_source=chatgpt.com "Luật Phòng, chống rửa tiền 2022"
[8]: https://owasp.org/www-project-api-security/?utm_source=chatgpt.com "OWASP API Security Project"
[9]: https://eips.ethereum.org/EIPS/eip-7964?utm_source=chatgpt.com "ERC-7964: Crosschain EIP-712 Signatures"
[10]: https://docs.erc4337.io/index.html "ERC-4337 Documentation"
[11]: https://eips.ethereum.org/EIPS/eip-7769?utm_source=chatgpt.com "ERC-7769: JSON-RPC API for ERC-4337"
[12]: https://fidoalliance.org/passkeys/?utm_source=chatgpt.com "FIDO Passkeys: Passwordless Authentication"
[13]: https://owasp.org/API-Security/editions/2023/en/0x11-t10/?utm_source=chatgpt.com "OWASP Top 10 API Security Risks – 2023"
[14]: https://spiffe.io/docs/latest/spiffe-about/overview/?utm_source=chatgpt.com "SPIFFE Overview"
[15]: https://www.envoyproxy.io/docs/envoy/latest/start/sandboxes/double-proxy?utm_source=chatgpt.com "Double proxy (with mTLS encryption)"
[16]: https://developer.hashicorp.com/vault/docs/secrets/transit?utm_source=chatgpt.com "Transit secrets engine | Vault"
[17]: https://docs.temporal.io/workflows?utm_source=chatgpt.com "Temporal Workflow | Temporal Platform Documentation"
[18]: https://docs.nats.io/nats-concepts/jetstream?utm_source=chatgpt.com "JetStream - NATS Docs"
[19]: https://opentelemetry.io/docs/specs/otel/?utm_source=chatgpt.com "OpenTelemetry Specification 1.53.0"
[20]: https://gateway.envoyproxy.io/?utm_source=chatgpt.com "Envoy Gateway - Envoy proxy"
