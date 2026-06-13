# BÁO CÁO NGHIÊN CỨU CHUYÊN SÂU — RAMPOS

**Ngày tổng hợp:** 31/05/2026
**Phạm vi:** 5 lĩnh vực nghiên cứu, 11 truy vấn chuyên sâu, 93 URLs, 60 claims, 15 xác minh
**Phương pháp:** Multi-modal search → Fetch & Extract → 3-vote Adversarial Verification → Completeness Critic → Synthesis
**Ngôn ngữ:** Tiếng Việt

---

## TÓM TẮT ĐIỀU HÀNH (Executive Summary)

RampOS định vị là một nền tảng hạ tầng B2B "compliance-first" cho các sàn giao dịch, ví và nền tảng fintech cần tích hợp on/off-ramp crypto-fiat, với thị trường ban đầu là Việt Nam.

**7 phát hiện chính:**

1. **Thị trường toàn cầu** tăng trưởng CAGR 10.7%–19.2%, đạt $11.7B–$14.8B vào 2033–2034 [TRUNG BÌNH]. Không có dữ liệu cụ thể cho corridor VND.

2. **Việt Nam đang mở cửa:** Luật Công nghệ Số có hiệu lực từ Q3/2026, Nghị quyết 05 sandbox [CAO]. OKX, HashKey, CAEX đã tham gia thí điểm. Bithumb + SSI Digital.

3. **ERC-4337** đạt 2.15M HandleOps [CAO] nhưng lỗ hổng Post-Op Fund Drain v0.8.0 là rủi ro thực sự [CAO]. Chưa có bằng chứng AA là differentiator cho B2B tại VN.

4. **Khung pháp lý VN** đang hình thành nhanh nhưng lộ trình cấp phép chính xác cho hạ tầng on/off-ramp chưa rõ [TRUNG BÌNH]. Lệnh cấm SBV là rủi ro nền tảng.

5. **Temporal.io** có adoption mạnh (16+ định chế tài chính) nhưng là dependency vận hành nặng.

6. **RFQ cho VND:** Với corridor VND mỏng (1-2 MM), RFQ có thể thêm phức tạp không lợi ích [RỦI RO CAO].

7. **Khoảng trống nghiêm trọng:** Chưa xác minh repo độc lập, chưa phỏng vấn khách hàng, chưa mô hình tài chính số liệu thực, chưa trích dẫn văn bản pháp lý gốc SBV.

---

## 1. THỊ TRƯỜNG ON/OFF-RAMP CRYPTO

### 1.1 Quy mô thị trường toàn cầu

| Nguồn | Quy mô hiện tại | Dự báo | CAGR | Độ tin cậy |
|-------|----------------|--------|------|-----------|
| Dataintelo (2025) | $8.4B (2025) | — | 17.0% | [TRUNG BÌNH] |
| Market Intelo | $2.4B (2024) | $11.7B (2033) | 19.2% | [TRUNG BÌNH] |
| Intel Market Research | $5.84B (2025) | $14.76B (2034) | 10.7% | [TRUNG BÌNH] |

**⚠️ Mâu thuẫn:** Chênh lệch $2.4B vs $8.4B — do khác biệt định nghĩa phạm vi và phương pháp luận. Tất cả là báo cáo thương mại, không peer-reviewed.

**Chỉ số giao dịch thực tế (2024):**
- Coinbase: ~$450B tổng khối lượng [TRUNG BÌNH]
- MoonPay: >$3B on-ramp [TRUNG BÌNH]
- Sàn lớn: ~$4.2T spot volume kết hợp [TRUNG BÌNH]

### 1.2 Đối thủ cạnh tranh chính

**Widget/API Provider (đối thủ trực tiếp):**

| Provider | Loại hình | Phạm vi | Hỗ trợ VND | Điểm yếu cho VN |
|----------|----------|--------|-----------|----------------|
| **MoonPay** | Widget/API | 160+ QG | ❌ | Không VND |
| **TransFi** | API B2B | Châu Á, LATAM | ✅ | Liquidity pool nhỏ |
| **Banxa** | B2B Platform | Toàn cầu | ❌ | Tập trung phương Tây |
| **Onramper** | Aggregator | 150+ QG | ❌ | Phụ thuộc bên thứ 3 |
| **Ramp Network** | Widget/API | 150+ QG | ❌ | Không VND |
| **Paybis** | Direct + API | 180+ QG | ❌ | Không VND corridor |
| **Wyre** 💀 | API B2B | Mỹ | ❌ | Đã ngừng hoạt động 2023 |

