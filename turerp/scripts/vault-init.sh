#!/bin/sh
# Initialize Vault with development secrets for Turerp ERP
# WARNING: This script is for local development only. Never use in production.

set -e

VAULT_ADDR="${VAULT_ADDR:-http://127.0.0.1:8200}"
VAULT_TOKEN="${VAULT_TOKEN:-myroot}"

export VAULT_ADDR
export VAULT_TOKEN

# Wait for Vault to be ready
echo "Waiting for Vault to be ready..."
until vault status > /dev/null 2>&1; do
    sleep 1
done

echo "Vault is ready. Seeding development secrets..."

# Enable KV v2 secrets engine at /secret if not already enabled
vault secrets enable -path=secret kv-v2 > /dev/null 2>&1 || true

# Generate a random 64-character hex string for JWT secret
JWT_SECRET=$(cat /dev/urandom | tr -dc 'a-f0-9' | head -c 64)

# Store JWT secret
vault kv put secret/turerp/jwt \
    secret="$JWT_SECRET"

# Store database URL
vault kv put secret/turerp/database \
    url="postgres://postgres:postgres@localhost:5432/turerp"

# Store Redis URL
vault kv put secret/turerp/redis \
    url="redis://localhost:6379"

echo "Vault secrets seeded successfully."
