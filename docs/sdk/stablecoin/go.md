# Stablecoin Go SDK

This guide covers using the RampOS Go SDK for stablecoin operations including swaps, bridges, and balance management.

---

## Installation

```bash
go get github.com/rampos/sdk-go
```

## Requirements

- Go 1.21+

---

## Quick Start

### Initialize the Client

```go
package main

import (
    "context"
    "log"
    "os"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    client := rampos.NewClient(os.Getenv("RAMPOS_API_KEY"))

    // Optional: set base URL
    client.SetBaseURL("https://api.ramp.vn/v1")

    // Optional: set timeout
    client.SetTimeout(30 * time.Second)
}
```

---

## Stablecoin Operations

### List Supported Tokens

```go
package main

import (
    "context"
    "fmt"
    "log"

    rampos "github.com/rampos/sdk-go"
)

func listTokens(client *rampos.Client) {
    ctx := context.Background()

    // Get all tokens
    tokens, err := client.Stablecoin.GetTokens(ctx, nil)
    if err != nil {
        log.Fatal(err)
    }

    fmt.Println("Supported tokens:")
    for _, token := range tokens {
        fmt.Printf("%s - %s\n", token.Symbol, token.Name)
        fmt.Printf("  Price: $%s\n", token.PriceUSD)

        chains := make([]string, len(token.Chains))
        for i, c := range token.Chains {
            chains[i] = c.ChainName
        }
        fmt.Printf("  Chains: %v\n", chains)
    }
}

func listTokensByChain(client *rampos.Client, chain string) {
    ctx := context.Background()

    tokens, err := client.Stablecoin.GetTokens(ctx, &rampos.GetTokensParams{
        Chain: chain,
    })
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Tokens on %s:\n", chain)
    for _, token := range tokens {
        for _, c := range token.Chains {
            if c.ChainName == chain {
                fmt.Printf("%s: %s\n", token.Symbol, c.ContractAddress)
            }
        }
    }
}

func listUsdStablecoins(client *rampos.Client) {
    ctx := context.Background()

    tokens, err := client.Stablecoin.GetTokens(ctx, &rampos.GetTokensParams{
        Category: "usd",
    })
    if err != nil {
        log.Fatal(err)
    }

    fmt.Println("USD Stablecoins:")
    for _, token := range tokens {
        fmt.Printf("%s: $%s\n", token.Symbol, token.PriceUSD)
    }
}
```

### Get User Balances

```go
func getUserBalances(client *rampos.Client, userID string) (*rampos.BalancesResponse, error) {
    ctx := context.Background()

    response, err := client.Stablecoin.GetBalances(ctx, &rampos.GetBalancesParams{
        UserID: userID,
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Total value: $%s\n", response.TotalValueUSD)
    fmt.Println("Balances:")

    for _, balance := range response.Balances {
        fmt.Printf("%s (%s):\n", balance.Symbol, balance.Chain)
        fmt.Printf("  Balance: %s\n", balance.BalanceFormatted)
        fmt.Printf("  Locked: %s\n", balance.LockedBalance)
        fmt.Printf("  Value: $%s\n", balance.ValueUSD)
    }

    return response, nil
}

func getSpecificBalance(client *rampos.Client, userID, symbol, chain string) (*rampos.StablecoinBalance, error) {
    ctx := context.Background()

    response, err := client.Stablecoin.GetBalances(ctx, &rampos.GetBalancesParams{
        UserID: userID,
        Symbol: symbol,
        Chain:  chain,
    })
    if err != nil {
        return nil, err
    }

    if len(response.Balances) > 0 {
        balance := response.Balances[0]
        fmt.Printf("%s on %s: %s\n", symbol, chain, balance.BalanceFormatted)
        return &balance, nil
    }

    fmt.Println("No balance found")
    return nil, nil
}
```

---

## Swapping Tokens

### Get Swap Quote

