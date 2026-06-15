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

    expect((await screen.findAllByText(/cfg_bundle_demo_001/i)).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/whitelisted only/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/approval/i)).toBeInTheDocument();
    expect(screen.getAllByText(/fallback/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/source/i)).toBeInTheDocument();
  });
});
