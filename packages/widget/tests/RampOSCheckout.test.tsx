import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import React from 'react';
import RampOSCheckout from '../src/components/RampOSCheckout';
import { RampOSEventEmitter } from '../src/utils/events';

describe('RampOSCheckout', () => {
  beforeEach(() => {
    RampOSEventEmitter.resetInstance();
  });

  it('renders with default state showing asset selection', () => {
    render(<RampOSCheckout apiKey="test-key" />);
    expect(screen.getByText('RampOS Checkout')).toBeInTheDocument();
    expect(screen.getByText('Select an asset to purchase')).toBeInTheDocument();
    expect(screen.getByText('USDC')).toBeInTheDocument();
    expect(screen.getByText('USDT')).toBeInTheDocument();
  });

  it('renders with pre-selected asset skips to amount step', () => {
    render(<RampOSCheckout apiKey="test-key" asset="USDC" />);
    expect(screen.getByText('Amount (USDC)')).toBeInTheDocument();
  });

  it('renders with both asset and amount skips to payment method', () => {
    render(<RampOSCheckout apiKey="test-key" asset="USDC" amount={100} />);
    expect(screen.getByText('Select payment method')).toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', () => {
    const onClose = vi.fn();
    render(<RampOSCheckout apiKey="test-key" onClose={onClose} />);

    const closeButton = screen.getByLabelText('Close');
    fireEvent.click(closeButton);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calls onReady on mount', () => {
    const onReady = vi.fn();
    render(<RampOSCheckout apiKey="test-key" onReady={onReady} />);
    expect(onReady).toHaveBeenCalledOnce();
  });

  it('navigates through asset selection flow', () => {
    render(<RampOSCheckout apiKey="test-key" />);

    // Click on USDC
    fireEvent.click(screen.getByText('USDC'));

    // Should now show amount input
    expect(screen.getByText('Amount (USDC)')).toBeInTheDocument();
  });

  it('shows Powered by RampOS footer', () => {
    render(<RampOSCheckout apiKey="test-key" />);
    expect(screen.getByText('Powered by RampOS')).toBeInTheDocument();
  });

  it('applies custom theme colors', () => {
    const { container } = render(
      <RampOSCheckout
        apiKey="test-key"
        theme={{ primaryColor: '#ff0000', backgroundColor: '#f0f0f0' }}
      />
    );
    const widget = container.querySelector('[data-testid="rampos-checkout"]') as HTMLElement;
    expect(widget).toBeInTheDocument();
    expect(widget.style.backgroundColor).toBe('rgb(240, 240, 240)');
  });

  it('validates amount before proceeding', () => {
    render(<RampOSCheckout apiKey="test-key" asset="USDC" />);

    // Try to continue without entering amount
    fireEvent.click(screen.getByText('Continue'));

    // Should show error
    expect(screen.getByText('Please enter a valid amount')).toBeInTheDocument();
  });

  it('processes checkout flow to success', async () => {
    const onSuccess = vi.fn();
    render(
      <RampOSCheckout
        apiKey="test-key"
        asset="USDC"
        amount={50}
        onSuccess={onSuccess}
      />
    );

    // Select payment method
    fireEvent.click(screen.getByText('Bank Transfer'));

    // Should show summary
    expect(screen.getByText('Confirm Payment')).toBeInTheDocument();

    // Confirm
    fireEvent.click(screen.getByText('Confirm Payment'));

    // Should show processing
    expect(screen.getByText('Processing your transaction...')).toBeInTheDocument();

    // Wait for success
    await waitFor(() => {
      expect(screen.getByText('Payment Successful!')).toBeInTheDocument();
    }, { timeout: 5000 });

    expect(onSuccess).toHaveBeenCalledOnce();
    expect(onSuccess).toHaveBeenCalledWith(
      expect.objectContaining({
        status: 'success',
        amount: 50,
        asset: 'USDC',
      })
    );
  });
});
