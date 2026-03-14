#!/usr/bin/env bash
# Ref: FT-SSF-025
# Blue-Green deployment script
set -euo pipefail

ACTIVE_SLOT=$(cat /tmp/sf-active-slot 2>/dev/null || echo "blue")
if [ "$ACTIVE_SLOT" = "blue" ]; then
    DEPLOY_SLOT="green"
else
    DEPLOY_SLOT="blue"
fi

echo "[DEPLOY] Building $DEPLOY_SLOT slot..."
docker compose -p "sf-$DEPLOY_SLOT" up -d --build

echo "[DEPLOY] Health check on $DEPLOY_SLOT..."
for i in $(seq 1 10); do
    if curl -sf "http://localhost:8099/health" > /dev/null 2>&1; then
        echo "[DEPLOY] $DEPLOY_SLOT healthy!"
        echo "$DEPLOY_SLOT" > /tmp/sf-active-slot
        echo "[DEPLOY] Stopping old $ACTIVE_SLOT slot..."
        docker compose -p "sf-$ACTIVE_SLOT" down || true
        echo "[DEPLOY] Blue-Green deploy complete: $DEPLOY_SLOT is active"
        exit 0
    fi
    sleep 2
done

echo "[DEPLOY] FAILED: $DEPLOY_SLOT not healthy, rolling back"
docker compose -p "sf-$DEPLOY_SLOT" down || true
exit 1
