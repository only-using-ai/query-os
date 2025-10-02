#!/bin/bash

# Q Release and Install Script
# This script builds the q binary in release mode, runs tests, and installs it

set -e  # Exit on any error

echo "🚀 Starting Q Release Process"
echo "================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the query-os directory."
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not installed. Please install Rust first."
    exit 1
fi

print_status "Running tests..."
if cargo test --release; then
    print_success "All tests passed!"
else
    print_error "Tests failed. Aborting release."
    exit 1
fi

print_status "Building in release mode..."
if cargo build --release; then
    print_success "Release build completed!"
else
    print_error "Release build failed."
    exit 1
fi

print_status "Installing q binary..."
if cargo install --path . --force; then
    print_success "Q installed successfully!"
else
    print_error "Installation failed."
    exit 1
fi

# Verify installation
print_status "Verifying installation..."
CARGO_BIN_DIR=$(cargo install --list | grep -A 1 "Installed package.*query-os" | tail -1 | awk '{print $2}' | xargs dirname 2>/dev/null || echo "")
Q_BINARY_PATH=""

# Try to find the installed binary
if command -v q &> /dev/null; then
    Q_BINARY_PATH=$(which q)
elif [ -n "$CARGO_BIN_DIR" ] && [ -f "$CARGO_BIN_DIR/q" ]; then
    Q_BINARY_PATH="$CARGO_BIN_DIR/q"
elif [ -f "$HOME/.cargo/bin/q" ]; then
    Q_BINARY_PATH="$HOME/.cargo/bin/q"
fi

if [ -n "$Q_BINARY_PATH" ]; then
    print_success "Q is installed at: $Q_BINARY_PATH"

    # Test basic functionality
    print_status "Testing basic functionality..."
    if "$Q_BINARY_PATH" "SELECT name FROM . LIMIT 1" &> /dev/null; then
        print_success "Basic functionality test passed!"
    else
        print_warning "Basic functionality test failed, but installation completed."
    fi

    # Check if q is in PATH
    if ! command -v q &> /dev/null; then
        print_warning "Q is not in your PATH."
        print_status "Add this to your shell profile:"
        print_status "  export PATH=\"\$HOME/.cargo/bin:\$PATH\""
        print_status "Or run Q directly: $Q_BINARY_PATH"
    fi
else
    print_error "Q installation verification failed."
    print_status "You can try running: cargo install --path . --force"
    exit 1
fi

echo ""
echo "================================"
print_success "Q Release Complete! 🎉"
echo ""
echo "Usage examples:"
echo "  q \"SELECT * FROM .\""
echo "  q \"SELECT * FROM ps LIMIT 5\""
echo "  q \"SELECT name, type FROM /tmp WHERE type = 'file'\""
echo ""
print_status "Happy querying! 📊"
