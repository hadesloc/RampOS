import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@/test/test-utils";
import userEvent from "@testing-library/user-event";
import { ChainSelector } from "../ChainSelector";
import { TokenSelector } from "../TokenSelector";
import { IntentPreview } from "../IntentPreview";
import { IntentBuilder } from "../IntentBuilder";
import { type ChainInfo, type TokenInfo, type IntentRoute } from "@/hooks/use-intent-builder";

// Mock the sdk-client to prevent real API calls
vi.mock("@/lib/sdk-client", () => ({
  adminApiRequest: vi.fn(),
  ApiError: class extends Error {
    status: number;
    code: string;
    constructor(status: number, code: string, message: string) {
      super(message);
      this.status = status;
      this.code = code;
    }
  },
}));

const mockChains: ChainInfo[] = [
  { id: "ethereum", name: "Ethereum", icon: "ETH" },
  { id: "bsc", name: "BNB Smart Chain", icon: "BNB" },
  { id: "polygon", name: "Polygon", icon: "MATIC" },
];

const mockTokens: TokenInfo[] = [
  { address: "0x0000", symbol: "ETH", name: "Ether", decimals: 18 },
  { address: "0xA0b8", symbol: "USDC", name: "USD Coin", decimals: 6, balance: "1000.00" },
];

const mockRoute: IntentRoute = {
  steps: [
    {
      type: "swap",
      fromChain: "ethereum",
      toChain: "ethereum",
      fromToken: "ETH",
      toToken: "USDC",
      protocol: "Uniswap",
      estimatedTime: 30,
    },
    {
      type: "bridge",
      fromChain: "ethereum",
      toChain: "polygon",
      fromToken: "USDC",
      toToken: "USDC",
      protocol: "Stargate",
      estimatedTime: 120,
    },
  ],
  estimatedTotalTime: 150,
  estimatedFees: {
    gas: "0.005",
    protocol: "0.002",
    total: "0.007",
    currency: "ETH",
  },
};

// --- ChainSelector Tests ---
describe("ChainSelector", () => {
  it("renders with label", () => {
    render(
      <ChainSelector label="Source Chain" value="" chains={mockChains} onChange={vi.fn()} />
    );
    expect(screen.getByText("Source Chain")).toBeInTheDocument();
  });

  it("renders select trigger with placeholder", () => {
    render(
      <ChainSelector label="Source Chain" value="" chains={mockChains} onChange={vi.fn()} />
    );
    expect(screen.getByText("Select chain")).toBeInTheDocument();
  });

  it("renders as disabled when disabled prop is true", () => {
    render(
      <ChainSelector label="Source Chain" value="" chains={mockChains} onChange={vi.fn()} disabled />
    );
    const trigger = screen.getByTestId("chain-selector-source-chain");
    expect(trigger).toBeDisabled();
  });
});

// --- TokenSelector Tests ---
describe("TokenSelector", () => {
  it("renders with label", () => {
    render(
      <TokenSelector label="Source Token" value="" tokens={mockTokens} onChange={vi.fn()} />
    );
    expect(screen.getByText("Source Token")).toBeInTheDocument();
  });

  it("shows placeholder when no chain selected", () => {
    render(
      <TokenSelector label="Source Token" value="" tokens={[]} onChange={vi.fn()} />
    );
    expect(screen.getByText("Select chain first")).toBeInTheDocument();
  });

  it("shows token select placeholder when tokens available", () => {
    render(
      <TokenSelector label="Source Token" value="" tokens={mockTokens} onChange={vi.fn()} />
    );
    expect(screen.getByText("Select token")).toBeInTheDocument();
  });

  it("renders as disabled when tokens list is empty", () => {
    render(
      <TokenSelector label="Source Token" value="" tokens={[]} onChange={vi.fn()} />
    );
    const trigger = screen.getByTestId("token-selector-source-token");
    expect(trigger).toBeDisabled();
  });
});

