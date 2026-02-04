import { describe, it, expect, afterEach, vi } from 'vitest'
import {
  constantTimeEqual,
  createAdminSessionToken,
  isAdminSessionTokenValid,
} from '../admin-auth'

afterEach(() => {
  vi.useRealTimers()
})

describe('constantTimeEqual', () => {
  it('returns true for identical strings', () => {
    expect(constantTimeEqual('alpha', 'alpha')).toBe(true)
  })

  it('returns false for different lengths', () => {
    expect(constantTimeEqual('alpha', 'alph')).toBe(false)
  })

  it('returns false for different content', () => {
    expect(constantTimeEqual('alpha', 'alphb')).toBe(false)
  })
})

describe('admin session tokens', () => {
  it('creates a token that validates with the same secret', () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-02-04T12:00:00Z'))

    const secret = 'super-secret'
    const token = createAdminSessionToken(secret, 60)

    expect(token.split('.')).toHaveLength(3)
    expect(isAdminSessionTokenValid(token, secret)).toBe(true)
  })

  it('rejects tokens signed with another secret', () => {
    const token = createAdminSessionToken('secret-a', 60)
    expect(isAdminSessionTokenValid(token, 'secret-b')).toBe(false)
  })

  it('rejects expired tokens', () => {
    vi.useFakeTimers()
    const baseTime = new Date('2026-02-04T12:00:00Z')
    vi.setSystemTime(baseTime)

    const token = createAdminSessionToken('secret', 1)

    vi.setSystemTime(new Date(baseTime.getTime() + 2000))

    expect(isAdminSessionTokenValid(token, 'secret')).toBe(false)
  })
})
