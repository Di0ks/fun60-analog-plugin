#!/usr/bin/env bash
# Install the FUN60 Analog plugin for the Wooting Analog SDK.
#
# The SDK scans /usr/local/share/WootingAnalogPlugins/ and loads each
# SUBDIRECTORY it finds (then dlopens the library inside). Our plugin is a
# C-ABI library, which the SDK auto-detects (no _plugin_create symbol).
#
# Usage: ./install.sh [--uninstall]
# (root password will be prompted for installation)
set -euo pipefail

PLUGIN_DIR="/usr/local/share/WootingAnalogPlugins/fun60_analog_plugin"
SO_NAME="libfun60_analog_plugin.so"

cd "$(dirname "$0")"

if [[ "${1:-}" == "--uninstall" ]]; then
    sudo rm -rf "$PLUGIN_DIR"
    echo "Removed $PLUGIN_DIR"
    exit 0
fi

if [[ ! -f "target/release/$SO_NAME" ]]; then
    echo "Release build not found; building..."
    cargo build --release
fi

sudo mkdir -p "$PLUGIN_DIR"
sudo install -m 0644 "target/release/$SO_NAME" "$PLUGIN_DIR/$SO_NAME"
echo "Installed $PLUGIN_DIR/$SO_NAME"
echo
echo "Note: the SDK (libwooting_analog_sdk.so) must be re-initialised by any"
echo "application using it to pick up the new plugin (restart the app, or"
echo "call wooting_analog_uninitialise/initialise)."
