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

    // Create payin intent
    intent, err := client.Payins.Create(context.Background(), &rampos.CreatePayinRequest{
        UserID:    "usr_123",
        AmountVND: 1000000,
    })
    if err != nil {
        log.Fatal(err)
    }

    fmt.Printf("Created payin intent: %s\n", intent.IntentID)
    fmt.Printf("Status: %s\n", intent.Status)
}
