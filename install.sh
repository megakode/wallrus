#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.megakode.Wallrus"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PREFIX="${PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
DATADIR="$PREFIX/share"
APPDIR="$DATADIR/applications"
ICONDIR="$DATADIR/icons/hicolor/scalable/apps"
METADIR="$DATADIR/metainfo"
PALETTEDIR="$DATADIR/wallrus/palettes"

# Build release binary
echo "Building wallrus (release)..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

# Install binary
echo "Installing binary to $BINDIR..."
install -Dm755 "$SCRIPT_DIR/target/release/wallrus" "$BINDIR/wallrus"

# Install desktop file
echo "Installing desktop file to $APPDIR..."
install -Dm644 "$SCRIPT_DIR/data/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"

# Install icon
echo "Installing icon to $ICONDIR..."
install -Dm644 "$SCRIPT_DIR/data/icons/$APP_ID.svg" "$ICONDIR/$APP_ID.svg"

# Install metainfo
echo "Installing metainfo to $METADIR..."
install -Dm644 "$SCRIPT_DIR/data/$APP_ID.metainfo.xml" "$METADIR/$APP_ID.metainfo.xml"

# Install bundled palettes
if [ -d "$SCRIPT_DIR/data/palettes" ]; then
    echo "Installing palettes to $PALETTEDIR..."
    mkdir -p "$PALETTEDIR"
    cp -r "$SCRIPT_DIR/data/palettes/"* "$PALETTEDIR/"
fi

# Update icon cache if possible
if command -v gtk4-update-icon-cache &>/dev/null; then
    echo "Updating icon cache..."
    gtk4-update-icon-cache "$DATADIR/icons/hicolor/" 2>/dev/null || true
elif command -v gtk-update-icon-cache &>/dev/null; then
    gtk-update-icon-cache "$DATADIR/icons/hicolor/" 2>/dev/null || true
fi

echo ""
echo "Wallrus installed successfully."
echo "  Binary:   $BINDIR/wallrus"
echo "  Desktop:  $APPDIR/$APP_ID.desktop"
echo "  Icon:     $ICONDIR/$APP_ID.svg"
echo "  Palettes: $PALETTEDIR/"
echo ""
echo "You may need to log out and back in for the icon to appear."
