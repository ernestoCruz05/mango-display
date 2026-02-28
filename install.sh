#!/bin/bash
set -e

echo "Building and installing mdisplay via cargo..."
cargo install --path .

echo "Installing desktop entry to ~/.local/share/applications..."
mkdir -p ~/.local/share/applications
cp mdisplay.desktop ~/.local/share/applications/

if command -v update-desktop-database &> /dev/null; then
    update-desktop-database ~/.local/share/applications || true
fi

echo ""
echo "=============================================="
echo "Installation complete!"
echo "You can now run 'mdisplay' from your terminal,"
echo "or find it in your application launcher (like Rofi)."
echo "=============================================="
