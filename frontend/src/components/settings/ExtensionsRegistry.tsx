"use client";

import { useEffect, useState } from "react";
import { Loader2, Puzzle } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type ExtensionsResponse = {
  actionMode: string;
  actions: Array<{
    actionId: string;
    label: string;
    description: string;
    enabled: boolean;
  }>;
};

export default function ExtensionsRegistry() {
  const [data, setData] = useState<ExtensionsResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      const response = await fetch("/api/proxy/v1/admin/extensions");
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
            <Puzzle className="h-4 w-4 text-muted-foreground" />
            Extensions Registry
          </CardTitle>
          <CardDescription>
            Review whitelisted extension actions only. This is not a plugin runtime.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading extension registry...
            </div>
          ) : (
            data?.actions.map((action) => (
              <div key={action.actionId} className="rounded-lg border p-4 text-sm">
                <div className="font-medium">{action.label}</div>
                <div>{action.description}</div>
                <div>Enabled: {action.enabled ? "yes" : "no"}</div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
