"use client";

import { useEffect, useState } from "react";
import { Loader2, Network } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type ReviewItem = {
  entityId: string;
  legalName: string;
  reviewStatus: string;
  summary: {
    missingRequirements: string[];
    reviewFlags: string[];
  };
};

type ReviewResponse = {
  queue: ReviewItem[];
  actionMode: string;
};

export default function KybGraphReview() {
  const [data, setData] = useState<ReviewResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      const response = await fetch("/api/proxy/v1/admin/kyb/reviews");
      setData(await response.json());
      setLoading(false);
    };

    void load();
  }, []);

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Network className="h-4 w-4 text-muted-foreground" />
            KYB Ownership Review
          </CardTitle>
          <CardDescription>
            Review relational ownership edges, missing licensing docs, and review flags.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading KYB review graph...
            </div>
          ) : (
            data?.queue.map((item) => (
              <div key={item.entityId} className="rounded-lg border p-4 text-sm">
                <div className="font-medium">{item.legalName}</div>
                <div>Status: {item.reviewStatus}</div>
                <div>Missing: {item.summary.missingRequirements.join(", ")}</div>
                <div>Flags: {item.summary.reviewFlags.join(", ")}</div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
