"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";

export interface PasskeyLoginProps {
  rpId?: string;
  onSuccess?: (credential: PublicKeyCredential) => void;
  onError?: (error: Error) => void;
}

type LoginState = "idle" | "loading" | "success" | "error";

export function PasskeyLogin({
  rpId = window.location.hostname,
  onSuccess,
  onError,
}: PasskeyLoginProps) {
  const [state, setState] = React.useState<LoginState>("idle");
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);

  const handleLogin = React.useCallback(async () => {
    if (!window.PublicKeyCredential) {
      const err = new Error("WebAuthn is not supported in this browser");
      setErrorMessage(err.message);
      setState("error");
      onError?.(err);
      return;
    }

    setState("loading");
    setErrorMessage(null);

    try {
      const challenge = crypto.getRandomValues(new Uint8Array(32));

      const credential = (await navigator.credentials.get({
        publicKey: {
          challenge,
          rpId,
          userVerification: "preferred",
          timeout: 60000,
        },
      })) as PublicKeyCredential | null;

      if (!credential) {
        throw new Error("Authentication was cancelled");
      }

      setState("success");
      onSuccess?.(credential);
    } catch (err) {
      const error =
        err instanceof Error ? err : new Error("Passkey authentication failed");
      setErrorMessage(error.message);
      setState("error");
      onError?.(error);
    }
  }, [rpId, onSuccess, onError]);

  return (
    <div className="flex flex-col gap-3">
      <Button
        onClick={handleLogin}
        isLoading={state === "loading"}
        disabled={state === "loading"}
        variant={state === "error" ? "destructive" : "default"}
        className="w-full"
      >
        {state === "success" ? "Authenticated" : "Sign in with Passkey"}
      </Button>
      {state === "error" && errorMessage && (
        <p className="text-sm text-destructive">{errorMessage}</p>
      )}
    </div>
  );
}
