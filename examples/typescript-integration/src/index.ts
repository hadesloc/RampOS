import { RampOSClient } from '@rampos/sdk';
import dotenv from 'dotenv';

dotenv.config();

const API_URL = process.env.RAMPOS_API_URL || 'http://localhost:3000';
const TENANT_ID = process.env.RAMPOS_TENANT_ID || 'your-tenant-id';
const API_KEY = process.env.RAMPOS_API_KEY || 'your-api-key';
const API_SECRET = process.env.RAMPOS_API_SECRET || 'your-api-secret';

async function main() {
  console.log('🚀 Starting RampOS Integration Example');

  // 1. Initialize Client
  const client = new RampOSClient({
    baseUrl: API_URL,
    tenantId: TENANT_ID,
    apiKey: API_KEY,
    apiSecret: API_SECRET,
  });

  try {
    // 2. Create Payin Intent
    console.log('\nCreating Payin Intent...');
    const payin = await client.intents.createPayin({
      userId: 'user_123',
      amount: 1000000, // 1,000,000 VND
      currency: 'VND',
      paymentMethod: 'BANK_TRANSFER',
      metadata: {
        orderId: 'ORD-001'
      }
    });
    console.log('✅ Payin Intent Created:', payin.id);
    console.log('Make payment to:', payin.paymentDetails);

    // 3. Poll for status (simulated)
    console.log('\nPolling for status...');
    let currentStatus = payin.status;
    let attempts = 0;
    while (currentStatus !== 'COMPLETED' && attempts < 3) {
      const updatedIntent = await client.intents.get(payin.id);
      currentStatus = updatedIntent.status;
      console.log(`Status: ${currentStatus}`);
      if (currentStatus === 'COMPLETED') break;

      attempts++;
      await new Promise(r => setTimeout(r, 1000));
    }

    // 4. Check User Balance
    console.log('\nChecking User Balance...');
    const balance = await client.ledger.getBalance('user_123', 'VND');
    console.log(`User Balance: ${balance.available} VND`);

    // 5. Create Payout Intent
    if (parseFloat(balance.available) >= 500000) {
        console.log('\nCreating Payout Intent...');
        const payout = await client.intents.createPayout({
            userId: 'user_123',
            amount: 500000,
            currency: 'VND',
            bankAccount: {
                bankCode: 'VCB',
                accountNumber: '1234567890',
                accountName: 'NGUYEN VAN A'
            }
        });
        console.log('✅ Payout Intent Created:', payout.id);
    } else {
        console.log('\n⚠️ Insufficient balance for payout example');
    }

  } catch (error: any) {
    console.error('❌ Error:', error.message);
    if (error.response) {
        console.error('Data:', error.response.data);
    }
  }
}

main();
