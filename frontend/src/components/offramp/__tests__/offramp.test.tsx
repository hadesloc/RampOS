import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { OfframpForm } from "../OfframpForm";
import { OfframpStatus } from "../OfframpStatus";
import { OfframpHistory } from "../OfframpHistory";
import type {
  OfframpIntent,
  OfframpQuoteResponse,
} from "@/hooks/use-offramp";

// Mock radix-ui select
vi.mock("@radix-ui/react-select", async () => {
  const React = await import("react");
  const Root = ({ children }: { children: React.ReactNode }) =>
    React.createElement("div", { "data-testid": "select-root" }, children);
  const Trigger = React.forwardRef<
    HTMLButtonElement,
    React.PropsWithChildren<{ className?: string; id?: string }>
  >(({ children, className, id, ...props }, ref) =>
    React.createElement("button", { ref, className, id, role: "combobox", ...props }, children)
  );
  Trigger.displayName = "Trigger";
  const Value = ({ placeholder }: { placeholder?: string }) =>
    React.createElement("span", null, placeholder || "");
  const Content = React.forwardRef<HTMLDivElement, React.PropsWithChildren<{ className?: string }>>(
    ({ children }, ref) => React.createElement("div", { ref }, children)
  );
  Content.displayName = "Content";
  const Item = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ value: string; className?: string }>
  >(({ children, value, ...props }, ref) =>
    React.createElement("div", { ref, role: "option", "data-value": value, ...props }, children)
  );
  Item.displayName = "Item";
  const ItemText = ({ children }: { children: React.ReactNode }) => React.createElement("span", null, children);
  const ItemIndicator = ({ children }: { children: React.ReactNode }) => React.createElement("span", null, children);
  const Icon = ({ children }: { children: React.ReactNode }) => React.createElement("span", null, children);
  const Portal = ({ children }: { children: React.ReactNode }) => React.createElement("div", null, children);
  const Viewport = React.forwardRef<HTMLDivElement, React.PropsWithChildren<{ className?: string }>>(
    ({ children }, ref) => React.createElement("div", { ref }, children)
  );
  Viewport.displayName = "Viewport";
  const Group = ({ children }: { children: React.ReactNode }) => React.createElement("div", null, children);
  const Label = React.forwardRef<HTMLDivElement, React.PropsWithChildren<{ className?: string }>>(
    ({ children }, ref) => React.createElement("div", { ref }, children)
  );
  Label.displayName = "Label";
  const Separator = React.forwardRef<HTMLDivElement, { className?: string }>((props, ref) =>
    React.createElement("div", { ref })
  );
  Separator.displayName = "Separator";
  const ScrollUpButton = React.forwardRef<HTMLDivElement, React.PropsWithChildren<{ className?: string }>>(
    ({ children }, ref) => React.createElement("div", { ref }, children)
  );
  ScrollUpButton.displayName = "ScrollUpButton";
  const ScrollDownButton = React.forwardRef<HTMLDivElement, React.PropsWithChildren<{ className?: string }>>(
    ({ children }, ref) => React.createElement("div", { ref }, children)
  );
  ScrollDownButton.displayName = "ScrollDownButton";

  return { Root, Trigger, Value, Content, Item, ItemText, ItemIndicator, Icon, Portal, Viewport, Group, Label, Separator, ScrollUpButton, ScrollDownButton };
});

const mockQuote: OfframpQuoteResponse = {
  quoteId: "quote-1",
  cryptoAsset: "USDT",
  cryptoAmount: "100",
  exchangeRate: "25000",
  grossVndAmount: "2500000",
  netVndAmount: "2475000",
  feeTotal: "25000",
  expiresAt: "2099-01-01T00:00:00Z",
};

const mockIntent: OfframpIntent = {
  id: "intent-1",
  state: "CRYPTO_RECEIVED",
  cryptoAmount: "100",
  cryptoAsset: "USDT",
  netVndAmount: "2475000",
  grossVndAmount: "2500000",
  exchangeRate: "25000",
  linkedRfqId: "rfq-linked-1",
  winningLpId: "lp-winning-1",
  matchedRate: "25050",
  settlementId: "settlement-1",
  createdAt: "2025-01-15T10:00:00Z",
  updatedAt: "2025-01-15T10:05:00Z",
};

