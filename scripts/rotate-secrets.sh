#!/bin/bash
# Script to generate new secrets
DB_PASSWORD=$(openssl rand -hex 32)
echo "POSTGRES_PASSWORD=$DB_PASSWORD"
echo "DATABASE_PASSWORD=$DB_PASSWORD"
echo "RAMPOS_ADMIN_KEY=$(openssl rand -hex 24)"
echo "RAMPOS_ENCRYPTION_KEY=$(openssl rand -base64 32)"
