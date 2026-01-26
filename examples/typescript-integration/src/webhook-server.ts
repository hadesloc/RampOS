import express from 'express';
import bodyParser from 'body-parser';
import { RampOSWebhook } from '@rampos/sdk';
import dotenv from 'dotenv';

dotenv.config();

const app = express();
const port = process.env.PORT || 3001;
const WEBHOOK_SECRET = process.env.RAMPOS_WEBHOOK_SECRET || 'your-webhook-secret';

// Use raw body parser for signature verification
app.use(bodyParser.json({
    verify: (req: any, res, buf) => {
        req.rawBody = buf;
    }
}));

app.post('/webhook', (req, res) => {
    const signature = req.headers['x-rampos-signature'] as string;

    // Verify signature
    if (!RampOSWebhook.verifySignature(JSON.stringify(req.body), signature, WEBHOOK_SECRET)) {
        console.error('❌ Invalid signature');
        return res.status(401).send('Invalid signature');
    }

    const event = req.body;
    console.log(`\n🔔 Received Webhook: ${event.type}`);
    console.log('Payload:', JSON.stringify(event.payload, null, 2));

    // Handle events
    switch (event.type) {
        case 'INTENT.UPDATED':
            handleIntentUpdate(event.payload);
            break;
        case 'KYC.VERIFIED':
            console.log(`User ${event.payload.userId} verified!`);
            break;
        default:
            console.log('Unknown event type');
    }

    res.status(200).send('OK');
});

function handleIntentUpdate(payload: any) {
    console.log(`Intent ${payload.id} is now ${payload.status}`);
    if (payload.status === 'COMPLETED') {
        console.log('🎉 Transaction successful!');
    } else if (payload.status === 'FAILED') {
        console.error('Transaction failed:', payload.error);
    }
}

app.listen(port, () => {
    console.log(`Webhook server listening at http://localhost:${port}`);
});
