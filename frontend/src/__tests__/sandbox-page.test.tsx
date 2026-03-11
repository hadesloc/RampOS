import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import SandboxPage from '@/app/[locale]/(admin)/sandbox/page';

const mockFetch = vi.fn();

describe('SandboxPage', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal('fetch', mockFetch);
  });

  it('loads presets and seeds a sandbox tenant through the bounded contract', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            code: 'BASELINE',
            name: 'Baseline Sandbox',
            seedPackageVersion: '2026-03-08',
            defaultScenarios: ['PAYIN_BASELINE'],
            metadata: { supports_replay: true },
            resetStrategy: 'RESET_TO_PRESET',
            resetSemantics: { drop_runtime_events: true },
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          tenantId: 'tenant_sandbox_001',
          tenantName: 'Sandbox Tenant',
          tenantStatus: 'PENDING',
          presetCode: 'BASELINE',
          scenarioCode: 'PAYIN_BASELINE',
          createdAt: '2026-03-08T09:00:00Z',
        }),
      });

    render(<SandboxPage />);

    expect(await screen.findByLabelText(/tenant name/i)).toBeInTheDocument();
    expect(screen.getAllByText('Baseline Sandbox').length).toBeGreaterThan(0);

    fireEvent.change(screen.getByLabelText(/tenant name/i), {
      target: { value: 'Sandbox Tenant' },
    });
    fireEvent.click(screen.getByRole('button', { name: /seed tenant/i }));

    await waitFor(() => {
      expect(screen.getAllByText(/tenant_sandbox_001/i).length).toBeGreaterThan(0);
    });

    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      '/api/proxy/v1/admin/sandbox/seed',
      expect.objectContaining({
        method: 'POST',
      }),
    );
  });

  it('loads replay data and shows the placeholder action state for unavailable flows', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            code: 'BASELINE',
            name: 'Baseline Sandbox',
            seedPackageVersion: '2026-03-08',
            defaultScenarios: ['PAYIN_BASELINE'],
            metadata: { supports_replay: true },
            resetStrategy: 'RESET_TO_PRESET',
            resetSemantics: { drop_runtime_events: true },
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          journeyId: 'intent_sandbox_001',
          generatedAt: '2026-03-08T11:00:00Z',
          redactionApplied: true,
          entries: [
            {
              sequence: 1,
              source: 'webhook',
              referenceId: 'evt_intent_sandbox_001',
              occurredAt: '2026-03-08T11:00:00Z',
              label: 'Sandbox replay placeholder',
              status: 'PENDING_IMPLEMENTATION',
              payload: {
                apiSecret: '[REDACTED]',
              },
            },
          ],
        }),
      });

    render(<SandboxPage />);

    expect(await screen.findByLabelText(/journey id/i)).toBeInTheDocument();
    expect(screen.getAllByText('Baseline Sandbox').length).toBeGreaterThan(0);

    fireEvent.change(screen.getByLabelText(/journey id/i), {
      target: { value: 'intent_sandbox_001' },
    });
    fireEvent.click(screen.getByRole('button', { name: /load replay/i }));

    await waitFor(() => {
      expect(screen.getAllByText(/pending_implementation/i).length).toBeGreaterThan(0);
    });
    expect(screen.getByText(/\[REDACTED\]/i)).toBeInTheDocument();
    expect(screen.getByText(/scenario execution will land/i)).toBeInTheDocument();
    expect(screen.getByText(/reset workflow stays bounded/i)).toBeInTheDocument();
  });
});
