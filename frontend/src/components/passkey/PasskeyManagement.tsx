"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { passkeyApi, PasskeyApiError, type PasskeyInfo } from "@/lib/passkey-api";
import { PasskeyRegister } from "./PasskeyRegister";

type ManagementState = "idle" | "loading" | "error";

export interface PasskeyManagementProps {
  userName?: string;
}

export function PasskeyManagement({ userName }: PasskeyManagementProps) {
  const [passkeys, setPasskeys] = React.useState<PasskeyInfo[]>([]);
  const [state, setState] = React.useState<ManagementState>("loading");
  const [errorMessage, setErrorMessage] = React.useState<string | null>(null);
  const [showRegister, setShowRegister] = React.useState(false);
  const [renamingId, setRenamingId] = React.useState<string | null>(null);
  const [renameValue, setRenameValue] = React.useState("");
  const [deletingId, setDeletingId] = React.useState<string | null>(null);
  const [actionLoading, setActionLoading] = React.useState(false);

  const loadPasskeys = React.useCallback(async () => {
    setState("loading");
    setErrorMessage(null);
    try {
      const list = await passkeyApi.listPasskeys();
      setPasskeys(list);
      setState("idle");
    } catch (err) {
      const message =
        err instanceof PasskeyApiError
          ? err.message
          : "Khong the tai danh sach passkey.";
      setErrorMessage(message);
      setState("error");
    }
  }, []);

  React.useEffect(() => {
    loadPasskeys();
  }, [loadPasskeys]);

  const handleRename = async (credentialId: string) => {
    if (!renameValue.trim()) return;
    setActionLoading(true);
    try {
      const updated = await passkeyApi.renamePasskey(
        credentialId,
        renameValue.trim(),
      );
      setPasskeys((prev) =>
        prev.map((p) => (p.id === credentialId ? updated : p)),
      );
      setRenamingId(null);
      setRenameValue("");
    } catch (err) {
      const message =
        err instanceof PasskeyApiError
          ? err.message
          : "Doi ten passkey that bai.";
      setErrorMessage(message);
    } finally {
      setActionLoading(false);
    }
  };

  const handleDelete = async (credentialId: string) => {
    setActionLoading(true);
    try {
      await passkeyApi.deletePasskey(credentialId);
      setPasskeys((prev) => prev.filter((p) => p.id !== credentialId));
      setDeletingId(null);
    } catch (err) {
      const message =
        err instanceof PasskeyApiError
          ? err.message
          : "Xoa passkey that bai.";
      setErrorMessage(message);
    } finally {
      setActionLoading(false);
    }
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleDateString("vi-VN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return dateStr;
    }
  };

  if (showRegister) {
    return (
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-medium">Them passkey moi</h3>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setShowRegister(false)}
          >
            Quay lai
          </Button>
        </div>
        <PasskeyRegister
          userName={userName}
          onSuccess={() => {
            setShowRegister(false);
            loadPasskeys();
          }}
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-medium">Quan ly Passkey</h3>
        <Button size="sm" onClick={() => setShowRegister(true)}>
          Them passkey
        </Button>
      </div>

      {state === "error" && errorMessage && (
        <p className="text-sm text-destructive">{errorMessage}</p>
      )}

      {state === "loading" && (
        <p className="text-sm text-muted-foreground">Dang tai...</p>
      )}

      {state === "idle" && passkeys.length === 0 && (
        <p className="text-sm text-muted-foreground">
          Chua co passkey nao duoc dang ky.
        </p>
      )}

      {passkeys.map((passkey) => (
        <div
          key={passkey.id}
          className="flex flex-col gap-2 rounded-lg border p-4"
        >
          <div className="flex items-center justify-between">
            {renamingId === passkey.id ? (
              <div className="flex items-center gap-2">
                <Input
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  placeholder="Ten moi"
                  className="h-8 w-48"
                  disabled={actionLoading}
                />
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => handleRename(passkey.id)}
                  disabled={actionLoading || !renameValue.trim()}
                >
                  Luu
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    setRenamingId(null);
                    setRenameValue("");
                  }}
                  disabled={actionLoading}
                >
                  Huy
                </Button>
              </div>
            ) : (
              <span className="font-medium">{passkey.name}</span>
            )}

            {renamingId !== passkey.id && deletingId !== passkey.id && (
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    setRenamingId(passkey.id);
                    setRenameValue(passkey.name);
                  }}
                >
                  Doi ten
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="text-destructive"
                  onClick={() => setDeletingId(passkey.id)}
                >
                  Xoa
                </Button>
              </div>
            )}
          </div>

          <div className="flex gap-4 text-xs text-muted-foreground">
            <span>Tao: {formatDate(passkey.createdAt)}</span>
            {passkey.lastUsedAt && (
              <span>Lan cuoi: {formatDate(passkey.lastUsedAt)}</span>
            )}
          </div>

          {deletingId === passkey.id && (
            <div className="flex items-center gap-2 rounded bg-destructive/10 p-2">
              <span className="text-sm">Xac nhan xoa passkey nay?</span>
              <Button
                size="sm"
                variant="destructive"
                onClick={() => handleDelete(passkey.id)}
                disabled={actionLoading}
              >
                Xoa
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setDeletingId(null)}
                disabled={actionLoading}
              >
                Huy
              </Button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
