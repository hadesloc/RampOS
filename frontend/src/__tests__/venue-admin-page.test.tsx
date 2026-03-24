import React from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import VenueAdminPage from "@/app/[locale]/(admin)/venue/page";

vi.mock("@/components/venue/VenueFundingWorkbench", () => ({
  VenueFundingWorkbench: () => <div>Venue Funding Workbench</div>,
}));

describe("VenueAdminPage", () => {
  it("renders the bounded admin venue review workspace", () => {
    render(<VenueAdminPage />);

    expect(screen.getByText(/Venue Funding Workbench/i)).toBeInTheDocument();
  });
});
