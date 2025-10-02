#!/bin/bash

# Q Quick Install Script
# Installs the q binary without running tests or full release process

set -e  # Exit on any error

echo "📦 Installing Q (Filesystem Query Tool)"
echo "========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed. Please install Rust first."
    print_status "Visit: https://rustup.rs/"
    exit 1
fi

print_status "Building and installing Q..."
if cargo install --path . --force; then
    print_success "Q installed successfully!"
else
    print_error "Installation failed."
    exit 1
fi

# Verify installation
print_status "Verifying installation..."
Q_BINARY_PATH=""

# Try to find the installed binary
if command -v q &> /dev/null; then
    Q_BINARY_PATH=$(which q)
elif [ -f "$HOME/.cargo/bin/q" ]; then
    Q_BINARY_PATH="$HOME/.cargo/bin/q"
fi

if [ -n "$Q_BINARY_PATH" ]; then
    print_success "Q is installed at: $Q_BINARY_PATH"

    # Check if q is in PATH
    if ! command -v q &> /dev/null; then
        print_warning "Q is not in your PATH."
        print_status "Add this to your shell profile:"
        print_status "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        print_status "Or run Q directly: $Q_BINARY_PATH"
    fi

    echo ""
    echo "Usage examples:"
    echo "  q --query \"SELECT * FROM .\""
    echo "  q --query \"SELECT * FROM ps LIMIT 5\""
    echo "  q --query \"SELECT name, type FROM /tmp WHERE type = 'file'\""
    echo ""
    print_success "Installation complete! 🎉"
else
    print_error "Installation verification failed."
    print_status "You can try running: cargo install --path . --force"
    exit 1
fi
