# RampOS Enterprise Documentation

Welcome to the RampOS Enterprise documentation. This guide covers deployment, configuration, and operations for enterprise customers.

---

## Quick Navigation

| Document | Description | Audience |
|----------|-------------|----------|
| [Deployment Guide](./deployment.md) | Installation (K8s, Docker, bare metal) | DevOps Engineers |
| [Configuration](./configuration.md) | All config options and environment variables | DevOps Engineers |
| [SSO Setup](./sso-setup.md) | SAML/OIDC integration guide | Identity Engineers |
| [Operations](./operations.md) | Monitoring, backup, disaster recovery | SRE Teams |
| [API Limits](./api-limits.md) | Rate limits and quota management | Developers, DevOps |

---

## Enterprise Overview

RampOS Enterprise provides a complete fiat on/off-ramp solution with enterprise-grade features:

### Key Features

| Feature | Starter | Professional | Enterprise |
|---------|---------|--------------|------------|
| Transaction Volume | 1B VND/month | 50B VND/month | Unlimited |
| API Rate Limit | 100 req/min | 1,000 req/min | Custom |
| Tenants | 1 | 10 | Unlimited |
| Support SLA | Business hours | 24/5 | 24/7 |
| Custom Domain | No | Yes | Yes |
| SSO Integration | No | OIDC only | SAML + OIDC |
| Dedicated Infrastructure | No | No | Yes |
| On-premise Deployment | No | No | Yes |
| Custom Branding | No | Yes | Yes |
| Audit Logs Retention | 30 days | 1 year | Custom |

### Pricing Tiers

| Tier | Monthly Price | Included Volume | Overage Rate |
|------|---------------|-----------------|--------------|
| **Starter** | $500 | 1B VND | 0.1% |
| **Professional** | $2,500 | 50B VND | 0.08% |
| **Enterprise** | Custom | Unlimited | Negotiated |

---

## Architecture

### Standard Deployment

```
┌─────────────────────────────────────────────────────────────────┐
│                        Load Balancer                            │
│                    (NGINX / AWS ALB / GCP LB)                   │
└─────────────────────────────────┬───────────────────────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
           ┌────────▼────────┐        ┌─────────▼────────┐
           │   RampOS API    │        │   RampOS API     │
           │   (Replica 1)   │        │   (Replica N)    │
           └────────┬────────┘        └─────────┬────────┘
                    │                           │
                    └─────────────┬─────────────┘
                                  │
         ┌────────────────────────┼────────────────────────┐
         │                        │                        │
┌────────▼────────┐     ┌─────────▼─────────┐    ┌────────▼────────┐
│   PostgreSQL    │     │      Redis        │    │      NATS       │
│   (Primary +    │     │   (Sentinel/      │    │   JetStream     │
│    Replicas)    │     │    Cluster)       │    │   (Cluster)     │
└─────────────────┘     └───────────────────┘    └─────────────────┘
```

### Enterprise Deployment (Dedicated)

