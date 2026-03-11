import React from "react";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { createAdminSessionToken } from "@/lib/admin-auth";

const cookiesMock = vi.fn();
const redirectMock = vi.fn();

vi.mock("next/headers", () => ({
  cookies: cookiesMock,
}));

vi.mock("@/navigation", () => ({
  redirect: (...args: unknown[]) => redirectMock(...args),
}));

vi.mock("@/components/layout/sidebar", () => ({
  default: () => <aside>Sidebar</aside>,
}));

vi.mock("@/components/layout/page-container", () => ({
  PageContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="page-container">{children}</div>
  ),
}));

vi.mock("@/components/ui/command-palette", () => ({
  CommandPalette: () => <div>Command Palette</div>,
}));

describe("AdminLayout", () => {
  beforeEach(() => {
    vi.resetModules();
    cookiesMock.mockReset();
    redirectMock.mockReset();
    redirectMock.mockImplementation(() => {
      throw new Error("NEXT_REDIRECT");
    });
    delete process.env.RAMPOS_ADMIN_KEY;
  });

  it("redirects unauthenticated requests to the locale-aware admin login route", async () => {
    process.env.RAMPOS_ADMIN_KEY = "test-admin-key";
    cookiesMock.mockResolvedValue({
      get: vi.fn(() => undefined),
    });

    const { default: AdminLayout } = await import("@/app/[locale]/(admin)/layout");

    await expect(
      AdminLayout({
        children: <div>Protected admin page</div>,
        params: Promise.resolve({ locale: "en" }),
      }),
    ).rejects.toThrow("NEXT_REDIRECT");

    expect(redirectMock).toHaveBeenCalledWith("/en/admin-login");
  });

  it("renders admin pages when the request has a valid admin session", async () => {
    const adminKey = "test-admin-key";
    process.env.RAMPOS_ADMIN_KEY = adminKey;
    const token = createAdminSessionToken(adminKey, 60);

    cookiesMock.mockResolvedValue({
      get: vi.fn(() => ({ value: token })),
    });

    const { default: AdminLayout } = await import("@/app/[locale]/(admin)/layout");
    const view = await AdminLayout({
      children: <div>Protected admin page</div>,
      params: Promise.resolve({ locale: "vi" }),
    });

    render(view);

    expect(screen.getByText("Protected admin page")).toBeInTheDocument();
    expect(redirectMock).not.toHaveBeenCalled();
  });
});
