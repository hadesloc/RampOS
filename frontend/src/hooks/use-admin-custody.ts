import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { adminApiRequest } from "@/lib/sdk-client";

export interface CustodyKeyResponse {
  userId: string;
  publicKey: string;
  generation: number;
  shareCount: number;
  createdAt: string;
}

export interface CustodySignResponse {
  userId: string;
  signature: string;
  algorithm: string;
}

export interface CustodyPolicy {
  whitelistAddresses: string[];
  dailyLimit: string;
  requireMultiApprovalAbove: string;
  enabled: boolean;
  updatedAt: string;
}

export interface CustodyPolicyCheckResponse {
  decision: "allow" | "deny" | "require_approval";
  reason?: string;
}

export interface CustodyPolicyPayload {
  userId: string;
  whitelistAddresses: string[];
  dailyLimit: string;
  requireMultiApprovalAbove: string;
  enabled: boolean;
}

export interface CustodyPolicyCheckPayload {
  userId: string;
  toAddress: string;
  amount: string;
  currency: string;
  chainId?: string;
}

const custodyApi = {
  generateKey: async (userId: string): Promise<CustodyKeyResponse> => {
    return adminApiRequest<CustodyKeyResponse>("/v1/custody/keys/generate", {
      method: "POST",
      body: JSON.stringify({ userId }),
    });
  },

  signUserOperation: async (payload: {
    userId: string;
    userOperation: Record<string, unknown>;
  }): Promise<CustodySignResponse> => {
    return adminApiRequest<CustodySignResponse>("/v1/custody/sign", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },

  getPolicy: async (userId: string): Promise<CustodyPolicy> => {
    const params = new URLSearchParams({ userId });
    return adminApiRequest<CustodyPolicy>(`/v1/custody/policies?${params.toString()}`);
  },

  updatePolicy: async (payload: CustodyPolicyPayload): Promise<CustodyPolicy> => {
    return adminApiRequest<CustodyPolicy>("/v1/custody/policies", {
      method: "PUT",
      body: JSON.stringify(payload),
    });
  },

  checkPolicy: async (
    payload: CustodyPolicyCheckPayload,
  ): Promise<CustodyPolicyCheckResponse> => {
    return adminApiRequest<CustodyPolicyCheckResponse>("/v1/custody/policies/check", {
      method: "POST",
      body: JSON.stringify(payload),
    });
  },
};

export function useCustodyPolicy(userId: string) {
  return useQuery<CustodyPolicy>({
    queryKey: ["admin-custody-policy", userId],
    queryFn: () => custodyApi.getPolicy(userId),
    enabled: !!userId,
  });
}

export function useGenerateCustodyKey() {
  return useMutation({
    mutationFn: (userId: string) => custodyApi.generateKey(userId),
  });
}

export function useSignCustodyUserOperation() {
  return useMutation({
    mutationFn: (payload: { userId: string; userOperation: Record<string, unknown> }) =>
      custodyApi.signUserOperation(payload),
  });
}

export function useUpdateCustodyPolicy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (payload: CustodyPolicyPayload) => custodyApi.updatePolicy(payload),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: ["admin-custody-policy", variables.userId],
      });
    },
  });
}

export function useCheckCustodyPolicy() {
  return useMutation({
    mutationFn: (payload: CustodyPolicyCheckPayload) => custodyApi.checkPolicy(payload),
  });
}

export { custodyApi };
