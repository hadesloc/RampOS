import React from "react";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import KybPage from "@/app/[locale]/(admin)/compliance/kyb/page";

const mockFetch = vi.fn();

describe("KybPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads the kyb review queue", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        actionMode: "review_only",
        queue: [
          {
            entityId: "biz_review_001",
            legalName: "Ramp Ops Vietnam Ltd",
            reviewStatus: "needs_review",
            summary: {
              missingRequirements: ["shareholder_register"],
              reviewFlags: ["foreign_ownership_concentration"],
            },
          },
        ],
      }),
    });

    render(<KybPage />);

    expect(await screen.findByText(/ramp ops vietnam ltd/i)).toBeInTheDocument();
    expect(screen.getByText(/shareholder_register/i)).toBeInTheDocument();
  });
});
