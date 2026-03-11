import React from "react";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import PassportAdminPage from "@/app/[locale]/(admin)/compliance/passport/page";

const mockFetch = vi.fn();

describe("PassportAdminPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads passport queue", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        actionMode: "consent_review",
        queue: [
          {
            packageId: "pkg_passport_active_001",
            userId: "user_passport_001",
            consentStatus: "granted",
            reviewStatus: "pending_review",
            targetTenantId: "tenant_review",
          },
        ],
      }),
    });

    render(<PassportAdminPage />);

    expect(await screen.findByText(/pkg_passport_active_001/i)).toBeInTheDocument();
    expect(screen.getByText(/pending_review/i)).toBeInTheDocument();
  });
});
