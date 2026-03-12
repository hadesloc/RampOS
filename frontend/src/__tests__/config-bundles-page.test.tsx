import React from "react";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import ConfigBundlesPage from "@/app/[locale]/(admin)/settings/config-bundles/page";

const mockFetch = vi.fn();

describe("ConfigBundlesPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads config bundle summary", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        bundle: {
          bundleId: "cfg_bundle_demo_001",
          tenantName: "RampOS Demo Tenant",
          actionMode: "whitelisted_only",
          sections: ["branding", "domains"],
          approvalStatus: "fallback",
          source: "fallback",
          rolloutScope: { scope: "tenant" },
        },
      }),
    });

    render(<ConfigBundlesPage />);

    expect(await screen.findByText(/cfg_bundle_demo_001/i)).toBeInTheDocument();
    expect(screen.getByText(/whitelisted_only/i)).toBeInTheDocument();
    expect(screen.getByText(/approval: fallback/i)).toBeInTheDocument();
    expect(screen.getByText(/source: fallback/i)).toBeInTheDocument();
  });
});
