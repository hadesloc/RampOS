"use client";

import { OfframpForm } from "@/components/offramp/OfframpForm";
import { OfframpStatus } from "@/components/offramp/OfframpStatus";
import { OfframpHistory } from "@/components/offramp/OfframpHistory";
import { useOfframp } from "@/hooks/use-offramp";
import { ArrowDownLeft } from "lucide-react";

export default function OfframpPage() {
  const {
    selectedCurrency,
    setSelectedCurrency,
    currentQuote,
    currentIntent,
    createQuote,
    createOfframp,
    isCreatingQuote,
    isCreating,
  } = useOfframp();

  return (
    <div className="min-h-screen bg-[#09090B]">
      <div className="container mx-auto max-w-4xl py-8 px-4 space-y-8">
        {/* Page header */}
        <div className="flex items-start gap-4">
          <div className="h-10 w-10 rounded-xl bg-[#111113] border border-[#00FF87]/20 flex items-center justify-center shadow-[0_0_16px_rgba(0,255,135,0.12)] shrink-0">
            <ArrowDownLeft className="h-5 w-5 text-[#00FF87]" />
          </div>
          <div>
            <h1 className="text-xl font-bold tracking-tight text-white">
              Off-Ramp
            </h1>
            <p className="text-sm text-white/40 mt-0.5">
              Convert your crypto to VND and withdraw to your bank account
            </p>
          </div>
        </div>

        {/* Main grid */}
        <div className="grid gap-6 lg:grid-cols-2">
          <div className="rounded-xl border border-white/[0.06] bg-[#111113] overflow-hidden">
            <OfframpForm
              quote={currentQuote}
              onCreateQuote={createQuote}
              onCreateOfframp={createOfframp}
              isQuoting={isCreatingQuote}
              isSubmitting={isCreating}
              selectedCurrency={selectedCurrency}
              onCurrencyChange={setSelectedCurrency}
            />
          </div>

          <div className="rounded-xl border border-white/[0.06] bg-[#111113] overflow-hidden">
            <OfframpStatus
              intent={currentIntent.data}
              isLoading={currentIntent.isLoading}
            />
          </div>
        </div>

        {/* History */}
        <div className="rounded-xl border border-white/[0.06] bg-[#111113] overflow-hidden">
          <OfframpHistory
            intents={currentIntent.data ? [currentIntent.data] : []}
            total={currentIntent.data ? 1 : 0}
            totalPages={1}
            isLoading={false}
          />
        </div>
      </div>
    </div>
  );
}
