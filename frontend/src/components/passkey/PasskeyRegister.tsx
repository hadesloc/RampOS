"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export interface PasskeyRegisterProps {
  rpId?: string;
  rpName?: string;
  userId?: string;
  userName?: string;
  onSuccess?: (credential: PublicKeyCredential) => void;
  onError?: (error: Error) => void;
}

type RegisterState = "idle" | "loading" | "success" | "error";

export function PasskeyRegister({
  rpId = window.location.hostname,
  rpName = "RampOS",
  userId,
  userName,
  onSuccess,
  onError,
}: PasskeyRegisterProps) {
  const [state, setState] = React.useState<RegisterState>("idle");
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);
  const [displayName, setDisplayName] = React.useState(userName ?? "");

  const handleRegister = React.useCallback(async () => {
    if (!window.PublicKeyCredential) {
      const err = new Error("WebAuthn is not supported in this browser");
      setErrorMessage(err.message);
      setState("error");
      onError?.(err);
      return;
    }

    if (!displayName.trim()) {
      const err = new Error("Please enter a display name");
      setErrorMessage(err.message);
      setState("error");
      onError?.(err);
      return;
    }

    setState("loading");
    setErrorMessage(null);

    try {
      const challenge = crypto.getRandomValues(new Uint8Array(32));
      const userIdBytes = userId
        ? new TextEncoder().encode(userId)
        : crypto.getRandomValues(new Uint8Array(16));

      const credential = (await navigator.credentials.create({
        publicKey: {
          challenge,
          rp: { name: rpName, id: rpId },
          user: {
            id: userIdBytes,
            name: displayName.trim(),
            displayName: displayName.trim(),
          },
          pubKeyCredParams: [
            { alg: -7, type: "public-key" },   // ES256
            { alg: -257, type: "public-key" },  // RS256
          ],
          authenticatorSelection: {
            authenticatorAttachment: "platform",
            userVerification: "preferred",
            residentKey: "preferred",
          },
          timeout: 60000,
          attestation: "none",
        },
      })) as PublicKeyCredential | null;

      if (!credential) {
        throw new Error("Registration was cancelled");
      }

      setState("success");
      onSuccess?.(credential);
    } catch (err) {
      const error =
        err instanceof Error ? err : new Error("Passkey registration failed");
      setErrorMessage(error.message);
      setState("error");
      onError?.(error);
    }
  }, [rpId, rpName, userId, displayName, onSuccess, onError]);

  return (
    <div className="flex flex-col gap-4">
      {!userName && (
        <div className="flex flex-col gap-2">
          <Label htmlFor="passkey-display-name">Display Name</Label>
          <Input
            id="passkey-display-name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="Enter your name"
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
        {state === "success" ? "Passkey Registered" : "Register Passkey"}
      </Button>
      {state === "error" && errorMessage && (
        <p className="text-sm text-destructive">{errorMessage}</p>
      )}
      {state === "success" && (
        <p className="text-sm text-green-600">
          Passkey registered successfully. You can now use it to sign in.
        </p>
      )}
    </div>
  );
}
