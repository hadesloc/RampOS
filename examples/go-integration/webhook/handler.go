package main

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"

	"github.com/joho/godotenv"
	rampos "github.com/rampos/sdk-go"
)

func main() {
	_ = godotenv.Load()
	webhookSecret := os.Getenv("RAMPOS_WEBHOOK_SECRET")
	if webhookSecret == "" {
		webhookSecret = "default-secret"
	}

	http.HandleFunc("/webhook", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
			return
		}

		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "Bad request", http.StatusBadRequest)
			return
		}
		defer r.Body.Close()

		signature := r.Header.Get("X-RampOS-Signature")

		// Verify Signature
		if !rampos.VerifyWebhookSignature(body, signature, webhookSecret) {
			log.Println("❌ Invalid signature")
			http.Error(w, "Invalid signature", http.StatusUnauthorized)
			return
		}

		var event rampos.WebhookEvent
		if err := json.Unmarshal(body, &event); err != nil {
			log.Println("Error decoding JSON:", err)
			http.Error(w, "Invalid JSON", http.StatusBadRequest)
			return
		}

		fmt.Printf("\n🔔 Received Webhook: %s\n", event.Type)
		// Process event...
		if event.Type == "INTENT.UPDATED" {
			fmt.Printf("Intent %s status update\n", event.Payload["id"])
		}

		w.WriteHeader(http.StatusOK)
		w.Write([]byte("OK"))
	})

	port := ":3002"
	fmt.Printf("Webhook server listening on %s\n", port)
	log.Fatal(http.ListenAndServe(port, nil))
}
