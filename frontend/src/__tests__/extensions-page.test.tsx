import React from "react";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import ExtensionsPage from "@/app/[locale]/(admin)/settings/extensions/page";

const mockFetch = vi.fn();

describe("ExtensionsPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads whitelisted extension actions", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        actionMode: "whitelisted_only",
        actions: [
          {
            actionId: "branding.apply",
            label: "Apply branding bundle",
            description: "Imports approved branding fields from a config bundle.",
            enabled: true,
          },
        ],
      }),
    });

    render(<ExtensionsPage />);

    expect(await screen.findByText(/apply branding bundle/i)).toBeInTheDocument();
    expect(screen.getByText(/enabled: yes/i)).toBeInTheDocument();
  });
});
