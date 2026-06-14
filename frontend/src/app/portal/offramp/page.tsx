"use client";

import { OfframpForm } from "@/components/offramp/OfframpForm";
import { OfframpStatus } from "@/components/offramp/OfframpStatus";
import { OfframpHistory } from "@/components/offramp/OfframpHistory";
import { useOfframp } from "@/hooks/use-offramp";

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
    <div className="container mx-auto max-w-4xl py-8 space-y-8">
      <div>
        <h1 className="text-2xl font-bold">Off-Ramp</h1>
        <p className="text-muted-foreground">
          Convert your crypto to VND and withdraw to your bank account
        </p>
      </div>

      <div className="grid gap-8 lg:grid-cols-2">
        <OfframpForm
          quote={currentQuote}
          onCreateQuote={createQuote}
          onCreateOfframp={createOfframp}
          isQuoting={isCreatingQuote}
          isSubmitting={isCreating}
          selectedCurrency={selectedCurrency}
          onCurrencyChange={setSelectedCurrency}
        />

        <OfframpStatus
          intent={currentIntent.data}
          isLoading={currentIntent.isLoading}
        />
      </div>

      <OfframpHistory
        intents={currentIntent.data ? [currentIntent.data] : []}
        total={currentIntent.data ? 1 : 0}
        totalPages={1}
        isLoading={false}
      />
    </div>
  );
}
