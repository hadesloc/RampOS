import { describe, it, expect } from 'vitest'
import { cn } from '../utils'

describe('cn utility', () => {
  it('merges class names', () => {
    const result = cn('foo', 'bar')
    expect(result).toBe('foo bar')
  })

  it('handles conditional classes', () => {
    const isActive = true
    const result = cn('base', isActive && 'active')
    expect(result).toBe('base active')
  })

  it('handles false conditional classes', () => {
    const isActive = false
    const result = cn('base', isActive && 'active')
    expect(result).toBe('base')
  })

  it('handles undefined and null', () => {
    const result = cn('base', undefined, null, 'other')
    expect(result).toBe('base other')
  })

  it('handles array of classes', () => {
    const result = cn(['foo', 'bar'])
    expect(result).toBe('foo bar')
  })

  it('handles object with boolean values', () => {
    const result = cn({
      base: true,
      active: true,
      disabled: false,
    })
    expect(result).toBe('base active')
  })

  it('merges tailwind classes correctly', () => {
    // tailwind-merge should handle conflicting classes
    const result = cn('p-4', 'p-6')
    expect(result).toBe('p-6')
  })

  it('merges tailwind responsive classes correctly', () => {
    const result = cn('text-sm', 'md:text-lg', 'text-base')
    expect(result).toBe('md:text-lg text-base')
  })

  it('handles mixed inputs', () => {
    const result = cn(
      'base',
      true && 'conditional',
      ['array1', 'array2'],
      { object: true, disabled: false }
    )
    expect(result).toBe('base conditional array1 array2 object')
  })

  it('handles empty inputs', () => {
    const result = cn()
    expect(result).toBe('')
  })

  it('handles whitespace in class names', () => {
    const result = cn('  foo  ', '  bar  ')
    expect(result).toBe('foo bar')
  })
})
