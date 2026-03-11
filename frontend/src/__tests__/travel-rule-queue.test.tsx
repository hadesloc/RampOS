import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import TravelRulePage from "@/app/[locale]/(admin)/compliance/travel-rule/page";

const mockFetch = vi.fn();

describe("TravelRulePage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads registry, disclosures, exceptions, retries a disclosure, and resolves an exception", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            vaspCode: "vasp-sg-1",
            legalName: "Example VASP Ltd",
            jurisdictionCode: "SG",
            transportProfile: "trp-bridge",
            endpointUri: "https://vasp.example/travel-rule",
            review: { status: "APPROVED" },
            interoperability: { status: "READY" },
            supportsInbound: true,
            supportsOutbound: true,
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            disclosureId: "trd_001",
            direction: "OUTBOUND",
            stage: "FAILED",
            queueStatus: null,
            failureCount: 1,
            maxFailuresBeforeException: 3,
            attemptCount: 1,
            transportProfile: "trp-bridge",
            matchedPolicyCode: "fatf-default",
            action: "DISCLOSE_BEFORE_SETTLEMENT",
            retryRecommended: true,
            terminal: false,
            updatedAt: "2026-03-09T10:00:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            exceptionId: "tre_001",
            disclosureId: "trd_001",
            status: "OPEN",
            reasonCode: "TIMEOUT",
            updatedAt: "2026-03-09T10:05:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          disclosureId: "trd_001",
          direction: "OUTBOUND",
          stage: "SENT",
          queueStatus: null,
          failureCount: 1,
          maxFailuresBeforeException: 3,
          attemptCount: 2,
          transportProfile: "trp-bridge",
          matchedPolicyCode: "fatf-default",
          action: "DISCLOSE_BEFORE_SETTLEMENT",
          retryRecommended: false,
          terminal: false,
          updatedAt: "2026-03-09T10:10:00Z",
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            vaspCode: "vasp-sg-1",
            legalName: "Example VASP Ltd",
            jurisdictionCode: "SG",
            transportProfile: "trp-bridge",
            endpointUri: "https://vasp.example/travel-rule",
            review: { status: "APPROVED" },
            interoperability: { status: "READY" },
            supportsInbound: true,
            supportsOutbound: true,
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            disclosureId: "trd_001",
            direction: "OUTBOUND",
            stage: "SENT",
            queueStatus: null,
            failureCount: 1,
            maxFailuresBeforeException: 3,
            attemptCount: 2,
            transportProfile: "trp-bridge",
            matchedPolicyCode: "fatf-default",
            action: "DISCLOSE_BEFORE_SETTLEMENT",
            retryRecommended: false,
            terminal: false,
            updatedAt: "2026-03-09T10:10:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            exceptionId: "tre_001",
            disclosureId: "trd_001",
            status: "OPEN",
            reasonCode: "TIMEOUT",
            updatedAt: "2026-03-09T10:05:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          exceptionId: "tre_001",
          disclosureId: "trd_001",
          status: "RESOLVED",
          reasonCode: "TIMEOUT",
          updatedAt: "2026-03-09T10:15:00Z",
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            vaspCode: "vasp-sg-1",
            legalName: "Example VASP Ltd",
            jurisdictionCode: "SG",
            transportProfile: "trp-bridge",
            endpointUri: "https://vasp.example/travel-rule",
            review: { status: "APPROVED" },
            interoperability: { status: "READY" },
            supportsInbound: true,
            supportsOutbound: true,
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            disclosureId: "trd_001",
            direction: "OUTBOUND",
            stage: "FAILED",
            queueStatus: null,
            failureCount: 1,
            maxFailuresBeforeException: 3,
            attemptCount: 2,
            transportProfile: "trp-bridge",
            matchedPolicyCode: "fatf-default",
            action: "DISCLOSE_BEFORE_SETTLEMENT",
            retryRecommended: true,
            terminal: false,
            updatedAt: "2026-03-09T10:15:00Z",
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            exceptionId: "tre_001",
            disclosureId: "trd_001",
            status: "RESOLVED",
            reasonCode: "TIMEOUT",
            updatedAt: "2026-03-09T10:15:00Z",
          },
        ],
      });

    render(<TravelRulePage />);

    expect(await screen.findByText(/vasp-sg-1/i)).toBeInTheDocument();
    expect(screen.getAllByText(/trd_001/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/tre_001/i).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: /retry/i }));
    await waitFor(() => {
      expect(screen.getByText(/retried disclosure trd_001/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /resolve/i }));
    await waitFor(() => {
      expect(screen.getByText(/resolved exception tre_001/i)).toBeInTheDocument();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(1, "/api/proxy/v1/admin/travel-rule/registry");
    expect(mockFetch).toHaveBeenNthCalledWith(2, "/api/proxy/v1/admin/travel-rule/disclosures");
    expect(mockFetch).toHaveBeenNthCalledWith(3, "/api/proxy/v1/admin/travel-rule/exceptions");
    expect(mockFetch).toHaveBeenNthCalledWith(
      4,
      "/api/proxy/v1/admin/travel-rule/disclosures/trd_001/retry",
      expect.objectContaining({
        method: "POST",
      }),
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      8,
      "/api/proxy/v1/admin/travel-rule/exceptions/tre_001/resolve",
      expect.objectContaining({
        method: "POST",
      }),
    );
  });

  it("renders empty states without crashing when every section is empty", async () => {
    mockFetch
      .mockResolvedValueOnce({ ok: true, json: async () => [] })
      .mockResolvedValueOnce({ ok: true, json: async () => [] })
      .mockResolvedValueOnce({ ok: true, json: async () => [] });

    render(<TravelRulePage />);

    expect(await screen.findByText(/no travel rule vasp records/i)).toBeInTheDocument();
    expect(screen.getByText(/no travel rule disclosures/i)).toBeInTheDocument();
    expect(screen.getByText(/no travel rule exceptions/i)).toBeInTheDocument();
  });
});
