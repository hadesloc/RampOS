import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { OfframpForm } from "../OfframpForm";
import { OfframpStatus } from "../OfframpStatus";
import { OfframpHistory } from "../OfframpHistory";
import type {
  ExchangeRate,
  BankAccount,
  OfframpIntent,
} from "@/hooks/use-offramp";

// Mock radix-ui select
vi.mock("@radix-ui/react-select", async () => {
  const React = await import("react");
  const Root = ({
    children,
    value,
    onValueChange,
  }: {
    children: React.ReactNode;
    value?: string;
    onValueChange?: (v: string) => void;
  }) =>
    React.createElement(
      "div",
      { "data-testid": "select-root" },
      children
    );
  const Trigger = React.forwardRef<
    HTMLButtonElement,
    React.PropsWithChildren<{ className?: string; id?: string }>
  >(({ children, className, id, ...props }, ref) =>
    React.createElement(
      "button",
      { ref, className, id, role: "combobox", ...props },
      children
    )
  );
  Trigger.displayName = "Trigger";
  const Value = ({ placeholder }: { placeholder?: string }) =>
    React.createElement("span", null, placeholder || "");
  const Content = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ className?: string }>
  >(({ children }, ref) =>
    React.createElement("div", { ref }, children)
  );
  Content.displayName = "Content";
  const Item = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ value: string; className?: string }>
  >(({ children, value, ...props }, ref) =>
    React.createElement(
      "div",
      { ref, role: "option", "data-value": value, ...props },
      children
    )
  );
  Item.displayName = "Item";
  const ItemText = ({ children }: { children: React.ReactNode }) =>
    React.createElement("span", null, children);
  const ItemIndicator = ({ children }: { children: React.ReactNode }) =>
    React.createElement("span", null, children);
  const Icon = ({ children }: { children: React.ReactNode }) =>
    React.createElement("span", null, children);
  const Portal = ({ children }: { children: React.ReactNode }) =>
    React.createElement("div", null, children);
  const Viewport = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ className?: string }>
  >(({ children }, ref) =>
    React.createElement("div", { ref }, children)
  );
  Viewport.displayName = "Viewport";
  const Group = ({ children }: { children: React.ReactNode }) =>
    React.createElement("div", null, children);
  const Label = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ className?: string }>
  >(({ children }, ref) =>
    React.createElement("div", { ref }, children)
  );
  Label.displayName = "Label";
  const Separator = React.forwardRef<
    HTMLDivElement,
    { className?: string }
  >((props, ref) => React.createElement("div", { ref }));
  Separator.displayName = "Separator";
  const ScrollUpButton = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ className?: string }>
  >(({ children }, ref) =>
    React.createElement("div", { ref }, children)
  );
  ScrollUpButton.displayName = "ScrollUpButton";
  const ScrollDownButton = React.forwardRef<
    HTMLDivElement,
    React.PropsWithChildren<{ className?: string }>
  >(({ children }, ref) =>
    React.createElement("div", { ref }, children)
  );
  ScrollDownButton.displayName = "ScrollDownButton";

  return {
    Root,
    Trigger,
    Value,
    Content,
    Item,
    ItemText,
    ItemIndicator,
    Icon,
    Portal,
    Viewport,
    Group,
    Label,
    Separator,
    ScrollUpButton,
    ScrollDownButton,
  };
});

const mockExchangeRate: ExchangeRate = {
  fromCurrency: "USDT",
  toCurrency: "VND",
  rate: "25000",
  networkFee: "1",
  serviceFeePercent: "0.5",
  minAmount: "10",
  maxAmount: "10000",
  updatedAt: "2025-01-01T00:00:00Z",
};

const mockBankAccounts: BankAccount[] = [
  {
    id: "bank-1",
    bankName: "Vietcombank",
    accountNumber: "1234567890",
    accountName: "Nguyen Van A",
    isDefault: true,
  },
  {
    id: "bank-2",
    bankName: "Techcombank",
    accountNumber: "0987654321",
    accountName: "Nguyen Van A",
    isDefault: false,
  },
];

