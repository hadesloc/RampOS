"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  isWebAuthnSupported,
  toRegistrationOptions,
  startRegistrationWithAbort,
  type SerializedCredential,
} from "@/lib/webauthn";
import { passkeyApi, PasskeyApiError } from "@/lib/passkey-api";

export interface PasskeyRegisterProps {
  userName?: string;
  onSuccess?: (credential: SerializedCredential) => void;
  onError?: (error: Error) => void;
}

type RegisterState = "idle" | "loading" | "success" | "error";

function getWebAuthnErrorMessage(err: unknown): string {
  if (err instanceof DOMException) {
    switch (err.name) {
      case "NotAllowedError":
        return "Dang ky bi huy hoac het thoi gian cho.";
      case "SecurityError":
        return "Dang ky that bai do loi bao mat (domain khong khop).";
      case "AbortError":
        return "Dang ky da bi huy.";
      case "InvalidStateError":
        return "Passkey nay da duoc dang ky truoc do.";
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
  return "Dang ky passkey that bai.";
}

export function PasskeyRegister({
  userName,
  onSuccess,
  onError,
}: PasskeyRegisterProps) {
  const [state, setState] = React.useState<RegisterState>("idle");
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);
  const [displayName, setDisplayName] = React.useState(userName ?? "");

  const handleRegister = React.useCallback(async () => {
    if (!isWebAuthnSupported()) {
      const err = new Error(
        "Trinh duyet khong ho tro WebAuthn. Vui long su dung trinh duyet khac.",
      );
      setErrorMessage(err.message);
      setState("error");
      onError?.(err);
      return;
    }

    if (!displayName.trim()) {
      const err = new Error("Vui long nhap ten hien thi.");
      setErrorMessage(err.message);
      setState("error");
      onError?.(err);
      return;
    }

    setState("loading");
    setErrorMessage(null);

    try {
      // 1. Fetch registration challenge from backend
      const challenge = await passkeyApi.getRegistrationChallenge(
        displayName.trim(),
      );

      // 2. Convert to browser options
      const options = toRegistrationOptions(challenge);

      // 3. Create credential via browser
      const serialized = await startRegistrationWithAbort(options);

      // 4. Verify with backend
      await passkeyApi.verifyRegistration(serialized);

      setState("success");
      onSuccess?.(serialized);
    } catch (err) {
      const message = getWebAuthnErrorMessage(err);
      setErrorMessage(message);
      setState("error");
      const error = err instanceof Error ? err : new Error(message);
      onError?.(error);
    }
  }, [displayName, onSuccess, onError]);

  return (
    <div className="flex flex-col gap-4">
      {!userName && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="passkey-display-name">Ten hien thi</Label>
          <Input
            id="passkey-display-name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="Nhap ten cua ban"
            disabled={state === "loading"}
          />
        </div>
      )}
      <Button
        onClick={handleRegister}
        isLoading={state === "loading"}
        disabled={state === "loading"}
        variant={state === "error" ? "destructive" : "default"}
        className="w-full"
      >
        {state === "loading"
          ? "Dang dang ky..."
          : state === "success"
            ? "Da dang ky Passkey"
            : "Dang ky Passkey"}
      </Button>
      {state === "error" && errorMessage && (
        <p className="text-sm text-destructive">{errorMessage}</p>
      )}
      {state === "success" && (
        <p className="text-sm text-green-600">
          Dang ky passkey thanh cong. Ban co the su dung passkey de dang nhap.
        </p>
      )}
    </div>
  );
}
