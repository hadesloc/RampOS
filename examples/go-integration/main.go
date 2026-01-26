package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"time"

	"github.com/joho/godotenv"
	rampos "github.com/rampos/sdk-go" // Pseudo-import for example
)

func main() {
	_ = godotenv.Load()

	apiURL := os.Getenv("RAMPOS_API_URL")
	if apiURL == "" {
		apiURL = "http://localhost:3000"
	}
	tenantID := os.Getenv("RAMPOS_TENANT_ID")
	apiKey := os.Getenv("RAMPOS_API_KEY")
	apiSecret := os.Getenv("RAMPOS_API_SECRET")

	// 1. Initialize Client
	client := rampos.NewClient(rampos.Config{
		BaseURL:   apiURL,
		TenantID:  tenantID,
		APIKey:    apiKey,
		APISecret: apiSecret,
	})

	ctx := context.Background()

	fmt.Println("🚀 Starting RampOS Go Integration Example")

	// 2. Create Payin Intent
	fmt.Println("\nCreating Payin Intent...")
	payinReq := rampos.CreatePayinRequest{
		UserID:        "user_123",
		Amount:        1000000,
		Currency:      "VND",
		PaymentMethod: "BANK_TRANSFER",
		Metadata: map[string]interface{}{
			"order_id": "ORD-GO-001",
		},
	}

	payin, err := client.Intents.CreatePayin(ctx, payinReq)
	if err != nil {
		log.Fatalf("❌ Failed to create payin: %v", err)
	}
	fmt.Printf("✅ Payin Intent Created: %s\n", payin.ID)

	// 3. Poll for Status
	fmt.Println("\nPolling for status...")
	for i := 0; i < 3; i++ {
		intent, err := client.Intents.Get(ctx, payin.ID)
		if err != nil {
			log.Printf("Error getting intent: %v", err)
			continue
		}
		fmt.Printf("Status: %s\n", intent.Status)
		if intent.Status == "COMPLETED" {
			break
		}
		time.Sleep(1 * time.Second)
	}

	// 4. Check Balance
	fmt.Println("\nChecking User Balance...")
	balance, err := client.Ledger.GetBalance(ctx, "user_123", "VND")
	if err != nil {
		log.Printf("⚠️ Could not fetch balance (maybe mocking is needed): %v", err)
	} else {
		fmt.Printf("User Balance: %s VND\n", balance.Available)
	}

	// 5. Create Payout
	fmt.Println("\nCreating Payout Intent...")
	payoutReq := rampos.CreatePayoutRequest{
		UserID:   "user_123",
		Amount:   500000,
		Currency: "VND",
		BankAccount: rampos.BankAccount{
			BankCode:      "VCB",
			AccountNumber: "9988776655",
			AccountName:   "NGUYEN VAN B",
		},
	}
	payout, err := client.Intents.CreatePayout(ctx, payoutReq)
	if err != nil {
		log.Printf("❌ Failed to create payout: %v", err)
	} else {
		fmt.Printf("✅ Payout Intent Created: %s\n", payout.ID)
	}
}