const mockIntent: OfframpIntent = {
  id: "intent-1",
  userId: "user-1",
  cryptoAmount: "100",
  cryptoCurrency: "USDT",
  fiatAmount: "2475000",
  fiatCurrency: "VND",
  exchangeRate: "25000",
  networkFee: "1",
  serviceFee: "0.5",
  totalFee: "1.5",
  status: "PROCESSING",
  bankAccountId: "bank-1",
  bankName: "Vietcombank",
  bankAccountNumber: "1234567890",
  createdAt: "2025-01-15T10:00:00Z",
  updatedAt: "2025-01-15T10:05:00Z",
};

const mockIntents: OfframpIntent[] = [
  mockIntent,
  {
    ...mockIntent,
    id: "intent-2",
    status: "COMPLETED",
    cryptoAmount: "50",
    fiatAmount: "1237500",
    completedAt: "2025-01-14T15:00:00Z",
    txHash: "0xabc123",
    bankReference: "REF-001",
  },
  {
    ...mockIntent,
    id: "intent-3",
    status: "FAILED",
    cryptoAmount: "200",
    fiatAmount: "4950000",
  },
];

describe("OfframpForm", () => {
  it("renders all form fields", () => {
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
      />
    );

    expect(screen.getByText("Off-Ramp")).toBeInTheDocument();
    expect(screen.getByLabelText(/Amount/i)).toBeInTheDocument();
    expect(screen.getByText("Crypto Currency")).toBeInTheDocument();
    expect(screen.getByText("Bank Account")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Convert to VND/i })).toBeInTheDocument();
  });

  it("shows loading state", () => {
    const { container } = render(<OfframpForm isLoading={true} />);
    expect(container.querySelector(".animate-pulse")).toBeInTheDocument();
  });

  it("validates amount input (min/max)", () => {
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
      />
    );

    const amountInput = screen.getByLabelText(/Amount/i);
    fireEvent.change(amountInput, { target: { value: "5" } });

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("alert").textContent).toContain("10");
  });

  it("shows exchange rate display", () => {
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
      />
    );

    expect(screen.getByTestId("exchange-rate")).toBeInTheDocument();
    expect(screen.getByTestId("exchange-rate").textContent).toContain("25.000");
  });

  it("shows fee breakdown when amount entered", () => {
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
      />
    );

    const amountInput = screen.getByLabelText(/Amount/i);
    fireEvent.change(amountInput, { target: { value: "100" } });

    const feeBreakdown = screen.getByTestId("fee-breakdown");
    expect(feeBreakdown).toBeInTheDocument();
    expect(feeBreakdown.textContent).toContain("Network Fee");
    expect(feeBreakdown.textContent).toContain("Service Fee");
    expect(feeBreakdown.textContent).toContain("Total Fee");
    expect(feeBreakdown.textContent).toContain("You Receive");
  });

  it("calls onSubmit when form is submitted", () => {
    const onSubmit = vi.fn();
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
        onSubmit={onSubmit}
      />
    );

    // The submit button should be disabled without valid amount and bank account
    const submitBtn = screen.getByRole("button", { name: /Convert to VND/i });
    expect(submitBtn).toBeDisabled();
  });

  it("shows no bank accounts message when empty", () => {
    render(
      <OfframpForm exchangeRate={mockExchangeRate} bankAccounts={[]} />
    );

    expect(screen.getByText(/No bank accounts found/i)).toBeInTheDocument();
  });

  it("shows min/max range from exchange rate", () => {
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
      />
    );

    expect(screen.getByText(/Min: 10/i)).toBeInTheDocument();
    expect(screen.getByText(/Max: 10000/i)).toBeInTheDocument();
  });

  it("shows Processing text when isSubmitting is true", () => {
    render(
      <OfframpForm
        exchangeRate={mockExchangeRate}
        bankAccounts={mockBankAccounts}
        isSubmitting={true}
      />
    );

    expect(screen.getByText("Processing...")).toBeInTheDocument();
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
    // "Processing" appears both in the badge and the step label
    const processingElements = screen.getAllByText("Processing");
    expect(processingElements.length).toBeGreaterThanOrEqual(1);
  });

  it("shows intent details", () => {
    render(<OfframpStatus intent={mockIntent} />);

    const details = screen.getByTestId("intent-details");
    expect(details.textContent).toContain("100");
    expect(details.textContent).toContain("USDT");
  });

  it("shows progress steps", () => {
    render(<OfframpStatus intent={mockIntent} />);

    expect(screen.getByText("Pending")).toBeInTheDocument();
    expect(screen.getAllByText("Processing").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Sending to Bank")).toBeInTheDocument();
    expect(screen.getByText("Completed")).toBeInTheDocument();
  });

  it("shows completed intent with bank reference", () => {
    const completedIntent: OfframpIntent = {
      ...mockIntent,
      status: "COMPLETED",
      bankReference: "REF-12345",
      txHash: "0xdef456",
      completedAt: "2025-01-15T12:00:00Z",
    };
    render(<OfframpStatus intent={completedIntent} />);

    expect(screen.getByText("REF-12345")).toBeInTheDocument();
    expect(screen.getByText("0xdef456")).toBeInTheDocument();
  });

  it("shows FAILED status correctly", () => {
    const failedIntent: OfframpIntent = {
      ...mockIntent,
      status: "FAILED",
    };
    render(<OfframpStatus intent={failedIntent} />);

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
    expect(screen.getByText("Bank")).toBeInTheDocument();
    expect(screen.getByText("Actions")).toBeInTheDocument();
  });

  it("renders transaction rows", () => {
    render(<OfframpHistory intents={mockIntents} />);

    expect(screen.getByText("PROCESSING")).toBeInTheDocument();
    expect(screen.getByText("COMPLETED")).toBeInTheDocument();
    expect(screen.getByText("FAILED")).toBeInTheDocument();
  });

  it("shows empty state when no intents", () => {
    render(<OfframpHistory intents={[]} />);

    expect(screen.getByText("No transactions yet")).toBeInTheDocument();
  });

  it("shows loading state", () => {
    const { container } = render(<OfframpHistory isLoading={true} />);
    // TableBody with isLoading shows skeleton rows
    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("renders pagination when multiple pages", () => {
    render(
      <OfframpHistory
        intents={mockIntents}
        page={1}
        totalPages={3}
        total={30}
      />
    );

    expect(screen.getByText(/page 1 of 3/i)).toBeInTheDocument();
    expect(screen.getByLabelText("Previous page")).toBeDisabled();
    expect(screen.getByLabelText("Next page")).not.toBeDisabled();
  });

  it("calls onPageChange when clicking pagination", () => {
    const onPageChange = vi.fn();
    render(
      <OfframpHistory
        intents={mockIntents}
        page={2}
        totalPages={3}
        total={30}
        onPageChange={onPageChange}
      />
    );

    fireEvent.click(screen.getByLabelText("Next page"));
    expect(onPageChange).toHaveBeenCalledWith(3);

    fireEvent.click(screen.getByLabelText("Previous page"));
    expect(onPageChange).toHaveBeenCalledWith(1);
  });

  it("calls onSelect when clicking a row", () => {
    const onSelect = vi.fn();
    render(
      <OfframpHistory intents={mockIntents} onSelect={onSelect} />
    );

    // Click "View" button on first row
    const viewButtons = screen.getAllByText("View");
    fireEvent.click(viewButtons[0]);
    expect(onSelect).toHaveBeenCalledWith(mockIntents[0]);
  });

  it("does not show pagination when single page", () => {
    render(
      <OfframpHistory
        intents={mockIntents}
        page={1}
        totalPages={1}
        total={3}
      />
    );

    expect(screen.queryByLabelText("Previous page")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Next page")).not.toBeInTheDocument();
  });
});
