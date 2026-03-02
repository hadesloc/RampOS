"use client";

import { useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { PageHeader } from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useToast } from "@/components/ui/use-toast";
import {
  useCheckCustodyPolicy,
  useCustodyPolicy,
  useGenerateCustodyKey,
  useSignCustodyUserOperation,
  useUpdateCustodyPolicy,
} from "@/hooks/use-admin-custody";

const DEFAULT_USER_OP = {
  sender: "0x0000000000000000000000000000000000000000",
  nonce: "1",
  initCode: "0x",
  callData: "0x",
  callGasLimit: "100000",
  verificationGasLimit: "100000",
  preVerificationGas: "21000",
  maxFeePerGas: "1000000000",
  maxPriorityFeePerGas: "1000000000",
  paymasterAndData: "0x",
  signature: "0x",
};

export default function AdminCustodyPage() {
  const { toast } = useToast();

  const [userId, setUserId] = useState("user-1");
  const [policyWhitelist, setPolicyWhitelist] = useState("");
  const [dailyLimit, setDailyLimit] = useState("1000000");
  const [requireMultiApprovalAbove, setRequireMultiApprovalAbove] = useState("500000");
  const [enabled, setEnabled] = useState(true);

  const [toAddress, setToAddress] = useState("0x0000000000000000000000000000000000000001");
  const [amount, setAmount] = useState("100");
  const [currency, setCurrency] = useState("USDC");
  const [chainId, setChainId] = useState("8453");

  const [userOperationText, setUserOperationText] = useState(
    JSON.stringify(DEFAULT_USER_OP, null, 2),
  );

  const [lastGeneratedKey, setLastGeneratedKey] = useState<string>("");
  const [lastSignature, setLastSignature] = useState<string>("");
  const [lastPolicyDecision, setLastPolicyDecision] = useState<string>("");

  const {
    data: policy,
    isLoading: policyLoading,
    refetch: refetchPolicy,
  } = useCustodyPolicy(userId);

  const generateKeyMutation = useGenerateCustodyKey();
  const signMutation = useSignCustodyUserOperation();
  const updatePolicyMutation = useUpdateCustodyPolicy();
  const checkPolicyMutation = useCheckCustodyPolicy();

  useEffect(() => {
    if (!policy) return;
    setPolicyWhitelist(policy.whitelistAddresses.join(", "));
    setDailyLimit(policy.dailyLimit);
    setRequireMultiApprovalAbove(policy.requireMultiApprovalAbove);
    setEnabled(policy.enabled);
  }, [policy]);

  const whitelistAddresses = useMemo(
    () =>
      policyWhitelist
        .split(",")
        .map((v) => v.trim())
        .filter(Boolean),
    [policyWhitelist],
  );

  const handleGenerateKey = async () => {
    try {
      const result = await generateKeyMutation.mutateAsync(userId);
      setLastGeneratedKey(result.publicKey);
      toast({ title: "Custody key generated", description: `Generation ${result.generation}` });
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: "Generate key failed",
        description: err.message || "An error occurred",
      });
    }
  };

  const handleSign = async () => {
    try {
      const parsed = JSON.parse(userOperationText);
      const result = await signMutation.mutateAsync({ userId, userOperation: parsed });
      setLastSignature(result.signature);
      toast({ title: "UserOperation signed", description: result.algorithm });
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: "Sign failed",
        description: err.message || "Invalid userOperation JSON",
      });
    }
  };

  const handleUpdatePolicy = async () => {
    try {
      const result = await updatePolicyMutation.mutateAsync({
        userId,
        whitelistAddresses,
        dailyLimit,
        requireMultiApprovalAbove,
        enabled,
      });
      setLastPolicyDecision(`Policy updated at ${result.updatedAt}`);
      toast({ title: "Policy updated" });
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: "Update policy failed",
        description: err.message || "An error occurred",
      });
    }
  };

  const handleCheckPolicy = async () => {
    try {
      const result = await checkPolicyMutation.mutateAsync({
        userId,
        toAddress,
        amount,
        currency,
        chainId: chainId || undefined,
      });
      setLastPolicyDecision(result.reason ? `${result.decision}: ${result.reason}` : result.decision);
      toast({ title: "Policy checked", description: result.decision });
    } catch (err: any) {
      toast({
        variant: "destructive",
        title: "Check policy failed",
        description: err.message || "An error occurred",
      });
    }
  };

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Custody Management"
        description="Generate MPC keys, sign UserOperation, and manage custody policy"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Custody" }]}
        actions={
          <Button variant="outline" size="icon" onClick={() => refetchPolicy()} disabled={policyLoading}>
            <RefreshCw className={`h-4 w-4 ${policyLoading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <Card>
        <CardHeader>
          <CardTitle>User Context</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <input
            className="w-full rounded-md border bg-background px-3 py-2 text-sm"
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            placeholder="user id"
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Key Generation</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Button onClick={handleGenerateKey} disabled={generateKeyMutation.isPending || !userId}>
            {generateKeyMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Generate Custody Key
          </Button>
          {lastGeneratedKey && (
            <div className="rounded-md border p-3 text-sm break-all">
              <div className="font-medium">Last Public Key</div>
              <div className="text-muted-foreground">{lastGeneratedKey}</div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>UserOperation Signing</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <textarea
            className="h-56 w-full rounded-md border bg-background px-3 py-2 text-xs font-mono"
            value={userOperationText}
            onChange={(e) => setUserOperationText(e.target.value)}
          />
          <Button onClick={handleSign} disabled={signMutation.isPending || !userId}>
            {signMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Sign UserOperation
          </Button>
          {lastSignature && (
            <div className="rounded-md border p-3 text-sm break-all">
              <div className="font-medium">Last Signature</div>
              <div className="text-muted-foreground">{lastSignature}</div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Policy Configuration</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-2">
          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">Whitelist Addresses (comma-separated)</label>
            <input
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={policyWhitelist}
              onChange={(e) => setPolicyWhitelist(e.target.value)}
              placeholder="0xabc..., 0xdef..."
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Daily Limit</label>
            <input
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={dailyLimit}
              onChange={(e) => setDailyLimit(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Require Multi Approval Above</label>
            <input
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              value={requireMultiApprovalAbove}
              onChange={(e) => setRequireMultiApprovalAbove(e.target.value)}
            />
          </div>
          <label className="flex items-center gap-2 text-sm md:col-span-2">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
              className="rounded"
            />
            Enable Policy
          </label>
          <div className="md:col-span-2">
            <Button onClick={handleUpdatePolicy} disabled={updatePolicyMutation.isPending || !userId}>
              {updatePolicyMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Save Policy
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Policy Check</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-2">
          <input
            className="rounded-md border bg-background px-3 py-2 text-sm md:col-span-2"
            value={toAddress}
            onChange={(e) => setToAddress(e.target.value)}
            placeholder="to address"
          />
          <input
            className="rounded-md border bg-background px-3 py-2 text-sm"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="amount"
          />
          <input
            className="rounded-md border bg-background px-3 py-2 text-sm"
            value={currency}
            onChange={(e) => setCurrency(e.target.value)}
            placeholder="currency"
          />
          <input
            className="rounded-md border bg-background px-3 py-2 text-sm md:col-span-2"
            value={chainId}
            onChange={(e) => setChainId(e.target.value)}
            placeholder="chain id"
          />
          <div className="md:col-span-2">
            <Button onClick={handleCheckPolicy} disabled={checkPolicyMutation.isPending || !userId}>
              {checkPolicyMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Check Policy
            </Button>
          </div>
          {lastPolicyDecision && (
            <div className="md:col-span-2 rounded-md border p-3 text-sm">
              <div className="font-medium">Last Policy Result</div>
              <div className="text-muted-foreground">{lastPolicyDecision}</div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
