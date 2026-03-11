import React from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import RiskLabPage from "@/app/[locale]/(admin)/risk-lab/page";

const mockFetch = vi.fn();

const catalogResponse = {
  entries: [
    {
      scorerKind: "RULE_BASED",
      label: "Rule-based scorer",
      supportsShadowCompare: true,
      safeFallback: "primary_default",
    },
    {
      scorerKind: "ONNX_HEURISTIC",
      label: "ONNX heuristic scorer",
      supportsShadowCompare: true,
      safeFallback: "heuristic_when_model_unloaded",
    },
  ],
};

const replayResponse = {
  replayId: "risk_replay_velocity_spike",
  primaryScore: {
    riskScore: {
      score: 78,
      riskFactors: [
        {
          ruleName: "velocity_1h_exceeded",
          contribution: 15,
          description: "8 txns in 1h (limit 5)",
        },
        {
          ruleName: "new_device_high_value",
          contribution: 10,
          description: "Transaction from new device with significant value",
        },
      ],
    },
    metadata: {
      ruleVersionId: "fraud-rules-v4",
      scorer: "rule_based",
      safeFallbackUsed: false,
      rawScore: 92,
      triggeredRules: ["velocity_1h_exceeded", "new_device_high_value"],
      topRiskFactors: [
        {
          ruleName: "velocity_1h_exceeded",
          contribution: 15,
          description: "8 txns in 1h (limit 5)",
        },
      ],
      featureSnapshot: {
        amountPercentile: 0.93,
        velocity1h: 8,
        velocity24h: 18,
        velocity7d: 42,
        timeOfDayAnomaly: 0.72,
        amountRoundingPattern: 0.8,
        recipientRecency: 1,
        historicalDisputeRate: 0.08,
        accountAgeDays: 4,
        amountToAvgRatio: 6.2,
        distinctRecipients24h: 7,
        deviceNovelty: 1,
        countryRisk: 0.82,
        isCrossBorder: 1,
        amountUsd: 24000,
        failedTxnCount24h: 4,
        cumulativeAmount24hUsd: 48000,
      },
    },
  },
  primaryDecision: {
    decision: "Review",
    decisionBasis: "score_in_review_band",
    boundaryDistance: 2,
    triggeredRules: ["velocity_1h_exceeded", "new_device_high_value"],
    topRiskFactors: ["velocity_1h_exceeded", "new_device_high_value"],
    thresholds: {
      allowBelow: 30,
      blockAbove: 80,
    },
  },
  challengerScore: {
    riskScore: {
      score: 84,
      riskFactors: [
        {
          ruleName: "onnx_velocity_signal",
          contribution: 18,
          description: "Velocity composite score 45.4",
        },
      ],
    },
    metadata: {
      ruleVersionId: "fraud-rules-v4",
      scorer: "onnx_heuristic",
      safeFallbackUsed: true,
      rawScore: 84,
      triggeredRules: ["onnx_velocity_signal"],
      topRiskFactors: [
        {
          ruleName: "onnx_velocity_signal",
          contribution: 18,
          description: "Velocity composite score 45.4",
        },
      ],
      featureSnapshot: {
        amountPercentile: 0.93,
        velocity1h: 8,
        velocity24h: 18,
        velocity7d: 42,
        timeOfDayAnomaly: 0.72,
        amountRoundingPattern: 0.8,
        recipientRecency: 1,
        historicalDisputeRate: 0.08,
        accountAgeDays: 4,
        amountToAvgRatio: 6.2,
        distinctRecipients24h: 7,
        deviceNovelty: 1,
        countryRisk: 0.82,
        isCrossBorder: 1,
        amountUsd: 24000,
        failedTxnCount24h: 4,
        cumulativeAmount24hUsd: 48000,
      },
    },
  },
  challengerDecision: {
    decision: "Block",
    decisionBasis: "score_above_block_threshold",
    boundaryDistance: 4,
    triggeredRules: ["onnx_velocity_signal"],
    topRiskFactors: ["onnx_velocity_signal"],
    thresholds: {
      allowBelow: 30,
      blockAbove: 80,
    },
  },
  scoreDelta: 6,
  graph: {
    nodes: [
      {
        id: "replay:risk_replay_velocity_spike",
        kind: "REPLAY",
        label: "Replay risk_replay_velocity_spike",
        weight: 78,
      },
      {
        id: "decision:risk_replay_velocity_spike",
        kind: "DECISION",
        label: "Review",
        weight: 2,
      },
      {
        id: "features:risk_replay_velocity_spike",
        kind: "FEATURE_SNAPSHOT",
        label: "Feature snapshot",
        weight: null,
      },
      {
        id: "factor:risk_replay_velocity_spike:velocity_1h_exceeded",
        kind: "RULE_FACTOR",
        label: "velocity_1h_exceeded",
        weight: 15,
      },
    ],
    edges: [
      {
        sourceId: "replay:risk_replay_velocity_spike",
        targetId: "decision:risk_replay_velocity_spike",
        kind: "EXPLAINS",
      },
      {
        sourceId: "features:risk_replay_velocity_spike",
        targetId: "replay:risk_replay_velocity_spike",
        kind: "EVALUATED_FROM",
      },
      {
        sourceId: "factor:risk_replay_velocity_spike:velocity_1h_exceeded",
        targetId: "decision:risk_replay_velocity_spike",
        kind: "TRIGGERED",
      },
    ],
  },
};