```
┌─────────────────────────────────────────────────────────────────┐
│                     Customer VPC / Network                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│   │   WAF/CDN    │───▶│   Ingress    │───▶│   API GW     │     │
│   └──────────────┘    └──────────────┘    └──────┬───────┘     │
│                                                   │              │
│                    ┌──────────────────────────────┘              │
│                    │                                             │
│   ┌────────────────▼────────────────┐                           │
│   │         RampOS Cluster          │                           │
│   │  ┌─────┐ ┌─────┐ ┌─────┐       │                           │
│   │  │ API │ │ API │ │ API │       │                           │
│   │  └─────┘ └─────┘ └─────┘       │                           │
│   │  ┌─────────────────────────┐    │                           │
│   │  │    Background Workers   │    │                           │
│   │  └─────────────────────────┘    │                           │
│   └─────────────────────────────────┘                           │
│                    │                                             │
│   ┌────────────────┴────────────────┐                           │
│   │          Data Layer             │                           │
│   │  ┌─────┐ ┌─────┐ ┌─────┐       │                           │
│   │  │ PG  │ │Redis│ │NATS │       │                           │
│   │  │ HA  │ │ HA  │ │ HA  │       │                           │
│   │  └─────┘ └─────┘ └─────┘       │                           │
│   └─────────────────────────────────┘                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Getting Started

### Step 1: Choose Deployment Model

| Model | Best For | Setup Time |
|-------|----------|------------|
| **SaaS (Managed)** | Quick start, minimal ops | < 1 hour |
| **Kubernetes** | Cloud-native teams | 2-4 hours |
| **Docker Compose** | Small deployments | 1-2 hours |
| **Bare Metal** | On-premise requirements | 1-2 days |

### Step 2: Prerequisites

**All Deployments:**
- PostgreSQL 16+
- Redis 7+
- TLS certificates
- DNS configuration

**Kubernetes:**
- Kubernetes 1.28+
- kubectl and kustomize
- Ingress controller
- cert-manager (recommended)

**Docker:**
- Docker 24+
- Docker Compose v2
- 16GB RAM minimum

### Step 3: Configuration

1. Review [Configuration Guide](./configuration.md) for all options
2. Set up SSO per [SSO Setup Guide](./sso-setup.md)
3. Configure rate limits per [API Limits Guide](./api-limits.md)

### Step 4: Deploy

Follow the [Deployment Guide](./deployment.md) for your chosen model.

### Step 5: Verify

```bash
# Health check
curl https://your-domain.com/health

# Readiness check
curl https://your-domain.com/ready

# Version check
curl https://your-domain.com/version
```

---

## Support

### Support Channels

| Tier | Email | Chat | Phone | Response SLA |
|------|-------|------|-------|--------------|
| Starter | Yes | No | No | 48 hours |
| Professional | Yes | Yes | No | 24 hours |
| Enterprise | Yes | Yes | Yes | 4 hours (P1) |

### Contact Information

- **Enterprise Support**: enterprise-support@ramp.vn
- **Security Issues**: security@ramp.vn
- **Billing**: billing@ramp.vn
- **Sales**: sales@ramp.vn

### Priority Levels

| Priority | Description | Response Target | Resolution Target |
|----------|-------------|-----------------|-------------------|
| P1 - Critical | System down, data loss | 15 min | 4 hours |
| P2 - High | Major feature broken | 1 hour | 8 hours |
| P3 - Medium | Feature degraded | 4 hours | 48 hours |
| P4 - Low | Questions, minor issues | 24 hours | Best effort |

---

## Compliance & Security

### Certifications

- SOC 2 Type II (in progress)
- ISO 27001 (planned)
- PCI DSS Level 1 (for card processing)

### Data Residency

| Region | Data Center | Availability |
|--------|-------------|--------------|
| Vietnam | Viettel IDC (Hanoi, HCMC) | Available |
| Singapore | AWS ap-southeast-1 | Available |
| Hong Kong | AWS ap-east-1 | Available |
| Custom | On-premise | Enterprise only |

### Security Features

- End-to-end encryption (TLS 1.3)
- At-rest encryption (AES-256)
- Audit logging with hash chain
- IP allowlisting
- API key rotation
- SSO with MFA enforcement

---

## Service Level Agreement

### Availability Targets

| Tier | Monthly Uptime | Credit |
|------|----------------|--------|
| Starter | 99.5% | 10% at < 99.0% |
| Professional | 99.9% | 25% at < 99.5% |
| Enterprise | 99.95% | Custom |

### Maintenance Windows

- **Scheduled**: Sundays 02:00-06:00 UTC (announced 7 days ahead)
- **Emergency**: As needed with best-effort notice
- **Zero-downtime** updates for Enterprise tier

---

## Related Documentation

- [API Reference](../api/README.md)
- [Security Guide](../security/README.md)
- [Compliance Architecture](../architecture/compliance.md)
- [Licensing Guide](../licensing/README.md)

---

**Version**: 1.0.0
**Last Updated**: 2026-02-06