**Đối thủ tại Việt Nam:**

| Đơn vị | Mô hình | Ghi chú |
|--------|--------|--------|
| **ONUS** | Sàn + ví + ramp VND | Đối thủ B2C chính, có quan hệ ngân hàng |
| **Remitano** | P2P OTC | Volume VND đáng kể |
| **SSI Digital + Bithumb** | Sắp ra mắt | Sandbox Q3/2026 |
| **CAEX** | Sàn thí điểm | Sandbox chính phủ |

### 1.3 Thị trường Việt Nam — Điểm ngoặt Q3/2026

**Chính sách:**
- ✅ Luật Công nghệ Số — hiệu lực Q3/2026 [CAO — WFW 27/01/2026]
- ✅ Nghị quyết 05 — sandbox crypto [CAO]
- ✅ OKX, HashKey, CAEX được cấp phép thí điểm [CAO]
- ✅ Bithumb + SSI Digital tham gia [CAO]
- ⚠️ Đề xuất thuế crypto đang thảo luận [TRUNG BÌNH]

**Thách thức cốt lõi:**
1. **Lệnh cấm SBV:** Cấm crypto như phương tiện thanh toán — chưa rõ điều chỉnh [RỦI RO CAO]
2. **Corridor VND mỏng:** Không dữ liệu công khai volume VND/USDT [KHOẢNG TRỐNG DỮ LIỆU]
3. **Cạnh tranh local:** ONUS, Remitano có sẵn quan hệ ngân hàng + users

**Cơ hội:**
- Hầu hết đối thủ quốc tế chưa hỗ trợ VND
- Sàn/ví mới vào sandbox cần hạ tầng tuân thủ nhanh
- "Compliance-first" là differentiator nếu giải quyết được AML/CTF nội địa

---

## 2. CÔNG NGHỆ ERC-4337 ACCOUNT ABSTRACTION

### 2.1 Tổng quan

| Chỉ số | Dữ liệu | Nguồn | Độ tin cậy |
|--------|---------|-------|-----------|
| HandleOps EntryPoint 0.7.0 | 2.15M | Etherscan | [CAO] |
| Phiên bản mới nhất | v0.9.0 (11/2025) | eth-infinitism GitHub | [CAO] |
| Brand customers Alchemy | 11 thương hiệu | Alchemy | [CAO] |
| EntryPoint canonical | `0x0000000071727de22e5e9d8baf0edac6f37da032` | Etherscan | [CAO] |

**Câu hỏi chiến lược:** ERC-4337 là dependency cứng, module tùy chọn, hay marketing? B2B buyer quan tâm operator tooling, audit trails, compliance — không phải smart contract wallet UX.

**Adoption tại VN/SEA:** Chưa có dữ liệu. Phần lớn dùng CEX + EOA — AA có thể là "solution looking for a problem".

### 2.2 Rủi ro bảo mật

**Lỗ hổng Post-Op Fund Drain (v0.8.0):**
- Cơ chế: Khai thác gas metering post-operation để rút quỹ Paymaster
- Chi phí tấn công: ~0.003 ETH — cực thấp
- Nguồn: Tech-Champion.com [TRUNG BÌNH]
- Impact RampOS: Nếu dùng Paymaster tài trợ gas → vector cần mitigrate

**Best practices:**
1. Luôn dùng EntryPoint mới nhất đã audit
2. Giới hạn gas trong Paymaster validation
3. Không lưu state trong validation
4. Audit độc lập (OpenZeppelin, Trail of Bits)
5. Monitoring on-chain bất thường

---

## 3. COMPLIANCE VÀ KHUNG PHÁP LÝ

### 3.1 FATF Travel Rule

- FATF Recommendation 16 áp dụng cho Virtual Assets
- Yêu cầu VASP thu thập/chia sẻ thông tin sender/receiver
- Giao thức: TRP, TRISA, OpenVASP [TRUNG BÌNH]
- Hàm ý: RampOS cần tích hợp Travel Rule ngay từ đầu

### 3.2 Quy định tại Việt Nam

| Văn bản | Nội dung | Hiệu lực | Độ tin cậy |
|---------|---------|---------|-----------|
| Luật Công nghệ Số | Quy định tài sản số | Q3/2026 | [CAO] |
| Nghị quyết 05 | Sandbox crypto | Đang triển khai | [CAO] |
| Luật AML/CFT 14/2022/QH15 | Phòng chống rửa tiền | Đã hiệu lực | [CAO] |
| Quy định thuế crypto | Đang thảo luận | Chưa rõ | [TRUNG BÌNH] |
| SBV Circulars | Cấm crypto thanh toán | Hiện hành | [KHOẢNG TRỐNG] |

