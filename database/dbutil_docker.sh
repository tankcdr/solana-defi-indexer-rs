#!/bin/bash
# dbutil_docker.sh - Docker-optimized wrapper for database utility

# Exit on error
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Print header
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Database Utility Tool${NC}"
echo -e "${GREEN}  Docker Edition${NC}"
echo -e "${GREEN}========================================${NC}"
echo

# Display usage information if requested
if [[ "$1" == "--help" || "$1" == "-h" || $# -eq 0 ]]; then
    echo "Database Utility Tool (Docker Edition)"
    echo
    echo "Usage: $0 OPERATION DEX [OPTIONS]"
    echo
    echo "This script manages database schemas for different DEXes in Docker environment."
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
        read -p "Continue with deletion? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

# Run the dbutil binary
echo -e "${GREEN}Running database operation...${NC}"

# In Docker we use the pre-built binary
/app/dbutil "$@" || {
    echo -e "${RED}Database operation failed.${NC}"
    echo "Try using the --verbose flag to see more details."
    exit 1
}

echo -e "${GREEN}Database operation completed successfully!${NC}"