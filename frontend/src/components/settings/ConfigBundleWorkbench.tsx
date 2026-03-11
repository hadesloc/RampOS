"use client";

import { useEffect, useState } from "react";
import { Download, Loader2 } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type BundleResponse = {
  bundle: {
    bundleId: string;
    tenantName: string;
    actionMode: string;
    sections: string[];
  };
};

export default function ConfigBundleWorkbench() {
  const [data, setData] = useState<BundleResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      const response = await fetch("/api/proxy/v1/admin/config-bundles/export");
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
            <Download className="h-4 w-4 text-muted-foreground" />
            Config Bundles
          </CardTitle>
          <CardDescription>
            Export approved tenant configuration sections and review import-safe bundle contents.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading config bundle...
            </div>
          ) : (
            <>
              <div className="font-medium">{data?.bundle.bundleId}</div>
              <div>Tenant: {data?.bundle.tenantName}</div>
              <div>Mode: {data?.bundle.actionMode}</div>
              <div>Sections: {data?.bundle.sections.join(", ")}</div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
