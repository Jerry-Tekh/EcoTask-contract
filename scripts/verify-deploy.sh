#!/usr/bin/env bash
set -euo pipefail

NETWORK="${NETWORK:-testnet}"

: "${SOROBAN_SECRET_KEY:?SOROBAN_SECRET_KEY is required}"
: "${ECO_TOKEN_ID:?ECO_TOKEN_ID is required}"
: "${TASK_REGISTRY_ID:?TASK_REGISTRY_ID is required}"
: "${REWARD_ENGINE_ID:?REWARD_ENGINE_ID is required}"
: "${TEST_USER:?TEST_USER is required}"

echo "Verifying contracts on $NETWORK..."

soroban contract invoke \
  --id "$ECO_TOKEN_ID" \
  --network "$NETWORK" \
  --source "$SOROBAN_SECRET_KEY" \
  -- \
  name

soroban contract invoke \
  --id "$ECO_TOKEN_ID" \
  --network "$NETWORK" \
  --source "$SOROBAN_SECRET_KEY" \
  -- \
  symbol

soroban contract invoke \
  --id "$ECO_TOKEN_ID" \
  --network "$NETWORK" \
  --source "$SOROBAN_SECRET_KEY" \
  -- \
  decimal

soroban contract invoke \
  --id "$TASK_REGISTRY_ID" \
  --network "$NETWORK" \
  --source "$SOROBAN_SECRET_KEY" \
  -- \
  task_count

soroban contract invoke \
  --id "$REWARD_ENGINE_ID" \
  --network "$NETWORK" \
  --source "$SOROBAN_SECRET_KEY" \
  -- \
  get_verification \
  --task_id 0 \
  --user "$TEST_USER"

echo "All contracts verified on $NETWORK"
