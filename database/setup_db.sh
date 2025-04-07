#!/bin/bash
# setup_db.sh - Wrapper for the database setup utility

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
echo -e "${GREEN}  Indexer Database Setup Utility${NC}"
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
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    echo "Database Setup Utility"
    echo
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "This script is a wrapper around the setup_db binary which applies the database schema."
    echo
    echo "Options:"
    echo "  --schema-file PATH      Path to the schema SQL file"
    echo "  --database-url URL      Override database URL from .env"
    echo "  --drop-existing         Drop existing tables before creation (USE WITH CAUTION)"
    echo "  --verbose, -v           Show verbose output"
    echo "  --help, -h              Show this help message"
    echo
    echo "Examples:"
    echo "  $0                      Run with default settings"
    echo "  $0 --verbose            Run with verbose output"
    echo "  $0 --drop-existing      Drop and recreate all tables"
    echo
    exit 0
fi

# Warn if --drop-existing is used
if [[ "$@" == *"--drop-existing"* ]]; then
    echo -e "${RED}WARNING: You are about to DROP ALL EXISTING TABLES!${NC}"
    echo "This will delete all data in the following tables:"
    echo "  - orca_whirlpool_events"
    echo "  - orca_traded_events"
    echo "  - orca_liquidity_increased_events"
    echo "  - orca_liquidity_decreased_events"
    echo
    read -p "Are you sure you want to continue? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Ensure the setup_db binary is built
echo -e "${GREEN}Building setup_db utility...${NC}"
cargo build --bin setup_db || {
    echo -e "${RED}Failed to build setup_db utility.${NC}"
    echo "Check the error messages above and make sure your Rust environment is set up correctly."
    exit 1
}

# Run the setup_db binary
echo -e "${GREEN}Running database setup...${NC}"
target/debug/setup_db "$@" || {
    echo -e "${RED}Database setup failed.${NC}"
    echo "Try using the --verbose flag to see more details."
    exit 1
}

echo -e "${GREEN}Database setup completed successfully!${NC}"