**Rủi ro pháp lý:**
1. **SBV duy trì lệnh cấm** → Toàn bộ mô hình không khả thi [RỦI RO CAO]
2. **Yêu cầu cấp phép** chưa rõ (VASP? Trung gian thanh toán?)
3. **Data sovereignty** — Nghị định 53/2022/ND-CP data localization
4. **Tax obligations** chưa xác định

---

## 4. KIẾN TRÚC WORKFLOW ENGINE

### 4.1 Temporal.io trong Fintech

- 16+ định chế tài chính dùng Temporal [TRUNG BÌNH]
- Case study: **ANZ Bank** — home loan origination [CAO]
- Use cases: payments, fraud detection, loan origination, identity verification

**Phù hợp cho on/off-ramp:**
1. Workflow durability cho giao dịch kéo dài 3-5 ngày
2. Saga pattern — compensating transactions
3. Audit trail đầy đủ
4. Built-in retry với exponential backoff

**Thách thức vận hành:**
- Độ phức tạp: DB + Elasticsearch + worker management
- Failure recovery: signal, reset, cancel workflows
- Chi phí: Temporal Cloud pricing phức tạp

### 4.2 Rust SDK

- `temporalio-sdk`, `temporalio-client`, `temporalio-sdk-core` đã release [CAO]
- Ưu: Cùng core SDK với Go/TS/Python/Java
- Nhược: Cộng đồng nhỏ hơn, ít case study production
- Rủi ro: Không phải SDK ưu tiên số 1 của Temporal

---

## 5. RFQ AUCTION & LIQUIDITY MODEL

### 5.1 Mô hình RFQ trong Crypto

| Nền tảng | Mô hình | Đối tượng |
|----------|--------|---------|
| **Paradigm** | RFQ block trading | Institution |
| **Liquid Mercury** | OTC desk software | Institution (white-label) |
| **FinchTrade** | RFQ OTC | Institution, HNWI |
| **DWF Labs** | LP + Research | Institution |

**Vấn đề cho VND corridor:**
- Số lượng LP hạn chế (1-2 MM)
- Spread rộng nếu ít cạnh tranh
- Rủi ro thanh khoản khi biến động
- **Kết luận:** Nếu không có ≥3-4 LP cạnh tranh, RFQ thêm phức tạp không lợi ích [RỦI RO CAO]

### 5.2 Hạ tầng thanh toán xuyên biên giới

- **BIS Project Nexus:** Kết nối IPS xuyên biên giới [CAO]
- **ASEAN:** PayNow-PromptPay, QR code xuyên biên giới
- **PSP Việt Nam:** NAPAS, VNPay, Momo, ZaloPay, ngân hàng nội địa

---

## 6. ĐÁNH GIÁ CHIẾN LƯỢC

### 6.1 SWOT Analysis

**ĐIỂM MẠNH:**
- S1: Thiết kế nghiệp vụ chi tiết, nhất quán nội bộ [CAO]
- S2: Multi-tenant architecture từ đầu [TRUNG BÌNH]
- S3: Compliance-first positioning — đúng thời điểm [TRUNG BÌNH]
- S4: Rust stack — hiệu năng + an toàn [CAO]
- S5: Sandbox/demo engine — vũ khí bán hàng [TRUNG BÌNH]

**ĐIỂM YẾU:**
- W1: ❌ Chưa xác minh repo độc lập — **NGHIÊM TRỌNG**
- W2: ❌ Không primary research — ICP hypothetical — **NGHIÊM TRỌNG**
- W3: ❌ Corridor VND chưa chứng minh — **NGHIÊM TRỌNG**
- W4: ❌ Phụ thuộc pháp lý chưa giải quyết — **NGHIÊM TRỌNG**
- W5: ❌ Không mô hình tài chính — **CAO**
- W6: ❌ Không security audit thực tế — **CAO**

**CƠ HỘI:**
- O1: Sandbox Q3/2026 — thời điểm hoàn hảo [CAO]
- O2: Đối thủ quốc tế chưa hỗ trợ VND [TRUNG BÌNH]
- O3: Hạ tầng thanh toán ASEAN đang hình thành [CAO]
- O4: Bithumb-SSI, OKX, HashKey → nhu cầu hạ tầng tuân thủ [TRUNG BÌNH]

