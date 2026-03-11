import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import WebhooksPage from "@/app/[locale]/(admin)/webhooks/page";

const listMock = vi.fn();
const retryMock = vi.fn();
const toastMock = vi.fn();

vi.mock("@/lib/api", () => ({
  webhooksApi: {
    list: (...args: unknown[]) => listMock(...args),
    retry: (...args: unknown[]) => retryMock(...args),
  },
}));

vi.mock("@/components/ui/use-toast", () => ({
  useToast: () => ({
    toast: toastMock,
  }),
}));

describe("WebhooksPage", () => {
  beforeEach(() => {
    listMock.mockReset();
    retryMock.mockReset();
    toastMock.mockReset();
  });

  it("shows SLA guardian recommendations without exposing retry execution", async () => {
    listMock.mockResolvedValue({
      data: [
        {
          id: "wh_001",
          tenant_id: "tenant_001",
          event_type: "intent.payout.failed",
          payload: { url: "https://ops.example/webhooks" },
          status: "FAILED",
          attempts: 3,
          max_attempts: 5,
          last_attempt_at: "2026-03-10T09:00:00Z",
          next_attempt_at: "2026-03-10T09:05:00Z",
          last_error: "504 from downstream endpoint",
          response_status: 504,
          created_at: "2026-03-10T08:55:00Z",
        },
        {
          id: "wh_002",
          tenant_id: "tenant_001",
          event_type: "intent.payin.pending",
          payload: { url: "https://ops.example/webhooks" },
          status: "PENDING",
          attempts: 1,
          max_attempts: 5,
          next_attempt_at: "2026-03-10T09:08:00Z",
          response_status: undefined,
          created_at: "2026-03-10T09:01:00Z",
        },
      ],
      total: 2,
      page: 1,
      per_page: 20,
      total_pages: 1,
    });

    render(<WebhooksPage />);

    expect(await screen.findByText(/webhook sla guardian/i)).toBeInTheDocument();
    expect(screen.getByText(/1 failed needs review inside 15 min/i)).toBeInTheDocument();
    expect(screen.getByText(/recommend endpoint health review before any replay/i)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /retry/i })).not.toBeInTheDocument();
    expect(retryMock).not.toHaveBeenCalled();
  });

  it("passes status and event type filters through the API list call", async () => {
    listMock
      .mockResolvedValueOnce({
        data: [],
        total: 0,
        page: 1,
        per_page: 20,
        total_pages: 0,
      })
      .mockResolvedValueOnce({
        data: [],
        total: 0,
        page: 1,
        per_page: 20,
        total_pages: 0,
      })
      .mockResolvedValueOnce({
        data: [],
        total: 0,
        page: 1,
        per_page: 20,
        total_pages: 0,
      });

    render(<WebhooksPage />);

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledWith({});
    });

    fireEvent.change(screen.getByDisplayValue(/all statuses/i), {
      target: { value: "FAILED" },
    });

    await waitFor(() => {
      expect(listMock).toHaveBeenLastCalledWith({ status: "FAILED" });
    });

    fireEvent.change(screen.getByDisplayValue(/all event types/i), {
      target: { value: "intent.payout" },
    });

    await waitFor(() => {
      expect(listMock).toHaveBeenLastCalledWith({
        event_type: "intent.payout",
        status: "FAILED",
      });
    });
  });
});
