import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import { WalletCard } from '../wallet-card'
import { AssetRow } from '../asset-row'
import { QuickActions } from '../quick-actions'
import { BalanceDisplay } from '../balance-display'
import { TransactionRow } from '../transaction-row'
import { ArrowUpRight } from 'lucide-react'

// Mock clipboard
Object.assign(navigator, {
  clipboard: {
    writeText: vi.fn(),
  },
});

describe('Portal Components', () => {
  describe('WalletCard', () => {
    it('renders address and status correctly', () => {
      render(<WalletCard address="0x1234567890abcdef" deployed={true} />)
      expect(screen.getByText(/0x1234...cdef/)).toBeInTheDocument()
      expect(screen.getByText('Deployed')).toBeInTheDocument()
    })

    it('handles copy action', () => {
      const onCopy = vi.fn()
      render(<WalletCard address="0x123" deployed={false} onCopy={onCopy} />)
      const copyBtn = screen.getByRole('button', { name: /copy address/i })
      fireEvent.click(copyBtn)
      expect(onCopy).toHaveBeenCalled()
    })

    it('shows loading state', () => {
        const { container } = render(<WalletCard address="" deployed={false} loading={true} />)
        expect(container.querySelector('.animate-pulse')).toBeInTheDocument()
    })
  })

  describe('AssetRow', () => {
    it('renders asset info correctly', () => {
      render(<AssetRow name="Bitcoin" symbol="BTC" balance="1.5" value="$50,000" />)
      expect(screen.getByText('Bitcoin')).toBeInTheDocument()
      expect(screen.getByText('BTC')).toBeInTheDocument()
      expect(screen.getByText('1.5')).toBeInTheDocument()
      expect(screen.getByText('$50,000')).toBeInTheDocument()
    })

    it('is clickable when onClick is provided', () => {
        const onClick = vi.fn()
        render(<AssetRow name="Ethereum" symbol="ETH" balance="10" onClick={onClick} />)
        fireEvent.click(screen.getByText('Ethereum'))
        expect(onClick).toHaveBeenCalled()
    })
  })

  describe('QuickActions', () => {
    it('renders actions list', () => {
      const actions = [
        { label: 'Deposit', icon: <ArrowUpRight />, href: '/deposit' },
        { label: 'Withdraw', icon: <ArrowUpRight />, href: '/withdraw' }
      ]
      render(<QuickActions actions={actions} />)
      expect(screen.getByText('Deposit')).toBeInTheDocument()
      expect(screen.getByText('Withdraw')).toBeInTheDocument()
    })
  })

  describe('BalanceDisplay', () => {
      it('renders balances correctly', () => {
          const balances = [
              { currency: 'VND', total: '1000000', available: '800000', locked: '200000' }
          ]
          render(<BalanceDisplay balances={balances} />)
          // Note: exact formatting depends on locale, so we check for presence
          expect(screen.getByText('Total Balance')).toBeInTheDocument()
      })
  })

  describe('TransactionRow', () => {
      it('renders transaction details', () => {
          render(
            <TransactionRow
                id="1"
                type="PAYIN_VND"
                amount="500000"
                currency="VND"
                status="completed"
                createdAt={new Date().toISOString()}
            />
          )
          expect(screen.getByText('PAYIN VND')).toBeInTheDocument()
          expect(screen.getByText('completed')).toBeInTheDocument()
      })
  })
})