```go
func getSwapQuote(
    client *rampos.Client,
    fromToken, toToken, chain, amount string,
) (*rampos.SwapQuote, error) {
    ctx := context.Background()

    quote, err := client.Stablecoin.GetSwapQuote(ctx, &rampos.SwapQuoteRequest{
        FromToken: fromToken,
        ToToken:   toToken,
        Chain:     chain,
        Amount:    amount,
    })
    if err != nil {
        return nil, err
    }

    fmt.Println("Swap Quote:")
    fmt.Printf("  From: %s %s\n", quote.FromToken.AmountFormatted, fromToken)
    fmt.Printf("  To: %s %s\n", quote.ToToken.ExpectedAmountFormatted, toToken)
    fmt.Printf("  Rate: %s\n", quote.ExchangeRate)
    fmt.Printf("  Fee: %s%%\n", quote.Fee.Percent)
    fmt.Printf("  Valid until: %s\n", quote.ValidUntil)

    return quote, nil
}
```

### Execute Swap

```go
func swapTokens(
    client *rampos.Client,
    userID, fromToken, toToken, chain, amount string,
) (*rampos.SwapResponse, error) {
    ctx := context.Background()

    // Get quote first
    quote, err := client.Stablecoin.GetSwapQuote(ctx, &rampos.SwapQuoteRequest{
        FromToken: fromToken,
        ToToken:   toToken,
        Chain:     chain,
        Amount:    amount,
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Swapping %s %s\n", quote.FromToken.AmountFormatted, fromToken)
    fmt.Printf("Expected: %s %s\n", quote.ToToken.ExpectedAmountFormatted, toToken)

    // Execute swap
    swap, err := client.Stablecoin.Swap(ctx, &rampos.SwapRequest{
        UserID:      userID,
        FromToken:   fromToken,
        ToToken:     toToken,
        Chain:       chain,
        Amount:      amount,
        SlippageBps: 50, // 0.5%
        ReferenceID: fmt.Sprintf("swap_%d", time.Now().Unix()),
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Swap initiated: %s\n", swap.SwapID)
    fmt.Printf("Status: %s\n", swap.Status)

    return swap, nil
}

func swapWithConfirmation(
    client *rampos.Client,
    userID, fromToken, toToken, chain, amount string,
) (*rampos.SwapResponse, error) {
    ctx := context.Background()

    // Execute swap
    swap, err := client.Stablecoin.Swap(ctx, &rampos.SwapRequest{
        UserID:      userID,
        FromToken:   fromToken,
        ToToken:     toToken,
        Chain:       chain,
        Amount:      amount,
        SlippageBps: 30,
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Swap ID: %s\n", swap.SwapID)

    // Poll for completion
    result := swap
    for result.Status == "PENDING" || result.Status == "EXECUTING" {
        time.Sleep(2 * time.Second)
        result, err = client.Stablecoin.GetSwap(ctx, swap.SwapID)
        if err != nil {
            return nil, err
        }
        fmt.Printf("Status: %s\n", result.Status)
    }

    if result.Status == "COMPLETED" {
        fmt.Println("Swap completed!")
        fmt.Printf("Received: %s %s\n", result.ToToken.AmountFormatted, toToken)
        fmt.Printf("Tx: %s\n", result.TransactionHash)
    } else {
        fmt.Printf("Swap failed: %s\n", result.Status)
    }

    return result, nil
}
```

### Swap USDT to USDC Example

```go
func swapUsdtToUsdc(client *rampos.Client, userID string, amountUsdt float64) (*rampos.SwapResponse, error) {
    ctx := context.Background()

    // Convert to base units (USDT has 6 decimals)
    amount := fmt.Sprintf("%.0f", amountUsdt*1_000_000)

    swap, err := client.Stablecoin.Swap(ctx, &rampos.SwapRequest{
        UserID:      userID,
        FromToken:   "USDT",
        ToToken:     "USDC",
        Chain:       "ethereum",
        Amount:      amount,
        SlippageBps: 50,
    })
    if err != nil {
        // Handle specific errors
        if apiErr, ok := err.(*rampos.APIError); ok {
            switch apiErr.Code {
            case "INSUFFICIENT_BALANCE":
                fmt.Println("Insufficient USDT balance")
            case "SLIPPAGE_EXCEEDED":
                fmt.Println("Price moved too much, try again")
            default:
                fmt.Printf("API error: %s\n", apiErr.Message)
            }
        }
        return nil, err
    }

    fmt.Printf("Swapping %.2f USDT to USDC\n", amountUsdt)
    fmt.Printf("Swap ID: %s\n", swap.SwapID)
    fmt.Printf("Expected USDC: %s\n", swap.ToToken.ExpectedAmountFormatted)

    return swap, nil
}
```

