#!/usr/bin/env bash
set -euo pipefail

NETWORK="${NETWORK:-testnet}"

: "${SPONSOR_KEY:?SPONSOR_KEY is required}"
: "${SPONSOR_ADDRESS:?SPONSOR_ADDRESS is required}"
: "${TASK_REGISTRY_ID:?TASK_REGISTRY_ID is required}"

echo "=== EcoTask Contract Integration Test ==="

LOCATION_HASH=$(printf '%s' "test-integration-$(date +%s)" | sha256sum | cut -d' ' -f1)

echo "Creating task..."
soroban contract invoke \
  --id "$TASK_REGISTRY_ID" \
  --network "$NETWORK" \
  --source "$SPONSOR_KEY" \
  -- \
  create_task \
  --creator "$SPONSOR_ADDRESS" \
  --task_type "{\"string\": \"TREE_PLANTING\"}" \
  --location_hash "$LOCATION_HASH" \
  --reward_amount 100 \
  --max_completions 10 \
  --expires_at 9999999999

echo "Task count:"
soroban contract invoke \
  --id "$TASK_REGISTRY_ID" \
  --network "$NETWORK" \
  --source "$SPONSOR_KEY" \
  -- \
  task_count

echo "=== Integration test complete ==="
