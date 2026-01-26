module rampos-go-example

go 1.21

require (
	github.com/rampos/sdk-go v0.1.0 // This would be the real path
	github.com/joho/godotenv v1.5.1
)

replace github.com/rampos/sdk-go => ../../sdk/go // Assuming Go SDK structure