---

## Bridging Cross-Chain

### Get Bridge Quote

```go
func getBridgeQuote(
    client *rampos.Client,
    token, fromChain, toChain, amount string,
) (*rampos.BridgeQuote, error) {
    ctx := context.Background()

    quote, err := client.Stablecoin.GetBridgeQuote(ctx, &rampos.BridgeQuoteRequest{
        Token:     token,
        FromChain: fromChain,
        ToChain:   toChain,
        Amount:    amount,
    })
    if err != nil {
        return nil, err
    }

    fmt.Println("Bridge Quote:")
    fmt.Printf("  %s: %s → %s\n", token, fromChain, toChain)
    fmt.Printf("  Amount: %s\n", quote.AmountFormatted)
    fmt.Printf("  Expected: %s\n", quote.ExpectedReceivedFormatted)
    fmt.Printf("  Fee: %s\n", quote.Fee.TotalFeeFormatted)

    fmt.Println("Available Routes:")
    for _, route := range quote.Routes {
        fmt.Printf("  %s: %s, fee: %s\n", route.Bridge, route.EstimatedTime, route.Fee)
    }
    fmt.Printf("Recommended: %s\n", quote.RecommendedRoute)

    return quote, nil
}
```

### Execute Bridge

```go
func bridgeTokens(
    client *rampos.Client,
    userID, token, fromChain, toChain, amount string,
) (*rampos.BridgeResponse, error) {
    ctx := context.Background()

    bridge, err := client.Stablecoin.Bridge(ctx, &rampos.BridgeRequest{
        UserID:      userID,
        Token:       token,
        FromChain:   fromChain,
        ToChain:     toChain,
        Amount:      amount,
        ReferenceID: fmt.Sprintf("bridge_%d", time.Now().Unix()),
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Bridge initiated: %s\n", bridge.BridgeID)
    fmt.Printf("Status: %s\n", bridge.Status)
    fmt.Printf("Expected completion: %s\n", bridge.EstimatedCompletionAt)

    return bridge, nil
}

func bridgeWithTracking(
    client *rampos.Client,
    userID, token, fromChain, toChain, amount string,
) (*rampos.BridgeResponse, error) {
    ctx := context.Background()

    // Start bridge
    bridge, err := client.Stablecoin.Bridge(ctx, &rampos.BridgeRequest{
        UserID:    userID,
        Token:     token,
        FromChain: fromChain,
        ToChain:   toChain,
        Amount:    amount,
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Bridge ID: %s\n", bridge.BridgeID)
    fmt.Printf("Bridging %s: %s → %s\n", token, fromChain, toChain)

    // Track progress
    result := bridge
    terminalStatuses := map[string]bool{
        "COMPLETED": true,
        "FAILED":    true,
        "REFUNDED":  true,
    }

    for !terminalStatuses[result.Status] {
        time.Sleep(10 * time.Second)
        result, err = client.Stablecoin.GetBridge(ctx, bridge.BridgeID)
        if err != nil {
            return nil, err
        }

        fmt.Printf("Status: %s\n", result.Status)

        if result.Status == "SOURCE_CONFIRMED" {
            fmt.Printf("Source tx confirmed: %s\n", result.FromChain.TransactionHash)
        }
    }

    if result.Status == "COMPLETED" {
        fmt.Println("Bridge completed!")
        fmt.Printf("Received: %s %s\n", result.ToChain.AmountFormatted, token)
        fmt.Printf("Destination tx: %s\n", result.ToChain.TransactionHash)
    } else {
        fmt.Printf("Bridge failed: %s\n", result.Status)
    }

    return result, nil
}
```

