# Network Policies & Traffic Flow

This document describes the network security policies implemented for the RampOS Kubernetes deployment.

## Overview

We follow a **Default Deny** security posture. All pod-to-pod communication is blocked by default unless explicitly allowed by a NetworkPolicy.

## Policies

### 1. Default Deny (`default-deny-all`)
- **Scope**: Namespace `rampos`
- **Effect**: Denies all Ingress and Egress traffic for all pods.

### 2. API Server (`rampos-server-policy`)
- **Ingress**:
  - Allowed from Ingress Controller (namespace labeled `kubernetes.io/metadata.name: ingress-nginx` or `name: ingress-nginx`) on port `8080`.
- **Egress**:
  - To `rampos-postgres` on port `5432`
  - To `rampos-redis` on port `6379`
  - To `rampos-nats` on port `4222`
  - To DNS (CoreDNS in `kube-system`) on port `53` (UDP/TCP)
  - To External Internet (HTTPS port `443`) - restricted from private ranges (10.x, 172.16.x, 192.168.x).

### 3. PostgreSQL (`rampos-postgres-policy`)
- **Ingress**:
  - Allowed from `rampos-server` on port `5432`.
  - Allowed from `rampos-migration` job on port `5432`.
- **Egress**: None allowed.

### 4. Redis (`rampos-redis-policy`)
- **Ingress**:
  - Allowed from `rampos-server` on port `6379`.
- **Egress**: None allowed.

### 5. NATS (`rampos-nats-policy`)
- **Ingress**:
  - Allowed from `rampos-server` on port `4222`.
  - Allowed from `rampos-nats` peers on port `6222` (Clustering).
- **Egress**:
  - To `rampos-nats` peers on port `6222` (Clustering).
  - To DNS (CoreDNS in `kube-system`) on port `53` (UDP/TCP).

## Troubleshooting

If services cannot communicate:
1. Check labels on pods: `kubectl get pods --show-labels`.
2. Ensure Ingress Controller namespace has the correct label (`kubernetes.io/metadata.name: ingress-nginx`).
3. Check for `NetworkPolicy` events or use a CNI with logging (like Cilium) to debug drops.
