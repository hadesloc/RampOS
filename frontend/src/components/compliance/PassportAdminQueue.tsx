"use client";

import { useEffect, useState } from "react";
import { Loader2, ShieldAlert } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type PassportQueueItem = {
  packageId: string;
  userId: string;
  sourceTenantId: string;
  targetTenantId: string;
  status: string;
  consentStatus: string;
  reviewStatus: string;
  fieldsShared: string[];
};

type QueueResponse = {
  queue: PassportQueueItem[];
  actionMode: string;
};

export default function PassportAdminQueue() {
  const [data, setData] = useState<QueueResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      const response = await fetch("/api/proxy/v1/admin/passport/queue");
      const payload = await response.json();
      setData(payload);
      setLoading(false);
    };

    void load();
  }, []);

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldAlert className="h-4 w-4 text-muted-foreground" />
            Passport Queue
          </CardTitle>
          <CardDescription>
            Review shared-vault packages, consent state, destination tenant, and freshness.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading passport queue...
            </div>
          ) : (
            data?.queue.map((item) => (
              <div key={item.packageId} className="rounded-lg border p-4 text-sm">
                <div className="font-medium">{item.packageId}</div>
                <div>User: {item.userId}</div>
                <div>Consent: {item.consentStatus}</div>
                <div>Review: {item.reviewStatus}</div>
                <div>Destination: {item.targetTenantId}</div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