### Bridge USDC from Ethereum to Arbitrum

```go
func bridgeUsdcToArbitrum(client *rampos.Client, userID string, amountUsdc float64) (*rampos.BridgeResponse, error) {
    ctx := context.Background()
    amount := fmt.Sprintf("%.0f", amountUsdc*1_000_000)

    // Get quote first
    quote, err := client.Stablecoin.GetBridgeQuote(ctx, &rampos.BridgeQuoteRequest{
        Token:     "USDC",
        FromChain: "ethereum",
        ToChain:   "arbitrum",
        Amount:    amount,
    })
    if err != nil {
        return nil, err
    }

    fmt.Printf("Bridge %.2f USDC: Ethereum → Arbitrum\n", amountUsdc)
    fmt.Printf("Fee: %s USDC\n", quote.Fee.TotalFeeFormatted)
    fmt.Printf("Expected time: %s\n", quote.Routes[0].EstimatedTime)

    // Execute bridge
    bridge, err := client.Stablecoin.Bridge(ctx, &rampos.BridgeRequest{
        UserID:    userID,
        Token:     "USDC",
        FromChain: "ethereum",
        ToChain:   "arbitrum",
        Amount:    amount,
    })
    if err != nil {
        return nil, err
    }

    return bridge, nil
}
```

---

## Getting Prices

```go
func getPrices(client *rampos.Client, symbols []string) ([]rampos.TokenPrice, error) {
    ctx := context.Background()

    var symbolsStr string
    if len(symbols) > 0 {
        symbolsStr = strings.Join(symbols, ",")
    }

    response, err := client.Stablecoin.GetPrices(ctx, &rampos.GetPricesParams{
        Symbols:  symbolsStr,
        Currency: "usd",
    })
    if err != nil {
        return nil, err
    }

    fmt.Println("Stablecoin Prices:")
    for _, price := range response.Prices {
        change, _ := strconv.ParseFloat(price.Change24hPercent, 64)
        arrow := "↑"
        if change < 0 {
            arrow = "↓"
        }
        fmt.Printf("%s: $%s %s%.2f%%\n", price.Symbol, price.PriceUSD, arrow, math.Abs(change))
    }

    return response.Prices, nil
}

func getVndPrices(client *rampos.Client) (*rampos.PricesResponse, error) {
    ctx := context.Background()

    response, err := client.Stablecoin.GetPrices(ctx, &rampos.GetPricesParams{
        Currency: "vnd",
    })
    if err != nil {
        return nil, err
    }

    fmt.Println("Stablecoin Prices (VND):")
    for _, price := range response.Prices {
        fmt.Printf("%s: ₫%s\n", price.Symbol, price.PriceVND)
    }
    fmt.Printf("\nUSD/VND Rate: %s\n", response.VNDUSDRate)

    return response, nil
}
```

---

## Complete Integration Example

