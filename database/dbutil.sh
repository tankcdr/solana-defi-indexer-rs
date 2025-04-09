#!/bin/bash
# dbutil.sh - Wrapper for the database utility

# Exit on error
set -e

# Change to the project root directory
cd "$(dirname "$0")/.." || exit 1

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Print header
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Database Utility Tool${NC}"
echo -e "${GREEN}========================================${NC}"
echo

# Ensure .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}Warning: .env file not found.${NC}"
    echo "You will need to provide a database URL using the --database-url option"
    echo "or set the DATABASE_URL environment variable."
    
    # Prompt to continue
    read -p "Continue without .env file? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Display usage information if requested
if [[ "$1" == "--help" || "$1" == "-h" || $# -eq 0 ]]; then
    echo "Database Utility Tool"
    echo
    echo "Usage: $0 OPERATION DEX [OPTIONS]"
    echo
    echo "This script manages database schemas for different DEXes."
    echo
    echo "OPERATION:"
    echo "  create                Create schema for the specified DEX"
    echo "  delete                Delete schema for the specified DEX"
    echo
    echo "DEX:"
    echo "  common                Common schema only"
    echo "  orca                  Orca schema (includes common schema)"
    echo "  raydium               Raydium schema (includes common schema)"
    echo "  all                   All schemas (common, orca, raydium)"
    echo
    echo "OPTIONS:"
    echo "  --database-url URL    Override database URL from .env"
    echo "  --verbose, -v         Show verbose output"
    echo "  --yes                 Skip confirmation prompts"
    echo "  --help, -h            Show this help message"
    echo
    echo "Examples:"
    echo "  $0 create orca        Create schema for Orca"
    echo "  $0 delete raydium     Delete Raydium schema (with confirmation)"
    echo "  $0 create all --yes   Create all schemas without confirmation"
    echo
    exit 0
fi

# Warn if delete operation is used
if [[ "$1" == "delete" ]]; then
    dex=$2
    if [[ "$@" != *"--yes"* ]]; then
        echo -e "${RED}WARNING: You are about to DELETE database schema for $dex!${NC}"
        echo "This operation cannot be easily undone and will result in data loss."
        echo
        echo "If you're sure, you can re-run with the --yes flag to skip this prompt."
        echo
        read -p "Continue with deletion? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

# Ensure the dbutil binary is built
echo -e "${GREEN}Building database utility...${NC}"
cargo build --bin dbutil || {
    echo -e "${RED}Failed to build dbutil binary.${NC}"
    echo "Check the error messages above and make sure your Rust environment is set up correctly."
    exit 1
}

# Run the dbutil binary
echo -e "${GREEN}Running database operation...${NC}"
target/debug/dbutil "$@" || {
    echo -e "${RED}Database operation failed.${NC}"
    echo "Try using the --verbose flag to see more details."
    exit 1
}

echo -e "${GREEN}Database operation completed successfully!${NC}"