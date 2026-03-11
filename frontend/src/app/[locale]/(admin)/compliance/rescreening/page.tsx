"use client";

import { useEffect, useState } from "react";

import RescreeningQueue, {
  type RescreeningRunRow,
} from "@/components/compliance/RescreeningQueue";

type RestrictionResponse = {
  userId: string;
  restrictionStatus: string;
  reason: string;
  updatedAt: string;
};

async function apiRequest<T>(endpoint: string, init?: RequestInit): Promise<T> {
  const url = `/api/proxy${endpoint}`;
  const options = init
    ? {
        ...init,
        headers: {
          "Content-Type": "application/json",
          ...init.headers,
        },
      }
    : undefined;

  const response = options ? await fetch(url, options) : await fetch(url);
  if (!response.ok) {
    let message = "Request failed";
    try {
      const payload = (await response.json()) as {
        message?: string;
        error?: { message?: string };
      };
      message = payload.message ?? payload.error?.message ?? message;
    } catch {
      // Use default fallback.
    }
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}

export default function RescreeningPage() {
  const [runs, setRuns] = useState<RescreeningRunRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [restrictingUserId, setRestrictingUserId] = useState<string | null>(null);
  const [notice, setNotice] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  const loadRuns = async (isRefresh = false) => {
    if (isRefresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(null);

    try {
      const response = await apiRequest<RescreeningRunRow[]>("/v1/admin/rescreening/runs");
      setRuns(response);
    } catch (requestError) {
      setRuns([]);
      setError(
        requestError instanceof Error ? requestError.message : "Failed to load rescreening queue",
      );
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadRuns();
  }, []);

  const handleApplyRestriction = async (userId: string) => {
    setRestrictingUserId(userId);
    setNotice(null);

    try {
      const updated = await apiRequest<RestrictionResponse>(
        `/v1/admin/rescreening/users/${userId}/restrict`,
        {
          method: "POST",
          body: JSON.stringify({
            restrictionStatus: "RESTRICTED",
            reason: "Triggered from rescreening queue",
          }),
        },
      );

      setRuns((current) =>
        current.map((row) =>
          row.userId === userId
            ? { ...row, restrictionStatus: updated.restrictionStatus }
            : row,
        ),
      );
      setNotice({
        type: "success",
        message: `Applied restriction for ${userId}.`,
      });
    } catch (requestError) {
      setNotice({
        type: "error",
        message:
          requestError instanceof Error ? requestError.message : "Failed to apply restriction",
      });
    } finally {
      setRestrictingUserId(null);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Continuous Rescreening</h1>
        <p className="text-muted-foreground">
          Monitor scheduled due-runs, alert-driven reviews, and bounded restriction actions for
          active users.
        </p>
      </div>

      <RescreeningQueue
        runs={runs}
        loading={loading}
        error={error}
        refreshing={refreshing}
        restrictingUserId={restrictingUserId}
        notice={notice}
        onRefresh={() => {
          setNotice(null);
          void loadRuns(true);
        }}
        onApplyRestriction={handleApplyRestriction}
      />
    </div>
  );
}