```go
package main

import (
    "context"
    "fmt"
    "log"
    "os"
    "strings"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    client := rampos.NewClient(os.Getenv("RAMPOS_API_KEY"))
    ctx := context.Background()
    userID := "usr_demo123"

    fmt.Println("=== Stablecoin Operations Demo ===\n")

    // 1. List tokens
    fmt.Println("1. Listing available tokens...")
    tokens, err := client.Stablecoin.GetTokens(ctx, &rampos.GetTokensParams{
        Category: "usd",
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Found %d USD stablecoins\n\n", len(tokens))

    // 2. Get balances
    fmt.Println("2. Getting user balances...")
    balances, err := client.Stablecoin.GetBalances(ctx, &rampos.GetBalancesParams{
        UserID: userID,
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Total portfolio: $%s\n\n", balances.TotalValueUSD)

    // 3. Get prices
    fmt.Println("3. Getting prices...")
    prices, err := client.Stablecoin.GetPrices(ctx, &rampos.GetPricesParams{
        Symbols: "USDT,USDC,DAI",
    })
    if err != nil {
        log.Fatal(err)
    }
    for _, p := range prices.Prices {
        fmt.Printf("  %s: $%s\n", p.Symbol, p.PriceUSD)
    }
    fmt.Println()

    // 4. Get swap quote
    fmt.Println("4. Getting swap quote (1000 USDT → USDC)...")
    swapQuote, err := client.Stablecoin.GetSwapQuote(ctx, &rampos.SwapQuoteRequest{
        FromToken: "USDT",
        ToToken:   "USDC",
        Chain:     "ethereum",
        Amount:    "1000000000",
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("  Expected: %s USDC\n", swapQuote.ToToken.ExpectedAmountFormatted)
    fmt.Printf("  Rate: %s\n\n", swapQuote.ExchangeRate)

    // 5. Get bridge quote
    fmt.Println("5. Getting bridge quote (USDC: ETH → Arbitrum)...")
    bridgeQuote, err := client.Stablecoin.GetBridgeQuote(ctx, &rampos.BridgeQuoteRequest{
        Token:     "USDC",
        FromChain: "ethereum",
        ToChain:   "arbitrum",
        Amount:    "1000000000",
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("  Expected: %s USDC\n", bridgeQuote.ExpectedReceivedFormatted)
    fmt.Printf("  Time: ~%s\n\n", bridgeQuote.Routes[0].EstimatedTime)

    fmt.Println("=== Demo Complete ===")
}
```

---

## Error Handling

```go
func safeSwap(client *rampos.Client, userID string) (*rampos.SwapResponse, error) {
    ctx := context.Background()

    swap, err := client.Stablecoin.Swap(ctx, &rampos.SwapRequest{
        UserID:    userID,
        FromToken: "USDT",
        ToToken:   "USDC",
        Chain:     "ethereum",
        Amount:    "1000000000",
    })
    if err != nil {
        if apiErr, ok := err.(*rampos.APIError); ok {
            switch apiErr.Code {
            case "INSUFFICIENT_BALANCE":
                log.Println("Not enough balance for swap")
            case "SLIPPAGE_EXCEEDED":
                log.Println("Price moved too much, please retry")
            case "QUOTE_EXPIRED":
                log.Println("Quote expired, getting new quote...")
            case "RATE_LIMITED":
                log.Printf("Rate limited, retry after %ds", apiErr.RetryAfter)
            default:
                log.Printf("API error: %s", apiErr.Message)
            }
        } else {
            log.Printf("Unexpected error: %v", err)
        }
        return nil, err
    }

    return swap, nil
}
```

---

## Go Types

```go
// Key types from the SDK
type StablecoinToken struct {
    Symbol   string          `json:"symbol"`
    Name     string          `json:"name"`
    Decimals int             `json:"decimals"`
    Category string          `json:"category"`
    Chains   []ChainInfo     `json:"chains"`
    PriceUSD string          `json:"priceUsd"`
}

type SwapRequest struct {
    UserID      string `json:"userId"`
    FromToken   string `json:"fromToken"`
    ToToken     string `json:"toToken"`
    Chain       string `json:"chain"`
    Amount      string `json:"amount"`
    SlippageBps int    `json:"slippageBps,omitempty"`
    ReferenceID string `json:"referenceId,omitempty"`
}

type BridgeRequest struct {
    UserID             string `json:"userId"`
    Token              string `json:"token"`
    FromChain          string `json:"fromChain"`
    ToChain            string `json:"toChain"`
    Amount             string `json:"amount"`
    DestinationAddress string `json:"destinationAddress,omitempty"`
    ReferenceID        string `json:"referenceId,omitempty"`
}
```

---

## Next Steps

- Read the [API Reference](./api-reference.md) for complete endpoint documentation
- Check the [TypeScript SDK Guide](./typescript.md) for TypeScript examples
- Learn about [Webhooks](../../api/webhooks.md) for event handling

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