const mockIntents: OfframpIntent[] = [
  mockIntent,
  {
    ...mockIntent,
    id: "intent-2",
    state: "COMPLETED",
    cryptoAmount: "50",
    netVndAmount: "1237500",
    completedAt: "2025-01-14T15:00:00Z",
    txHash: "0xabc123",
    bankReference: "REF-001",
  },
  {
    ...mockIntent,
    id: "intent-3",
    state: "FAILED",
    cryptoAmount: "200",
    netVndAmount: "4950000",
  },
];

describe("OfframpForm", () => {
  it("renders quote request fields", () => {
    render(<OfframpForm />);

    expect(screen.getByText("Off-Ramp")).toBeInTheDocument();
    expect(screen.getByLabelText(/Amount/i)).toBeInTheDocument();
    expect(screen.getByText("Crypto Asset")).toBeInTheDocument();
    expect(screen.getByLabelText(/Bank Code/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Account Number/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Account Name/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Request Quote/i })).toBeInTheDocument();
  });

  it("shows loading state", () => {
    const { container } = render(<OfframpForm isLoading={true} />);
    expect(container.querySelector(".animate-pulse")).toBeInTheDocument();
  });

  it("validates amount input", () => {
    render(<OfframpForm />);

    const amountInput = screen.getByLabelText(/Amount/i);
    fireEvent.change(amountInput, { target: { value: "0" } });

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("alert").textContent).toContain("greater than 0");
  });

  it("submits quote request with backend contract fields", () => {
    const onCreateQuote = vi.fn();
    render(<OfframpForm onCreateQuote={onCreateQuote} />);

    fireEvent.change(screen.getByLabelText(/Amount/i), { target: { value: "100" } });
    fireEvent.change(screen.getByLabelText(/Bank Code/i), { target: { value: "VCB" } });
    fireEvent.change(screen.getByLabelText(/Account Number/i), { target: { value: "1234567890" } });
    fireEvent.change(screen.getByLabelText(/Account Name/i), { target: { value: "NGUYEN VAN A" } });
    fireEvent.click(screen.getByRole("button", { name: /Request Quote/i }));

    expect(onCreateQuote).toHaveBeenCalledWith({
      cryptoAsset: "USDT",
      amount: "100",
      bankCode: "VCB",
      accountNumber: "1234567890",
      accountName: "NGUYEN VAN A",
    });
  });

  it("shows quote summary after quote response", () => {
    render(<OfframpForm quote={mockQuote} />);

    expect(screen.getByTestId("quote-summary")).toBeInTheDocument();
    expect(screen.getByTestId("exchange-rate").textContent).toContain("25.000");
    expect(screen.getByTestId("fee-breakdown").textContent).toContain("Total Fee");
    expect(screen.getByTestId("fee-breakdown").textContent).toContain("You Receive");
  });

  it("creates off-ramp from quote with chain ID", () => {
    const onCreateOfframp = vi.fn();
    render(<OfframpForm quote={mockQuote} onCreateOfframp={onCreateOfframp} />);

    fireEvent.change(screen.getByLabelText(/Chain ID/i), { target: { value: "56" } });
    fireEvent.click(screen.getByRole("button", { name: /Create Off-Ramp/i }));

    expect(onCreateOfframp).toHaveBeenCalledWith({ chainId: 56 });
  });

  it("shows submitting text when isSubmitting is true", () => {
    render(<OfframpForm quote={mockQuote} isSubmitting={true} />);

    expect(screen.getByText("Creating off-ramp...")).toBeInTheDocument();
  });
});

