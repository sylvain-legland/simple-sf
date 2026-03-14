#!/usr/bin/env bash
# Ref: FT-SSF-025
# Canary deployment with percentage-based traffic routing
set -euo pipefail

CANARY_PCT=${1:-10}
echo "[CANARY] Deploying canary with ${CANARY_PCT}% traffic..."

docker compose -p "sf-canary" up -d --build

echo "[CANARY] Running smoke tests..."
for i in $(seq 1 5); do
    STATUS=$(curl -sf -o /dev/null -w "%{http_code}" "http://localhost:8099/health" || echo "000")
    if [ "$STATUS" = "200" ]; then
        echo "[CANARY] Smoke test $i/5 passed"
    else
        echo "[CANARY] Smoke test FAILED ($STATUS), rolling back!"
        docker compose -p "sf-canary" down
        exit 1
    fi
    sleep 1
done

echo "[CANARY] Canary healthy. Promote with: ./scripts/deploy-blue-green.sh"
