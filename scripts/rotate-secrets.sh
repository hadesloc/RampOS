#!/bin/bash
# Script to generate new secrets
echo "POSTGRES_PASSWORD=$(openssl rand -hex 16)"
echo "RAMPOS_ADMIN_KEY=$(openssl rand -hex 24)"
echo "RAMPOS_ENCRYPTION_KEY=$(openssl rand -base64 32)"
