# Release Notes: RampOS v1.0.0

**Date:** 2026-01-25
**Version:** v1.0.0 (Initial Release)

## 🚀 Giới thiệu

RampOS v1.0.0 là giải pháp hạ tầng "Bring Your Own Rails" (BYOR) toàn diện dành cho các sàn giao dịch crypto/VND tại Việt Nam. Phiên bản này cung cấp một nền tảng vận hành mạnh mẽ, tích hợp sẵn các công cụ tuân thủ (Compliance), quản lý tài khoản thông minh (Account Abstraction), và hệ thống đối soát tự động.

RampOS giúp các doanh nghiệp tập trung vào phát triển sản phẩm và trải nghiệm người dùng, trong khi chúng tôi xử lý các vấn đề phức tạp về vận hành, bảo mật và tuân thủ pháp lý.

## ✨ Tính năng chính (Key Features)

### 1. Core Orchestrator
*   **State Machine chuẩn hóa**: Quản lý vòng đời giao dịch (pay-in, pay-out, trade) minh bạch và nhất quán.
*   **Double-Entry Ledger**: Hệ thống sổ cái kế toán kép đảm bảo tính toàn vẹn dữ liệu tài chính.
*   **Tự động đối soát (Reconciliation)**: Đối soát tự động giữa sổ cái nội bộ và các đối tác thanh toán (Bank/PSP).
*   **Webhook System**: Thông báo thời gian thực về trạng thái giao dịch, rủi ro và tuân thủ với bảo mật HMAC-SHA256.

### 2. Compliance Pack 🛡️
*   **KYC Tiering**: Phân cấp xác minh danh tính người dùng linh hoạt.
*   **AML Rules Engine**: Tự động phát hiện và cảnh báo các giao dịch đáng ngờ theo luật định.
*   **KYT Hooks**: Tích hợp giám sát giao dịch on-chain (Know Your Transaction).
*   **Case Management**: Công cụ quản lý hồ sơ rủi ro và báo cáo tuân thủ.

### 3. Account Abstraction (AA) Kit 🔐
*   **ERC-4337 Support**: Hỗ trợ ví thông minh (Smart Accounts) chuẩn Ethereum.
*   **Gasless Transactions**: Cơ chế Paymaster giúp người dùng giao dịch không cần ETH làm phí gas.
*   **Session Keys**: Cấp quyền giao dịch tạm thời, tăng cường bảo mật và UX.
*   **Batch Execution**: Thực thi nhiều giao dịch trong một lệnh duy nhất.

### 4. SDK & Integration 🛠️
*   **Rails Adapter SDK**: Dễ dàng tích hợp với các ngân hàng và cổng thanh toán tại Việt Nam.
*   **RESTful API**: API chuẩn hóa, dễ sử dụng và tích hợp.
*   **SDK (Go/Rust/JS)**: Hỗ trợ đa ngôn ngữ cho việc phát triển client.

## 📦 Danh sách Module

| Module | Trạng thái | Mô tả |
| :--- | :--- | :--- |
| **API** | ✅ Ready | HTTP API server (Axum) quản lý Intent, User, Webhooks. |
| **Compliance** | ✅ Ready | Engine xử lý KYC, AML, KYT và Case Management. |
| **AA Kit** | ✅ Ready | Bộ công cụ Account Abstraction (Contracts + Services). |
| **SDK** | ✅ Ready | Client SDKs và Adapter SDK cho việc tích hợp. |
| **Ledger** | ✅ Ready | Core ledger service với double-entry bookkeeping. |

## 🚀 Quick Start

### Yêu cầu hệ thống
*   Docker & Docker Compose
*   Git

### Cài đặt nhanh

1.  **Clone repository:**
    ```bash
    git clone https://github.com/your-org/rampos.git
    cd rampos
    ```

2.  **Khởi chạy hạ tầng (Docker):**
    ```bash
    docker-compose up -d
    ```
    *Lệnh này sẽ khởi động toàn bộ stack gồm: API Server, Database (Postgres), Redis, NATS, và các services phụ trợ.*

3.  **Kiểm tra trạng thái:**
    Truy cập `http://localhost:8080/health` để kiểm tra health check.

4.  **Tài liệu chi tiết:**
    *   [API Documentation](docs/API.md)
    *   [Deployment Guide](docs/DEPLOY.md)

---
*Cảm ơn bạn đã sử dụng RampOS! Mọi đóng góp và phản hồi xin gửi về [Issues](https://github.com/your-org/rampos/issues).*
