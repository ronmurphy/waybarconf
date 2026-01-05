#!/bin/bash

# dependencies.sh - Install dependencies for WaybarConf on Arch Linux
# Matches dependencies from modern-labwc setup script

echo "Checking for Arch Linux..."
if ! command -v pacman &> /dev/null; then
    echo "Error: 'pacman' not found. This script is intended for Arch Linux."
    exit 1
fi

echo "Updating package database..."
sudo pacman -Sy

echo "Installing dependencies..."
# Core build dependencies
# rust: compiler and cargo
# git: for cloning if needed
# base-devel: common build tools
# gtk4, libadwaita: GUI toolkit libraries
# waybar: the target application

# modern-labwc dependencies:
# imagemagick, labwc, wl-clipboard, cliphist, wl-clip-persist: core desktop/utils
# rofi: launcher
# ffmpegthumbnailer, ffmpeg: media
# dunst: notifications
# matugen: theming
# foot: terminal
# swww: wallpaper
# swayidle, hyprlock: idle/lock management
# qt5-wayland, qt6-wayland: Qt apps on Wayland
# nm-connection-editor: network GUI
# polkit-gnome, gnome-keyring: auth/secrets

# Fonts & Themes:
# otf-font-awesome, inter-font, ttf-roboto
# papirus-icon-theme, adw-gtk-theme

sudo pacman -S --needed --noconfirm \
    rust \
    git \
    base-devel \
    gtk4 \
    libadwaita \
    waybar \
    imagemagick \
    labwc \
    wl-clipboard \
    cliphist \
    wl-clip-persist \
    rofi \
    ffmpegthumbnailer \
    ffmpeg \
    dunst \
    matugen \
    foot \
    swww \
    swayidle \
    hyprlock \
    qt5-wayland \
    qt6-wayland \
    nm-connection-editor \
    polkit-gnome \
    gnome-keyring \
    otf-font-awesome \
    inter-font \
    ttf-roboto \
    papirus-icon-theme \
    adw-gtk-theme

echo "Dependencies installed successfully!"

# Clone modern-labwc for base configurations
echo "Cloning modern-labwc repository..."
LABWC_DIR="$HOME/modern-labwc"

if [ -d "$LABWC_DIR" ]; then
    echo "Directory $LABWC_DIR already exists. Skipping clone."
    echo "You may want to run 'git pull' inside it to update."
else
    git clone https://github.com/Harsh-bin/modern-labwc "$LABWC_DIR"
    echo "Cloned modern-labwc to $LABWC_DIR"
fi

# Sync configs to waybarconf presets
# This allows us to bundle the "native" modern-labwc configs with our app
echo "Syncing modern-labwc configs to ./presets/modern-labwc-base..."
mkdir -p "presets/modern-labwc-base"
cp -r "$LABWC_DIR/config/waybar/"* "presets/modern-labwc-base/"
echo "Configs synced."

echo "--------------------------------------------------------"
echo "NOTE: WaybarConf relies on configurations and assets from modern-labwc."
echo "If you are setting up a fresh environment, consider running the setup script:"
echo "  cd $LABWC_DIR && ./setup.sh"
echo "--------------------------------------------------------"
