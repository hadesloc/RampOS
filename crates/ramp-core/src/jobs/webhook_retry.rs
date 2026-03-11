//! Webhook Retry Background Worker (F04.05)
//!
//! Periodically scans for pending webhook deliveries that are
//! ready for retry and processes them.

use ramp_common::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::service::webhook_delivery::WebhookDeliveryService;

/// Background worker that processes webhook retries every 30 seconds
pub struct WebhookRetryWorker {
    delivery_service: Arc<WebhookDeliveryService>,
    /// How often to poll for pending deliveries (default: 30s)
    poll_interval: Duration,
    /// Maximum deliveries to process per tick
    batch_size: usize,
}

impl WebhookRetryWorker {
    /// Create a new WebhookRetryWorker
    pub fn new(delivery_service: Arc<WebhookDeliveryService>) -> Self {
        Self {
            delivery_service,
            poll_interval: Duration::from_secs(30),
            batch_size: 50,
        }
    }

    /// Create with custom poll interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Create with custom batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Run the worker continuously (blocking)
    pub async fn run(&self) {
        info!(
            poll_interval_secs = self.poll_interval.as_secs(),
            batch_size = self.batch_size,
            "Starting webhook retry worker"
        );

        let mut interval = tokio::time::interval(self.poll_interval);

        loop {
            interval.tick().await;

            match self.process_pending().await {
                Ok(count) => {
                    if count > 0 {
                        info!(
                            processed = count,
                            "Webhook retry worker processed deliveries"
                        );
                    }
                }
                Err(e) => {
                    error!(error = %e, "Webhook retry worker failed to process pending deliveries");
                }
            }
        }
    }

    /// Process a single batch of pending deliveries
    ///
    /// Returns the number of deliveries processed.
    pub async fn process_pending(&self) -> Result<usize> {
        let pending = self.delivery_service.get_pending_deliveries()?;

        let to_process = pending.into_iter().take(self.batch_size);
        let mut processed = 0;

        for delivery in to_process {
            // In a real implementation, this would make an HTTP request
            // to the endpoint_url and handle the response.
            // Here we log that the delivery is ready for processing.

            info!(
                delivery_id = %delivery.id,
                event_id = %delivery.event_id,
                endpoint_url = %delivery.endpoint_url,
                attempt = delivery.attempts + 1,
                "Processing webhook delivery retry"
            );

            // NOTE: Actual HTTP delivery would happen here.
            // The caller (or an integration layer) is responsible for
            // calling mark_delivered() or schedule_retry() based on the result.

            processed += 1;
        }

        Ok(processed)
    }

    /// Get the current count of pending deliveries
    pub fn pending_count(&self) -> Result<usize> {
        let pending = self.delivery_service.get_pending_deliveries()?;
        Ok(pending.len())
    }
}
