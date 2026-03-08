#!/usr/bin/env bash
# embed_python.sh — Bundle Python 3.12 + SF platform into SimpleSF/Resources/
# Run once before building: ./Scripts/embed_python.sh
# Requires: curl, unzip, internet access

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
RESOURCES="$REPO_ROOT/SimpleSF/Resources"
PYTHON_DIR="$RESOURCES/Python.framework"
SITE_PACKAGES="$RESOURCES/site-packages"
PLATFORM_DIR="$RESOURCES/platform"
SF_SOURCE="$HOME/_MACARON-SOFTWARE/platform"  # adjust if needed

PYTHON_VERSION="3.12.9"
# python-build-standalone releases: https://github.com/indygreg/python-build-standalone
PYTHON_RELEASE="20250311"
PYTHON_URL="https://github.com/indygreg/python-build-standalone/releases/download/${PYTHON_RELEASE}/cpython-${PYTHON_VERSION}+${PYTHON_RELEASE}-aarch64-apple-darwin-install_only.tar.gz"

echo "==> Simple SF — Python Embed Script"
echo "    Resources: $RESOURCES"
mkdir -p "$RESOURCES"

# ── 1. Download Python standalone ──────────────────────────────────────────
if [ ! -f "$PYTHON_DIR/Versions/3.12/bin/python3" ]; then
  echo "==> Downloading Python $PYTHON_VERSION standalone..."
  TMPTAR="$RESOURCES/.python_standalone.tar.gz"
  curl -L --progress-bar "$PYTHON_URL" -o "$TMPTAR"
  echo "==> Extracting Python..."
  mkdir -p "$PYTHON_DIR"
  tar -xzf "$TMPTAR" -C "$PYTHON_DIR" --strip-components=0
  rm "$TMPTAR"
  echo "    Python extracted: $(ls $PYTHON_DIR)"
else
  echo "==> Python already embedded, skipping download"
fi

PYTHON3="$PYTHON_DIR/python/bin/python3"
# python-build-standalone unpacks as python/bin/python3
if [ ! -f "$PYTHON3" ]; then
  PYTHON3="$(find "$PYTHON_DIR" -name "python3.12" -type f | head -1)"
fi
echo "    Python binary: $PYTHON3"

# ── 2. Install pip dependencies ────────────────────────────────────────────
echo "==> Installing SF Python dependencies..."
mkdir -p "$SITE_PACKAGES"
"$PYTHON3" -m pip install \
  --quiet \
  --target "$SITE_PACKAGES" \
  --no-deps \
  fastapi uvicorn[standard] httpx aiofiles python-multipart \
  pydantic pydantic-settings \
  markdown jinja2 \
  python-jose[cryptography] passlib[bcrypt] \
  2>&1 | tail -5
echo "    Dependencies installed in $SITE_PACKAGES"

# ── 3. Copy SF platform code ───────────────────────────────────────────────
echo "==> Copying SF platform from $SF_SOURCE..."
if [ ! -d "$SF_SOURCE" ]; then
  echo "ERROR: SF source not found at $SF_SOURCE"
  echo "       Set SF_SOURCE env var or edit this script."
  exit 1
fi
rm -rf "$PLATFORM_DIR"
rsync -a --quiet \
  --exclude='__pycache__' \
  --exclude='*.pyc' \
  --exclude='data/' \
  --exclude='.git' \
  --exclude='tests/' \
  --exclude='node_modules/' \
  "$SF_SOURCE/" "$PLATFORM_DIR/"
echo "    Platform copied: $(du -sh $PLATFORM_DIR | cut -f1)"

# ── 4. Create data directory stub ─────────────────────────────────────────
mkdir -p "$RESOURCES/data"
echo "    Data directory: $RESOURCES/data"

# ── 5. Summary ──────────────────────────────────────────────────────────────
echo ""
echo "✓ Embed complete!"
echo "  Python:        $(du -sh $PYTHON_DIR | cut -f1)"
echo "  site-packages: $(du -sh $SITE_PACKAGES | cut -f1)"
echo "  platform:      $(du -sh $PLATFORM_DIR | cut -f1)"
echo ""
echo "Now build SimpleSF in Xcode."
