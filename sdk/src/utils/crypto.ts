import { createHmac } from 'crypto';

/**
 * Signs a request using HMAC-SHA256.
 * Matches the Go SDK implementation: method\npath\ntimestamp\nbody
 */
export function signRequest(
  _apiKey: string,
  apiSecret: string,
  method: string,
  path: string,
  body: string,
  timestamp: number
): string {
  // Go SDK implementation: fmt.Sprintf("%s\n%s\n%d\n%s", method, path, timestamp, string(body))
  const message = `${method}\n${path}\n${timestamp}\n${body}`;
  return createHmac('sha256', apiSecret)
    .update(message)
    .digest('hex');
}