describe("OfframpStatus", () => {
  it("renders null when no intent", () => {
    const { container } = render(<OfframpStatus intent={null} />);
    expect(container.firstChild).toBeNull();
  });

  it("shows loading state", () => {
    const { container } = render(<OfframpStatus isLoading={true} />);
    expect(container.querySelector(".animate-pulse")).toBeInTheDocument();
  });

  it("shows correct status badge", () => {
    render(<OfframpStatus intent={mockIntent} />);

    expect(screen.getByText("Transaction Status")).toBeInTheDocument();
    expect(screen.getAllByText("Crypto Received").length).toBeGreaterThanOrEqual(1);
  });

  it("shows intent details and off-ramp linkage fields", () => {
    render(<OfframpStatus intent={mockIntent} />);

    const details = screen.getByTestId("intent-details");
    expect(details.textContent).toContain("100");
    expect(details.textContent).toContain("USDT");
    expect(details.textContent).toContain("rfq-linked-1");
    expect(details.textContent).toContain("lp-winning-1");
    expect(details.textContent).toContain("25050");
    expect(details.textContent).toContain("settlement-1");
  });

  it("shows progress steps", () => {
    render(<OfframpStatus intent={mockIntent} />);

    expect(screen.getByText("Quote Created")).toBeInTheDocument();
    expect(screen.getByText("Crypto Pending")).toBeInTheDocument();
    expect(screen.getAllByText("Crypto Received").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("VND Transferring")).toBeInTheDocument();
    expect(screen.getByText("Completed")).toBeInTheDocument();
  });

  it("shows completed intent with bank reference", () => {
    const completedIntent: OfframpIntent = {
      ...mockIntent,
      state: "COMPLETED",
      bankReference: "REF-12345",
      txHash: "0xdef456",
      completedAt: "2025-01-15T12:00:00Z",
    };
    render(<OfframpStatus intent={completedIntent} />);

    expect(screen.getByText("REF-12345")).toBeInTheDocument();
    expect(screen.getByText("0xdef456")).toBeInTheDocument();
  });

  it("shows FAILED status correctly", () => {
    render(<OfframpStatus intent={{ ...mockIntent, state: "FAILED" }} />);

    expect(screen.getByText("Failed")).toBeInTheDocument();
  });
});

describe("OfframpHistory", () => {
  it("renders table with correct headers", () => {
    render(<OfframpHistory intents={mockIntents} />);

    expect(screen.getByText("Transaction History")).toBeInTheDocument();
    expect(screen.getByText("Date")).toBeInTheDocument();
    expect(screen.getByText("Amount (Crypto)")).toBeInTheDocument();
    expect(screen.getByText("Amount (VND)")).toBeInTheDocument();
    expect(screen.getByText("Status")).toBeInTheDocument();
    expect(screen.getByText("Actions")).toBeInTheDocument();
  });

  it("renders transaction rows", () => {
    render(<OfframpHistory intents={mockIntents} />);

    expect(screen.getByText("CRYPTO_RECEIVED")).toBeInTheDocument();
    expect(screen.getByText("COMPLETED")).toBeInTheDocument();
    expect(screen.getByText("FAILED")).toBeInTheDocument();
  });

  it("shows empty state when no intents", () => {
    render(<OfframpHistory intents={[]} />);

    expect(screen.getByText("No transactions yet")).toBeInTheDocument();
  });

  it("shows loading state", () => {
    const { container } = render(<OfframpHistory isLoading={true} />);
    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("renders pagination when multiple pages", () => {
    render(<OfframpHistory intents={mockIntents} page={1} totalPages={3} total={30} />);

    expect(screen.getByText(/page 1 of 3/i)).toBeInTheDocument();
    expect(screen.getByLabelText("Previous page")).toBeDisabled();
    expect(screen.getByLabelText("Next page")).not.toBeDisabled();
  });

  it("calls onPageChange when clicking pagination", () => {
    const onPageChange = vi.fn();
    render(<OfframpHistory intents={mockIntents} page={2} totalPages={3} total={30} onPageChange={onPageChange} />);

    fireEvent.click(screen.getByLabelText("Next page"));
    expect(onPageChange).toHaveBeenCalledWith(3);

    fireEvent.click(screen.getByLabelText("Previous page"));
    expect(onPageChange).toHaveBeenCalledWith(1);
  });

  it("calls onSelect when clicking a row", () => {
    const onSelect = vi.fn();
    render(<OfframpHistory intents={mockIntents} onSelect={onSelect} />);

    const viewButtons = screen.getAllByText("View");
    fireEvent.click(viewButtons[0]);
    expect(onSelect).toHaveBeenCalledWith(mockIntents[0]);
  });

  it("does not show pagination when single page", () => {
    render(<OfframpHistory intents={mockIntents} page={1} totalPages={1} total={3} />);

    expect(screen.queryByLabelText("Previous page")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Next page")).not.toBeInTheDocument();
  });
});
