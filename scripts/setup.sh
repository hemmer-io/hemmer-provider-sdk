#!/bin/bash
#
# Developer setup script for Hemmer Provider SDK
#
# This script sets up the development environment by:
#   1. Installing git hooks
#   2. Verifying Rust toolchain
#   3. Running initial build and tests
#
# Usage: ./scripts/setup.sh
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo -e "${BLUE}Hemmer Provider SDK - Developer Setup${NC}"
echo "========================================"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Step 1: Install git hooks
echo -e "${BLUE}[1/4]${NC} Installing git hooks..."
if [ -f "$PROJECT_ROOT/scripts/pre-commit" ]; then
    cp "$PROJECT_ROOT/scripts/pre-commit" "$PROJECT_ROOT/.git/hooks/pre-commit"
    chmod +x "$PROJECT_ROOT/.git/hooks/pre-commit"
    echo -e "${GREEN}Done${NC} Pre-commit hook installed"
else
    echo -e "${YELLOW}Warning${NC} Pre-commit script not found at scripts/pre-commit"
fi

# Step 2: Verify Rust toolchain
echo ""
echo -e "${BLUE}[2/4]${NC} Verifying Rust toolchain..."
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}Done${NC} $RUST_VERSION"
else
    echo -e "${RED}Error${NC} Rust is not installed. Please install from https://rustup.rs"
    exit 1
fi

# Check for required components
if command -v cargo &> /dev/null; then
    # Check for rustfmt
    if rustup component list --installed | grep -q rustfmt; then
        echo -e "${GREEN}Done${NC} rustfmt is installed"
    else
        echo -e "${YELLOW}Installing${NC} rustfmt..."
        rustup component add rustfmt
    fi

    # Check for clippy
    if rustup component list --installed | grep -q clippy; then
        echo -e "${GREEN}Done${NC} clippy is installed"
    else
        echo -e "${YELLOW}Installing${NC} clippy..."
        rustup component add clippy
    fi
fi

# Step 3: Build the project
echo ""
echo -e "${BLUE}[3/4]${NC} Building the project..."
cd "$PROJECT_ROOT"
if cargo build 2>&1; then
    echo -e "${GREEN}Done${NC} Build successful"
else
    echo -e "${RED}Error${NC} Build failed"
    exit 1
fi

# Step 4: Run tests
echo ""
echo -e "${BLUE}[4/4]${NC} Running tests..."
if cargo test --quiet 2>&1; then
    echo -e "${GREEN}Done${NC} All tests passed"
else
    echo -e "${RED}Error${NC} Tests failed"
    exit 1
fi

echo ""
echo "========================================"
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "You're ready to start developing. Useful commands:"
echo "  cargo build          - Build the project"
echo "  cargo test           - Run tests"
echo "  cargo clippy         - Run linter"
echo "  cargo fmt            - Format code"
echo "  cargo doc --open     - View documentation"
echo ""
