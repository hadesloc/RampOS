import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import RescreeningPage from "@/app/[locale]/(admin)/compliance/rescreening/page";

const mockFetch = vi.fn();

describe("RescreeningPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads due runs and applies a restriction action", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => [
          {
            userId: "user_rescreen_due",
            status: "pending",
            kycStatus: "VERIFIED",
            nextRunAt: "2026-03-09T10:00:00Z",
            triggerKind: "document_expiry",
            priority: "high",
            restrictionStatus: "NONE",
            alertCodes: ["document_expiry_due"],
          },
        ],
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          userId: "user_rescreen_due",
          restrictionStatus: "RESTRICTED",
          reason: "Triggered from rescreening queue",
          updatedAt: "2026-03-09T10:05:00Z",
        }),
      });

    render(<RescreeningPage />);

    expect(await screen.findByText(/user_rescreen_due/i)).toBeInTheDocument();
    expect(screen.getAllByText(/document expiry/i).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: /apply restriction/i }));

    await waitFor(() => {
      expect(screen.getByText(/applied restriction for user_rescreen_due/i)).toBeInTheDocument();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(1, "/api/proxy/v1/admin/rescreening/runs");
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/rescreening/users/user_rescreen_due/restrict",
      expect.objectContaining({
        method: "POST",
      }),
    );
  });

  it("renders an empty state when no users are due", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => [],
    });

    render(<RescreeningPage />);

    expect(await screen.findByText(/no users are currently due for continuous rescreening/i)).toBeInTheDocument();
  });
});