describe("RiskLabPage", () => {
  beforeEach(() => {
    mockFetch.mockReset();
    vi.stubGlobal("fetch", mockFetch);
  });

  it("loads the supported catalog and renders compare, replay, and explainability results", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => catalogResponse,
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => replayResponse,
      });

    render(<RiskLabPage />);

    expect(await screen.findByText(/rule-based scorer/i)).toBeInTheDocument();
    expect(
      screen.getByText(/load a replay to compare the primary scorer against the shadow lane/i),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /run replay/i }));

    await waitFor(() => {
      expect(screen.getByText(/risk_replay_velocity_spike/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/score delta/i)).toBeInTheDocument();
    expect(screen.getByText(/primary decision/i)).toBeInTheDocument();
    expect(screen.getByText(/challenger decision/i)).toBeInTheDocument();
    expect(screen.getAllByText(/review/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/^velocity_1h_exceeded$/i)).toBeInTheDocument();
    expect(screen.getByText(/^onnx_velocity_signal$/i)).toBeInTheDocument();
    expect(screen.getByText(/graph nodes/i)).toBeInTheDocument();

    expect(mockFetch.mock.calls[0]?.[0]).toBe("/api/proxy/v1/admin/risk-lab/catalog");
    expect(mockFetch).toHaveBeenNthCalledWith(
      2,
      "/api/proxy/v1/admin/risk-lab/replay",
      expect.objectContaining({
        method: "POST",
      }),
    );

    const replayRequest = mockFetch.mock.calls[1]?.[1];
    expect(replayRequest).toBeDefined();
    expect(JSON.parse(replayRequest.body as string)).toMatchObject({
      replayId: "risk_replay_velocity_spike",
      ruleVersionId: "fraud-rules-v4",
      challenger: {
        scorerKind: "ONNX_HEURISTIC",
      },
      featureVector: {
        velocity1h: 8,
        amountUsd: 24000,
        isCrossBorder: 1,
      },
    });
  });

  it("shows a replay loading state while waiting for compare results", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => catalogResponse,
      })
      .mockReturnValueOnce(new Promise(() => {}));

    render(<RiskLabPage />);

    expect(await screen.findByText(/onnx heuristic scorer/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /run replay/i }));

    expect(screen.getByRole("button", { name: /running replay/i })).toBeDisabled();
    expect(
      screen.getByText(/preparing compare and explanation surfaces for the selected replay/i),
    ).toBeInTheDocument();
  });

  it("renders a recoverable catalog error state", async () => {
    mockFetch.mockResolvedValueOnce({
      ok: false,
      json: async () => ({
        message: "Risk lab catalog unavailable",
      }),
    });

    render(<RiskLabPage />);

    expect(await screen.findByText(/risk lab catalog unavailable/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retry catalog/i })).toBeInTheDocument();
    expect(
      screen.getByText(/catalog data defines the bounded compare surface exposed by the backend/i),
    ).toBeInTheDocument();
  });

  it("renders replay request errors without replacing the workbench", async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => catalogResponse,
      })
      .mockResolvedValueOnce({
        ok: false,
        json: async () => ({
          message: "Replay contract unavailable",
        }),
      });

    render(<RiskLabPage />);

    expect(await screen.findByText(/rule-based scorer/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /run replay/i }));

    expect(await screen.findByText(/replay contract unavailable/i)).toBeInTheDocument();
    expect(screen.getByText(/adjust the feature snapshot or compare lane and rerun/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /run replay/i })).toBeInTheDocument();
  });
});
