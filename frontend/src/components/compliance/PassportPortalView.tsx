"use client";

import { useEffect, useState } from "react";
import { Loader2, ShieldCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { kycApi } from "@/lib/portal-api";

type PassportSummary = {
  packageId: string;
  sourceTenantId: string;
  status: string;
  consentStatus: string;
  destinationTenantId?: string | null;
  fieldsShared: string[];
  expiresAt?: string | null;
  revokedAt?: string | null;
  reuseAllowed: boolean;
};

type KycStatus = {
  status: string;
  tier: number;
  passportSummary?: PassportSummary | null;
};

export default function PassportPortalView() {
  const [data, setData] = useState<KycStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        setData(await kycApi.getStatus());
      } catch (requestError) {
        setError(requestError instanceof Error ? requestError.message : "Passport status unavailable");
      } finally {
        setLoading(false);
      }
    };

    void load();
  }, []);

  const passport = data?.passportSummary;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Reusable KYC Passport</CardTitle>
          <CardDescription>
            Review vault availability, consent state, and whether verification can be reused.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading passport status...
            </div>
          ) : error ? (
            <div className="text-sm text-destructive">{error}</div>
          ) : passport ? (
            <>
              <div className="flex items-center gap-2">
                <ShieldCheck className="h-4 w-4 text-emerald-600" />
                <span className="font-medium">{passport.packageId}</span>
              </div>
              <div className="grid gap-2 text-sm md:grid-cols-2">
                <div>Consent: {passport.consentStatus}</div>
                <div>Reuse allowed: {passport.reuseAllowed ? "yes" : "no"}</div>
                <div>Source tenant: {passport.sourceTenantId}</div>
                <div>Destination: {passport.destinationTenantId ?? "not scoped"}</div>
              </div>
              <div className="text-sm text-muted-foreground">
                Shared fields: {passport.fieldsShared.join(", ")}
              </div>
              <Button variant="outline" disabled>
                Share / Revoke coming next
              </Button>
            </>
          ) : (
            <div className="text-sm text-muted-foreground">
              No reusable passport package is available yet.
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