// --- IntentPreview Tests ---
describe("IntentPreview", () => {
  it("renders empty state when no route", () => {
    render(<IntentPreview route={null} />);
    expect(screen.getByText("Route Preview")).toBeInTheDocument();
    expect(
      screen.getByText("Configure your intent to see the execution route.")
    ).toBeInTheDocument();
  });

  it("renders loading skeleton", () => {
    render(<IntentPreview route={null} isLoading />);
    expect(screen.getByText("Route Preview")).toBeInTheDocument();
    // Skeletons render as divs with animate-pulse
    const preview = screen.getByTestId("intent-preview");
    const skeletons = preview.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("renders route steps", () => {
    render(<IntentPreview route={mockRoute} />);
    const steps = screen.getAllByTestId("route-step");
    expect(steps).toHaveLength(2);
  });

  it("renders step types as badges", () => {
    render(<IntentPreview route={mockRoute} />);
    expect(screen.getByText("swap")).toBeInTheDocument();
    expect(screen.getByText("bridge")).toBeInTheDocument();
  });

  it("renders estimated time", () => {
    render(<IntentPreview route={mockRoute} />);
    expect(screen.getByTestId("estimated-time")).toHaveTextContent("3 min");
  });

  it("renders total fees", () => {
    render(<IntentPreview route={mockRoute} />);
    expect(screen.getByTestId("total-fees")).toHaveTextContent("0.007 ETH");
  });

  it("renders protocol info for each step", () => {
    render(<IntentPreview route={mockRoute} />);
    expect(screen.getByText(/Uniswap/)).toBeInTheDocument();
    expect(screen.getByText(/Stargate/)).toBeInTheDocument();
  });
});

// --- IntentBuilder (full component) Tests ---
describe("IntentBuilder", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the main component", () => {
    render(<IntentBuilder />);
    expect(screen.getByTestId("intent-builder")).toBeInTheDocument();
    expect(screen.getByText("Cross-Chain Intent Builder")).toBeInTheDocument();
  });

  it("renders chain selectors", () => {
    render(<IntentBuilder />);
    expect(screen.getByText("Source Chain")).toBeInTheDocument();
    expect(screen.getByText("Destination Chain")).toBeInTheDocument();
  });

  it("renders token selectors", () => {
    render(<IntentBuilder />);
    expect(screen.getByText("Source Token")).toBeInTheDocument();
    expect(screen.getByText("Destination Token")).toBeInTheDocument();
  });

  it("renders amount input", () => {
    render(<IntentBuilder />);
    expect(screen.getByLabelText("Amount")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("0.00")).toBeInTheDocument();
  });

  it("renders action buttons", () => {
    render(<IntentBuilder />);
    expect(screen.getByRole("button", { name: /preview route/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /execute intent/i })).toBeInTheDocument();
  });

  it("has execute button disabled without amount", () => {
    render(<IntentBuilder />);
    const executeBtn = screen.getByRole("button", { name: /execute intent/i });
    expect(executeBtn).toBeDisabled();
  });

  it("has preview route button disabled initially", () => {
    render(<IntentBuilder />);
    const previewBtn = screen.getByRole("button", { name: /preview route/i });
    expect(previewBtn).toBeDisabled();
  });

  it("renders route preview section", () => {
    render(<IntentBuilder />);
    expect(screen.getByTestId("intent-preview")).toBeInTheDocument();
  });

  it("accepts amount input", async () => {
    const user = userEvent.setup();
    render(<IntentBuilder />);
    const input = screen.getByPlaceholderText("0.00");
    await user.type(input, "100");
    expect(input).toHaveValue(100);
  });

  it("renders source and destination sections", () => {
    render(<IntentBuilder />);
    expect(screen.getByText("Source")).toBeInTheDocument();
    expect(screen.getByText("Destination")).toBeInTheDocument();
  });
});
