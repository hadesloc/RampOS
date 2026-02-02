"use client";

import { useState } from "react";

export default function SettingsPage() {
  const [apiKey, setApiKey] = useState("ramp_pk_*****************************");
  const [webhookUrl, setWebhookUrl] = useState("https://api.example.com/webhooks");
  const [rateLimit, setRateLimit] = useState("100");
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    setSaved(true);
    setTimeout(() => setSaved(false), 3000);
  };

  return (
    <div className="space-y-6 max-w-2xl">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Settings</h1>
        <p className="text-muted-foreground">
          Configure your RampOS tenant settings
        </p>
      </div>

      {saved && (
        <div className="bg-green-100 border border-green-400 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-400 px-4 py-3 rounded">
          Settings saved successfully!
        </div>
      )}

      {/* API Configuration */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">API Configuration</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">API Key</label>
          <div className="flex gap-2">
            <input
              type="text"
              className="flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono bg-muted"
              value={apiKey}
              readOnly
            />
            <button className="px-4 py-2 text-sm border rounded-md hover:bg-muted bg-background">
              Regenerate
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            Use this key to authenticate API requests. Keep it secret!
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Webhook Secret</label>
          <div className="flex gap-2">
            <input
              type="text"
              className="flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono bg-muted"
              value="whsec_*****************************"
              readOnly
            />
            <button className="px-4 py-2 text-sm border rounded-md hover:bg-muted bg-background">
              Regenerate
            </button>
          </div>
          <p className="text-xs text-muted-foreground">
            Use this secret to verify webhook signatures.
          </p>
        </div>
      </div>

      {/* Webhook Configuration */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">Webhook Configuration</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">Webhook URL</label>
          <input
            type="url"
            className="w-full rounded-md border bg-background px-3 py-2 text-sm"
            value={webhookUrl}
            onChange={(e) => setWebhookUrl(e.target.value)}
            placeholder="https://your-server.com/webhooks"
          />
          <p className="text-xs text-muted-foreground">
            We will send webhook events to this URL.
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Enabled Events</label>
          <div className="space-y-2">
            {[
              "intent.payin.created",
              "intent.payin.confirmed",
              "intent.payout.created",
              "intent.payout.completed",
              "intent.trade.executed",
              "case.created",
              "case.resolved",
            ].map((event) => (
              <label key={event} className="flex items-center gap-2">
                <input type="checkbox" defaultChecked className="rounded" />
                <span className="text-sm font-mono">{event}</span>
              </label>
            ))}
          </div>
        </div>
      </div>

      {/* Rate Limiting */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">Rate Limiting</h2>

        <div className="space-y-2">
          <label className="text-sm font-medium">Requests per minute</label>
          <input
            type="number"
            className="w-full rounded-md border bg-background px-3 py-2 text-sm"
            value={rateLimit}
            onChange={(e) => setRateLimit(e.target.value)}
            min="10"
            max="1000"
          />
          <p className="text-xs text-muted-foreground">
            Maximum API requests allowed per minute (10-1000).
          </p>
        </div>
      </div>

      {/* Transaction Limits */}
      <div className="rounded-lg border bg-card p-6 space-y-4">
        <h2 className="text-lg font-semibold">Default Transaction Limits</h2>

        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">Min Payin (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              defaultValue="10000"
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Max Payin (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              defaultValue="500000000"
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Min Payout (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              defaultValue="50000"
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Max Payout (VND)</label>
            <input
              type="number"
              className="w-full rounded-md border bg-background px-3 py-2 text-sm"
              defaultValue="200000000"
            />
          </div>
        </div>
      </div>

      {/* Save Button */}
      <div className="flex justify-end">
        <button
          onClick={handleSave}
          className="px-6 py-2 bg-primary text-primary-foreground rounded-md font-medium hover:bg-primary/90"
        >
          Save Settings
        </button>
      </div>
    </div>
  );
}
