import { createHmac, timingSafeEqual } from 'crypto';

export class WebhookVerifier {
  /**
   * Verifies the signature of a webhook payload.
   *
   * @param payload - The raw request body as a string.
   * @param signature - The signature header sent by RampOS (e.g., X-RampOS-Signature).
   * @param secret - The webhook signing secret provided by RampOS.
   * @returns True if the signature is valid, false otherwise.
   * @throws Error if any parameter is missing.
   */
  public verify(payload: string, signature: string, secret: string): boolean {
    if (!payload) throw new Error('Payload is required');
    if (!signature) throw new Error('Signature is required');
    if (!secret) throw new Error('Secret is required');

    const hmac = createHmac('sha256', secret);
    const digest = hmac.update(payload).digest('hex');
    const expectedSignature = `sha256=${digest}`;

    const signatureBuffer = Buffer.from(signature);
    const expectedBuffer = Buffer.from(expectedSignature);

    if (signatureBuffer.length !== expectedBuffer.length) {
      return false;
    }

    return timingSafeEqual(signatureBuffer, expectedBuffer);
  }
}
