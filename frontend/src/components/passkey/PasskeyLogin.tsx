"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import {
  isWebAuthnSupported,
  toAuthenticationOptions,
  startAuthenticationWithAbort,
  type SerializedCredential,
} from "@/lib/webauthn";
import { passkeyApi, PasskeyApiError } from "@/lib/passkey-api";

export interface PasskeyLoginProps {
  onSuccess?: (credential: SerializedCredential) => void;
  onError?: (error: Error) => void;
}

type LoginState = "idle" | "loading" | "success" | "error";

function getWebAuthnErrorMessage(err: unknown): string {
  if (err instanceof DOMException) {
    switch (err.name) {
      case "NotAllowedError":
        return "Xac thuc bi huy hoac het thoi gian cho.";
      case "SecurityError":
        return "Xac thuc that bai do loi bao mat (domain khong khop).";
      case "AbortError":
        return "Xac thuc da bi huy.";
      case "InvalidStateError":
        return "Khong tim thay passkey phu hop.";
      default:
        return `Loi WebAuthn: ${err.message}`;
    }
  }
  if (err instanceof PasskeyApiError) {
    return `Loi server: ${err.message}`;
  }
  if (err instanceof Error) {
    return err.message;
  }
  return "Xac thuc passkey that bai.";
}

export function PasskeyLogin({ onSuccess, onError }: PasskeyLoginProps) {
  const [state, setState] = React.useState<LoginState>("idle");
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);

  const handleLogin = React.useCallback(async () => {
    if (!isWebAuthnSupported()) {
      const err = new Error(
        "Trinh duyet khong ho tro WebAuthn. Vui long su dung trinh duyet khac.",
      );
      setErrorMessage(err.message);
      setState("error");
      onError?.(err);
      return;
    }

    setState("loading");
    setErrorMessage(null);

    try {
      // 1. Fetch challenge from backend
      const challenge = await passkeyApi.getAuthenticationChallenge();

      // 2. Convert to browser options
      const options = toAuthenticationOptions(challenge);

      // 3. Get credential from browser
      const serialized = await startAuthenticationWithAbort(options);

      // 4. Verify with backend
      await passkeyApi.verifyAuthentication(serialized);

      setState("success");
      onSuccess?.(serialized);
    } catch (err) {
      const message = getWebAuthnErrorMessage(err);
      setErrorMessage(message);
      setState("error");
      const error = err instanceof Error ? err : new Error(message);
      onError?.(error);
    }
  }, [onSuccess, onError]);

  return (
    <div className="flex flex-col gap-3">
      <Button
        onClick={handleLogin}
        isLoading={state === "loading"}
        disabled={state === "loading"}
        variant={state === "error" ? "destructive" : "default"}
        className="w-full"
      >
        {state === "loading"
          ? "Dang xac thuc..."
          : state === "success"
            ? "Da xac thuc"
            : "Dang nhap bang Passkey"}
      </Button>
      {state === "error" && errorMessage && (
        <p className="text-sm text-destructive">{errorMessage}</p>
      )}
    </div>
  );
}
