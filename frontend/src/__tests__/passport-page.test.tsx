import React from "react";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import PassportPortalPage from "@/app/[locale]/portal/kyc/passport/page";

const mockFetch = vi.fn();

describe("PassportPortalPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads passport summary from portal kyc status", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        status: "VERIFIED",
        tier: 2,
        passportSummary: {
          packageId: "pkg_passport_001",
          sourceTenantId: "tenant_origin",
          consentStatus: "granted",
          destinationTenantId: "tenant_partner",
          fieldsShared: ["identity", "sanctions"],
          reuseAllowed: true,
        },
      }),
    });

    render(<PassportPortalPage />);

    expect(await screen.findByText(/pkg_passport_001/i)).toBeInTheDocument();
    expect(screen.getByText(/reuse allowed: yes/i)).toBeInTheDocument();
  });
});