**ĐE DỌA:**
- T1: 💀 SBV duy trì lệnh cấm — **HIỆN HỮU**
- T2: ⚠️ Cạnh tranh sàn quốc tế nguồn lực lớn — **CAO**
- T3: ⚠️ Local players lợi thế quan hệ — **CAO**
- T4: Wyre collapse precedent — **TRUNG BÌNH**

### 6.2 Khuyến nghị chiến lược

**🔴 KHẨN CẤP — Cần ngay:**
1. **Xác minh tính khả thi pháp lý:** Tra cứu văn bản gốc SBV, xác định lộ trình cấp phép, tham vấn luật sư fintech
2. **Xác minh repo:** Independent code review, test coverage, chạy docker-compose độc lập
3. **Primary market research:** Phỏng vấn 5-10 khách hàng tiềm năng, xác định WTP thực tế

**🟡 CAO — Trước pilot:**
4. **Mô hình tài chính:** Burn rate, unit economics, pricing sheet 3 tiers
5. **RFQ liquidity validation:** Tiếp cận LP, đánh giá spread thực tế
6. **Security audit:** SAST, dependency audit, pentest

**🟢 TRUNG BÌNH — Sau pilot:**
7. **ERC-4337 decision:** Core dependency hay optional module?
8. **Temporal runbooks:** Training, incident response
9. **Mở rộng corridor:** THB, SGD, MYR, IDR

---

## 7. NGUỒN THAM KHẢO

### Thị trường
1. Dataintelo. "Crypto On-Ramp Market Report 2034." [TRUNG BÌNH]
2. Market Intelo. "Crypto On-Ramp and Off-Ramp Solutions Market." [TRUNG BÌNH]
3. Intel Market Research. "Crypto Off-Ramp Service Market." [TRUNG BÌNH]
4. Paybis Blog. "Top 6 Best Crypto Onramp Solutions." Aug 2025. [TRUNG BÌNH]
5. Mastercard. "Crypto on-ramping and off-ramping." 2025. [CAO]

### Việt Nam
6. Watson Farley & Williams. "Landmark Legislation Regulates Digital Assets in Vietnam." 27/01/2026. [CAO]
7. CoinTelegraph. "Vietnam Tag Archive." [CAO]

### ERC-4337
8. Alchemy. "Account Abstraction." [CAO]
9. eth-infinitism. "Account Abstraction (GitHub)." [CAO]
10. Etherscan. "EntryPoint 0.7.0." [CAO]
11. Tech Champion. "ERC-4337 Post-Op Fund Drain." [TRUNG BÌNH]

### Compliance
12. FATF. "Virtual Assets." [CAO]
13. FATF Travel Rule — Wikipedia. [CAO]

### Temporal
14. Temporal. "Financial Services Solution." [CAO]
15. Temporal. "Rust & Core SDKs (GitHub)." [CAO]

### RFQ & Thanh toán
16. DWF Labs. "How RFQ Enables Crypto OTC Trading." [CAO]
17. FinchTrade. "Understanding RFQ Trading." [CAO]
18. Liquid Mercury. "Crypto OTC Desk Software Guide." Mar 2026. [CAO]
19. BIS. "Project Nexus." [CAO]

---

## PHỤ LỤC: KHOẢNG TRỐNG NGHIÊN CỨU

### Nghiêm trọng (phải đóng trước quyết định)

| # | Khoảng trống | Impact |
|---|-------------|--------|
| G1 | Tính khả thi pháp lý tại VN | Tồn tại dự án |
| G2 | Xác minh repo code độc lập | Mọi GTM promise |
| G3 | Primary customer research | ICP + pricing |
| G4 | Mô hình tài chính số liệu | Fundraising, runway |
| G5 | LP quan tâm cho VND corridor | RFQ model |

### Quan trọng (nên đóng trước pilot)

| # | Khoảng trống |
|---|-------------|
| G6 | Security audit thực tế |
| G7 | Temporal operational runbooks |
| G8 | ERC-4337 role clarity |
| G9 | Competitor deep-dive có tên + giá |
| G10 | PSP/bank integration costs tại VN |

---

*Báo cáo tổng hợp từ 93 URLs, 20 sources fetched, 60 claims extracted, 15 verified (3-vote adversarial). Tất cả claims chính đều có confidence level. Mâu thuẫn và khoảng trống được đánh dấu rõ ràng.*
