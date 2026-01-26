# Monitoring Dashboards Handoff

## Deliverables
- **Prometheus Config**: `monitoring/prometheus/prometheus.yml` configured to scrape `ramp-api`, `ramp-worker`, and `ramp-bundler`.
- **Grafana Provisioning**: `monitoring/grafana/provisioning/dashboards.yaml` configured to load dashboards from the file system.
- **Dashboards**:
    - `monitoring/grafana/dashboards/overview.json`: High-level system metrics (Intents, API Rate, Errors, Tenants).
    - `monitoring/grafana/dashboards/api-performance.json`: Latency, endpoints, rate limits, auth failures.
    - `monitoring/grafana/dashboards/compliance.json`: KYC/AML metrics, case counts, risk scores.
    - `monitoring/grafana/dashboards/ledger.json`: Transaction volumes, balance drift, entry creation.
    - `monitoring/grafana/dashboards/aa-operations.json`: Account Abstraction metrics (UserOps, Gas, Accounts).

## Implementation Details
- Dashboards use Prometheus as the datasource.
- Panel JSONs are self-contained and ready for import or provisioning.
- Metrics assume standard Prometheus counters and histograms (e.g., `http_requests_total`, `ramp_intents_total`).

## Verification
- Files are placed in `monitoring/` directory.
- JSON syntax is valid.
- Prometheus config points to `host.docker.internal` for Docker-based setups.
