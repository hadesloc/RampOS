import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import IncidentsPage from "@/app/[locale]/(admin)/incidents/page";

const mockFetch = vi.fn();

describe("IncidentsPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads incident search summary and timeline through the bounded admin API", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          data: [
            {
              incidentId: "incident_intent_intent_incident_001",
              matchedBy: ["intentId"],
              relatedReferenceIds: ["evt_incident_001", "intent_incident_001"],
              entryCount: 2,
              recommendationCount: 1,
              latestStatus: "PROCESSING",
              latestOccurredAt: "2026-03-09T11:00:00Z",
            },
          ],
          total: 1,
        }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          incidentId: "incident_intent_intent_incident_001",
          generatedAt: "2026-03-09T11:01:00Z",
          actionMode: "recommendation_only",
          entries: [
            {
              sequence: 1,
              sourceKind: "webhook",
              sourceReferenceId: "evt_incident_001",
              occurredAt: "2026-03-09T10:58:00Z",
              label: "Webhook intent.status.changed",
              status: "FAILED",
              confidence: "confirmed",
              relatedReferenceIds: ["intent_incident_001"],
              details: {
                intentId: "intent_incident_001",
                responseStatus: 504,
              },
            },
            {
              sequence: 2,
              sourceKind: "settlement",
              sourceReferenceId: "stl_incident_001",
              occurredAt: "2026-03-09T11:00:00Z",
              label: "Settlement status",
              status: "PROCESSING",
              confidence: "correlated",
              relatedReferenceIds: ["intent_incident_001", "RAMP-001"],
              details: {
                offrampIntentId: "intent_incident_001",
                bankReference: "RAMP-001",
              },
            },
          ],
          recommendations: [
            {
              code: "review_webhook_delivery",
              title: "Review webhook delivery",
              summary: "Validate endpoint health before replaying webhook delivery.",
              confidence: "confirmed",
              priority: "high",
              mode: "recommendation_only",
              relatedEntryIds: ["evt_incident_001"],
            },
          ],
        }),
      });

    render(<IncidentsPage />);

    fireEvent.change(screen.getByLabelText(/intent id/i), {
      target: { value: "intent_incident_001" },
    });
    fireEvent.click(screen.getByRole("button", { name: /load incident/i }));

    await waitFor(() => {
      expect(screen.getAllByText(/incident_intent_intent_incident_001/i).length).toBeGreaterThan(0);
    });

    expect(screen.getByText(/sla guardian/i)).toBeInTheDocument();
    expect(screen.getByText(/recommend escalation to webhook operations/i)).toBeInTheDocument();
    expect(screen.getByText(/review within 15 minutes/i)).toBeInTheDocument();
    expect(screen.getByText(/review webhook delivery/i)).toBeInTheDocument();
    expect(screen.getAllByTestId("incident-entry").length).toBe(2);
    expect(mockFetch).toHaveBeenNthCalledWith(
      1,
      "/api/proxy/v1/admin/incidents/search?intentId=intent_incident_001",
    );
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/incidents/timeline?intentId=intent_incident_001",
    );
  });
});
