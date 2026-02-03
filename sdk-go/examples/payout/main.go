package main

import (
    "context"
    "fmt"
    "log"

    rampos "github.com/rampos/sdk-go"
)

func main() {
    client := rampos.NewClient(
        "",
        "",
        rampos.WithAPIKey("your-api-key"),
        rampos.WithAPISecret("your-api-secret"),
        rampos.WithBaseURL("https://api.rampos.io"),
    )

    // Create payout intent
    intent, err := client.Payouts.Create(context.Background(), &rampos.CreatePayoutRequest{
        UserID:    "usr_123",
        AmountVND: 1000000,
        BankAccount: rampos.BankAccount{
            BankCode:      "970415", // VietinBank
            AccountNumber: "101000000000",
            AccountName:   "NGUYEN VAN A",
        },
    })
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Created payout intent: %s\n", intent.IntentID)
    fmt.Printf("Status: %s\n", intent.Status)
}
