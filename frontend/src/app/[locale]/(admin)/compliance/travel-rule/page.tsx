"use client";

import { useEffect, useState } from "react";

import TravelRuleQueue, {
  type TravelRuleDisclosureRow,
  type TravelRuleExceptionRow,
  type TravelRuleRegistryRow,
} from "@/components/compliance/TravelRuleQueue";

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

export default function TravelRulePage() {
  const [registry, setRegistry] = useState<TravelRuleRegistryRow[]>([]);
  const [disclosures, setDisclosures] = useState<TravelRuleDisclosureRow[]>([]);
  const [exceptions, setExceptions] = useState<TravelRuleExceptionRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [retryingId, setRetryingId] = useState<string | null>(null);
  const [resolvingId, setResolvingId] = useState<string | null>(null);
  const [notice, setNotice] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  const loadTravelRule = async (isRefresh = false) => {
    if (isRefresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(null);

    const [registryResult, disclosureResult, exceptionResult] = await Promise.allSettled([
      apiRequest<TravelRuleRegistryRow[]>("/v1/admin/travel-rule/registry"),
      apiRequest<TravelRuleDisclosureRow[]>("/v1/admin/travel-rule/disclosures"),
      apiRequest<TravelRuleExceptionRow[]>("/v1/admin/travel-rule/exceptions"),
    ]);

    if (registryResult.status === "fulfilled") {
      setRegistry(registryResult.value);
    }
    if (disclosureResult.status === "fulfilled") {
      setDisclosures(disclosureResult.value);
    }
    if (exceptionResult.status === "fulfilled") {
      setExceptions(exceptionResult.value);
    }

    const rejected = [registryResult, disclosureResult, exceptionResult].find(
      (result) => result.status === "rejected",
    );
    if (rejected?.status === "rejected") {
      setError(rejected.reason instanceof Error ? rejected.reason.message : "Failed to load Travel Rule data");
    }

    setLoading(false);
    setRefreshing(false);
  };

  useEffect(() => {
    void loadTravelRule();
  }, []);

  const handleRetryDisclosure = async (disclosureId: string) => {
    setRetryingId(disclosureId);
    setNotice(null);

    try {
      const updated = await apiRequest<TravelRuleDisclosureRow>(
        `/v1/admin/travel-rule/disclosures/${disclosureId}/retry`,
        {
          method: "POST",
          body: JSON.stringify({ simulatedStatus: "SENT" }),
        },
      );

      setDisclosures((current) =>
        current.map((row) => (row.disclosureId === disclosureId ? updated : row)),
      );
      setNotice({
        type: "success",
        message: `Retried disclosure ${disclosureId}.`,
      });
      await loadTravelRule(true);
    } catch (requestError) {
      setNotice({
        type: "error",
        message: requestError instanceof Error ? requestError.message : "Retry failed",
      });
    } finally {
      setRetryingId(null);
    }
  };

  const handleResolveException = async (exceptionId: string) => {
    setResolvingId(exceptionId);
    setNotice(null);

    try {
      const updated = await apiRequest<TravelRuleExceptionRow>(
        `/v1/admin/travel-rule/exceptions/${exceptionId}/resolve`,
        {
          method: "POST",
          body: JSON.stringify({ resolutionNote: "Resolved from admin queue" }),
        },
      );

      setExceptions((current) =>
        current.map((row) => (row.exceptionId === exceptionId ? updated : row)),
      );
      setNotice({
        type: "success",
        message: `Resolved exception ${exceptionId}.`,
      });
      await loadTravelRule(true);
    } catch (requestError) {
      setNotice({
        type: "error",
        message: requestError instanceof Error ? requestError.message : "Resolve failed",
      });
    } finally {
      setResolvingId(null);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Travel Rule Queue</h1>
        <p className="text-muted-foreground">
          Monitor registry readiness, disclosure retries, and exception resolution for Travel Rule
          operations.
        </p>
      </div>

      <TravelRuleQueue
        registry={registry}
        disclosures={disclosures}
        exceptions={exceptions}
        loading={loading}
        error={error}
        refreshing={refreshing}
        retryingId={retryingId}
        resolvingId={resolvingId}
        notice={notice}
        onRefresh={() => {
          setNotice(null);
          void loadTravelRule(true);
        }}
        onRetryDisclosure={handleRetryDisclosure}
        onResolveException={handleResolveException}
      />
    </div>
  );
}
