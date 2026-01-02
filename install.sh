#!/bin/bash

# WaybarConf Installer

set -e

echo "Building WaybarConf..."
cargo build --release

BIN_NAME="waybarconf"
INSTALL_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
DATA_DIR="$HOME/.local/share/waybarconf"

echo "Creating directories..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$APP_DIR"
mkdir -p "$ICON_DIR"
mkdir -p "$DATA_DIR"

echo "Installing binary..."
cp target/release/"$BIN_NAME" "$INSTALL_DIR/"

echo "Installing presets..."
cp -r presets "$DATA_DIR/"

echo "Creating desktop entry..."
cat <<EOF > "$APP_DIR/waybarconf.desktop"
[Desktop Entry]
Name=WaybarConf
Comment=Waybar Configuration Editor
Exec=$INSTALL_DIR/$BIN_NAME
Icon=waybarconf
Terminal=false
Type=Application
Categories=Settings;GTK;
EOF

# Note: We'd ideally copy an icon file here if we had a dedicated one.
# For now, we'll use a generic icon search or symbolic icon.

echo "Done! You can now run 'waybarconf' from your app launcher."
echo "Note: Make sure $INSTALL_DIR is in your PATH."
