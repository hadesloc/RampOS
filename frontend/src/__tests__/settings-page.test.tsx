import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import SettingsPage from "@/app/[locale]/(admin)/settings/page";

const listMock = vi.fn();
const updateConfigMock = vi.fn();
const regenerateKeysMock = vi.fn();
const regenerateWebhookSecretMock = vi.fn();
const toastMock = vi.fn();

vi.mock("@/lib/api", () => ({
  tenantsApi: {
    list: (...args: unknown[]) => listMock(...args),
    updateConfig: (...args: unknown[]) => updateConfigMock(...args),
    regenerateKeys: (...args: unknown[]) => regenerateKeysMock(...args),
    regenerateWebhookSecret: (...args: unknown[]) =>
      regenerateWebhookSecretMock(...args),
  },
}));

vi.mock("@/components/ui/use-toast", () => ({
  useToast: () => ({
    toast: toastMock,
  }),
}));

describe("SettingsPage", () => {
  beforeEach(() => {
    listMock.mockReset();
    updateConfigMock.mockReset();
    regenerateKeysMock.mockReset();
    regenerateWebhookSecretMock.mockReset();
    toastMock.mockReset();
  });

  it("fails closed when the admin session can access more than one tenant", async () => {
    listMock.mockResolvedValue([
      {
        id: "tenant_alpha",
        name: "Alpha",
        api_key_prefix: "alpha_",
        status: "ACTIVE",
        config: {},
        created_at: "2026-03-10T09:00:00Z",
        updated_at: "2026-03-10T09:00:00Z",
      },
      {
        id: "tenant_beta",
        name: "Beta",
        api_key_prefix: "beta_",
        status: "ACTIVE",
        config: {},
        created_at: "2026-03-10T09:00:00Z",
        updated_at: "2026-03-10T09:00:00Z",
      },
    ]);

    render(<SettingsPage />);

    expect(
      await screen.findByText(/settings require a single server-resolved tenant/i),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /save/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /regenerate/i })).not.toBeInTheDocument();
  });

  it("saves config and rotates secrets only for the single resolved tenant", async () => {
    listMock.mockResolvedValue([
      {
        id: "tenant_only",
        name: "Scoped Tenant",
        api_key_prefix: "tenant_",
        status: "ACTIVE",
        config: {
          webhook_url: "https://example.com/webhooks",
          rate_limit: "250",
        },
        created_at: "2026-03-10T09:00:00Z",
        updated_at: "2026-03-10T09:00:00Z",
      },
    ]);
    updateConfigMock.mockResolvedValue({});
    regenerateKeysMock.mockResolvedValue({
      api_key: "tenant_live_secret_key",
    });
    regenerateWebhookSecretMock.mockResolvedValue({
      webhook_secret: "whsec_live_secret_value",
    });

    render(<SettingsPage />);

    expect(await screen.findByDisplayValue("https://example.com/webhooks")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() => {
      expect(updateConfigMock).toHaveBeenCalledWith(
        "tenant_only",
        expect.objectContaining({
          webhook_url: "https://example.com/webhooks",
          rate_limit: "250",
        }),
      );
    });

    const regenerateButtons = screen.getAllByRole("button", { name: /regenerate/i });

    fireEvent.click(regenerateButtons[0]);
    fireEvent.click(regenerateButtons[1]);

    await waitFor(() => {
      expect(regenerateKeysMock).toHaveBeenCalledWith("tenant_only");
      expect(regenerateWebhookSecretMock).toHaveBeenCalledWith("tenant_only");
    });
  });
});
