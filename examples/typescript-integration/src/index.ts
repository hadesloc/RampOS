import { RampOSClient } from "@rampos/sdk";
import dotenv from "dotenv";

dotenv.config();

const API_URL = process.env.RAMPOS_API_URL || "http://localhost:3000";
const TENANT_ID = process.env.RAMPOS_TENANT_ID || "your-tenant-id";
const API_KEY = process.env.RAMPOS_API_KEY || "your-api-key";
const API_SECRET = process.env.RAMPOS_API_SECRET || "your-api-secret";

async function main() {
  console.log("Starting RampOS Integration Example");

  const client = new RampOSClient({
    baseUrl: API_URL,
    tenantId: TENANT_ID,
    apiKey: API_KEY,
    apiSecret: API_SECRET,
  });

  try {
    console.log("\nCreating Payin Intent...");
    const payin = await client.intents.createPayin({
      userId: "user_123",
      amount: 1000000,
      currency: "VND",
      paymentMethod: "BANK_TRANSFER",
      metadata: {
        orderId: "ORD-001",
      },
    });
    console.log("Payin Intent Created:", payin.id);
    console.log("Make payment to:", payin.paymentDetails);

    console.log("\nPolling for status...");
    let currentStatus = payin.status;
    let attempts = 0;
    while (currentStatus !== "COMPLETED" && attempts < 3) {
      const updatedIntent = await client.intents.get(payin.id);
      currentStatus = updatedIntent.status;
      console.log(`Status: ${currentStatus}`);
      if (currentStatus === "COMPLETED") break;

      attempts += 1;
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    console.log("\nChecking User Balance...");
    const balance = await client.ledger.getBalance("user_123", "VND");
    console.log(`User Balance: ${balance.available} VND`);

    if (parseFloat(balance.available) >= 500000) {
      console.log("\nCreating Payout Intent...");
      const payout = await client.intents.createPayout({
        userId: "user_123",
        amount: 500000,
        currency: "VND",
        bankAccount: {
          bankCode: "VCB",
          accountNumber: "1234567890",
          accountName: "NGUYEN VAN A",
        },
      });
      console.log("Payout Intent Created:", payout.id);
    } else {
      console.log("\nInsufficient balance for payout example");
    }

    console.log(
      "\nAdmin contract note: reconciliation workbench and evidence export are available through the thin rampos-cli preview or the admin endpoints documented in docs/SDK.md.",
    );
  } catch (error: any) {
    console.error("Error:", error.message);
    if (error.response) {
      console.error("Data:", error.response.data);
    }
  }
}

main();
