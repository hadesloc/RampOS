"use client";

import { useEffect, useMemo, useState } from "react";
import { KeyRound, Loader2, RefreshCw, ShieldCheck, PenTool } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import {
  useCheckCustodyPolicy,
  useCustodyPolicy,
  useGenerateCustodyKey,
  useSignCustodyUserOperation,
  useUpdateCustodyPolicy,
} from "@/hooks/use-admin-custody";
import {
  PageHeader,
  StatGrid,
  StatCard,
  Panel,
  StatusBadge,
} from "@/components/shared";

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

const inputClass =
  "w-full rounded-md border border-white/[0.08] bg-[#111113] px-3 py-2 text-sm font-mono focus:outline-none focus:border-[#7B61FF]/50 placeholder:text-muted-foreground/50";

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
    JSON.stringify(DEFAULT_USER_OP, null, 2)
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
    [policyWhitelist]
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
      setLastPolicyDecision(
        result.reason ? `${result.decision}: ${result.reason}` : result.decision
      );
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
    <main className="p-page flex flex-col gap-section">
      <PageHeader
        title="Custody Management"
        description="Generate MPC keys, sign UserOperations, and manage custody policies"
        actions={
          <Button
            variant="outline"
            size="icon"
            onClick={() => refetchPolicy()}
            disabled={policyLoading}
          >
            <RefreshCw className={`h-4 w-4 ${policyLoading ? "animate-spin" : ""}`} />
          </Button>
        }
      />

      <StatGrid cols={3}>
        <StatCard
          title="Active User"
          value={userId || "None"}
          icon={<KeyRound className="h-4 w-4" />}
          accentColor="violet"
        />
        <StatCard
          title="Policy Status"
          value={policyLoading ? "-" : policy ? (policy.enabled ? "Enabled" : "Disabled") : "N/A"}
          icon={<ShieldCheck className="h-4 w-4" />}
          accentColor={policy?.enabled ? "green" : "amber"}
          loading={policyLoading}
        />
        <StatCard
          title="Daily Limit"
          value={policyLoading ? "-" : policy?.dailyLimit ?? "-"}
          accentColor="cyan"
          loading={policyLoading}
        />
      </StatGrid>

      {/* User Context */}
      <Panel header={{ title: "User Context", description: "Select the user to manage custody for" }}>
        <div className="flex gap-3">
          <input
            className={inputClass + " flex-1"}
            value={userId}
            onChange={(e) => setUserId(e.target.value)}
            placeholder="User ID"
          />
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetchPolicy()}
            disabled={policyLoading || !userId}
            className="border-white/[0.08]"
          >
            {policyLoading ? (
              <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-3.5 w-3.5" />
            )}
            Load Policy
          </Button>
        </div>
        {policy && (
          <div className="mt-4 rounded-lg border border-white/[0.06] bg-white/[0.02] p-3 text-xs space-y-1.5">
            <div className="flex gap-2 items-center">
              <span className="text-muted-foreground w-40">Policy enabled</span>
              <StatusBadge status={policy.enabled ? "enabled" : "disabled"} />
            </div>
            <div className="flex gap-2">
              <span className="text-muted-foreground w-40">Daily limit</span>
              <span className="font-mono tabular-nums">{policy.dailyLimit}</span>
            </div>
            <div className="flex gap-2">
              <span className="text-muted-foreground w-40">Multi-approval above</span>
              <span className="font-mono tabular-nums">{policy.requireMultiApprovalAbove}</span>
            </div>
            <div className="flex gap-2">
              <span className="text-muted-foreground w-40">Whitelist</span>
              <span className="font-mono">
                {policy.whitelistAddresses.length} address
                {policy.whitelistAddresses.length !== 1 ? "es" : ""}
              </span>
            </div>
          </div>
        )}
      </Panel>

      <div className="grid gap-section md:grid-cols-2">
        {/* Key Generation */}
        <Panel
          header={{
            title: "Key Generation",
            description: "Generate a new MPC custody key for the user",
          }}
        >
          <div className="space-y-4">
            <Button
              onClick={handleGenerateKey}
              disabled={generateKeyMutation.isPending || !userId}
              className="bg-[#7B61FF] hover:bg-[#7B61FF]/90 text-white"
            >
              {generateKeyMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              <KeyRound className="mr-2 h-4 w-4" />
              Generate Custody Key
            </Button>
            {lastGeneratedKey && (
              <div className="rounded-lg border border-[#00FF87]/20 bg-[#00FF87]/5 p-3 space-y-1">
                <p className="text-xs font-semibold text-[#00FF87]">Last Public Key</p>
                <p className="text-xs font-mono break-all text-foreground">{lastGeneratedKey}</p>
              </div>
            )}
          </div>
        </Panel>

        {/* UserOperation Signing */}
        <Panel
          header={{
            title: "UserOperation Signing",
            description: "Sign an ERC-4337 UserOperation with the user's MPC key",
          }}
        >
          <div className="space-y-4">
            <textarea
              className="h-40 w-full rounded-md border border-white/[0.08] bg-[#111113] px-3 py-2 text-xs font-mono resize-none focus:outline-none focus:border-[#7B61FF]/50"
              value={userOperationText}
              onChange={(e) => setUserOperationText(e.target.value)}
            />
            <Button
              onClick={handleSign}
              disabled={signMutation.isPending || !userId}
              variant="outline"
              className="border-[#00D4FF]/30 text-[#00D4FF] hover:bg-[#00D4FF]/10"
            >
              {signMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              <PenTool className="mr-2 h-4 w-4" />
              Sign UserOperation
            </Button>
            {lastSignature && (
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.02] p-3 space-y-1">
                <p className="text-xs font-semibold text-muted-foreground">Last Signature</p>
                <p className="text-xs font-mono break-all text-foreground">{lastSignature}</p>
              </div>
            )}
          </div>
        </Panel>
      </div>

      <div className="grid gap-section md:grid-cols-2">
        {/* Policy Configuration */}
        <Panel
          header={{
            title: "Policy Configuration",
            description: "Set withdrawal rules and spending limits",
          }}
        >
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                Whitelist Addresses (comma-separated)
              </label>
              <input
                className={inputClass}
                value={policyWhitelist}
                onChange={(e) => setPolicyWhitelist(e.target.value)}
                placeholder="0xabc..., 0xdef..."
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                  Daily Limit
                </label>
                <input
                  className={inputClass}
                  value={dailyLimit}
                  onChange={(e) => setDailyLimit(e.target.value)}
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                  Multi-Approval Above
                </label>
                <input
                  className={inputClass}
                  value={requireMultiApprovalAbove}
                  onChange={(e) => setRequireMultiApprovalAbove(e.target.value)}
                />
              </div>
            </div>
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="checkbox"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
                className="rounded"
              />
              Enable Policy
            </label>
            <Button
              onClick={handleUpdatePolicy}
              disabled={updatePolicyMutation.isPending || !userId}
              className="w-full bg-[#7B61FF] hover:bg-[#7B61FF]/90 text-white"
            >
              {updatePolicyMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              Save Policy
            </Button>
          </div>
        </Panel>

        {/* Policy Check */}
        <Panel
          header={{
            title: "Policy Check",
            description: "Simulate a transaction against the current policy",
          }}
        >
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                Destination Address
              </label>
              <input
                className={inputClass}
                value={toAddress}
                onChange={(e) => setToAddress(e.target.value)}
                placeholder="0x..."
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                  Amount
                </label>
                <input
                  className={inputClass}
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                  Currency
                </label>
                <input
                  className={inputClass}
                  value={currency}
                  onChange={(e) => setCurrency(e.target.value)}
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <label className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground">
                Chain ID
              </label>
              <input
                className={inputClass}
                value={chainId}
                onChange={(e) => setChainId(e.target.value)}
                placeholder="8453"
              />
            </div>
            <Button
              onClick={handleCheckPolicy}
              disabled={checkPolicyMutation.isPending || !userId}
              variant="outline"
              className="w-full border-[#00D4FF]/30 text-[#00D4FF] hover:bg-[#00D4FF]/10"
            >
              {checkPolicyMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              <ShieldCheck className="mr-2 h-4 w-4" />
              Check Policy
            </Button>
            {lastPolicyDecision && (
              <div className="rounded-lg border border-white/[0.08] bg-white/[0.02] p-3 space-y-1">
                <p className="text-xs font-semibold text-muted-foreground">Last Policy Result</p>
                <p className="text-sm font-mono text-foreground">{lastPolicyDecision}</p>
              </div>
            )}
          </div>
        </Panel>
      </div>
    </main>
  );
}
