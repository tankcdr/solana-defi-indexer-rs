#!/bin/bash
# Script to load multiple Orca Whirlpool pools into the database

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
echo -e "${GREEN}  Orca Whirlpool Pool Loader Utility${NC}"
echo -e "${GREEN}========================================${NC}"
echo

# Load environment variables from .env file if it exists
if [ -f ".env" ]; then
    echo -e "${GREEN}Loading environment from .env file...${NC}"
    
    # Source the .env file to export all variables
    set -o allexport
    source .env
    set +o allexport
    
    echo "Environment loaded successfully."
else
    echo -e "${YELLOW}No .env file found. Using default environment settings.${NC}"
fi

# Build the tool if needed
echo -e "${GREEN}Building load_orca_pool utility...${NC}"
cargo build --bin load_orca_pool || {
    echo -e "${RED}Failed to build load_orca_pool utility.${NC}"
    echo "Check the error messages above and make sure your Rust environment is set up correctly."
    exit 1
}

# Function to load a single pool
load_pool() {
    local pool_address=$1
    echo -e "${GREEN}Loading pool: ${pool_address}${NC}"
    
    # Run the loader utility with additional args if provided
    if [ $# -gt 1 ]; then
        # Pass additional arguments after the pool address
        shift
        target/debug/load_orca_pool "$pool_address" "$@" || {
            echo -e "${RED}Failed to load pool: ${pool_address}${NC}"
            return 1
        }
    else
        # Just pass the pool address (env vars will be read from environment)
        target/debug/load_orca_pool "$pool_address" || {
            echo -e "${RED}Failed to load pool: ${pool_address}${NC}"
            return 1
        }
    fi
    
    echo -e "${GREEN}Successfully loaded pool: ${pool_address}${NC}"
    return 0
}

# Check if we have pool addresses as arguments
if [ $# -eq 0 ]; then
    # No arguments, try to load from a predefined list
    echo "No pool addresses provided as arguments."
    
    # Check if we have a pools.txt file with a list of pools
    if [ -f "database/config/orca_pools.txt" ]; then
        echo "Loading pools from database/config/orca_pools.txt..."
        while IFS= read -r pool_address || [ -n "$pool_address" ]; do
            # Skip empty lines and comments
            if [[ -z "$pool_address" || "$pool_address" =~ ^# ]]; then
                continue
            fi
            load_pool "$pool_address"
        done < "database/config/orca_pools.txt"
    else
        echo -e "${YELLOW}No pool list found at database/config/orca_pools.txt${NC}"
        echo "Please provide pool addresses as arguments or create a pools.txt file."
        echo
        echo "Usage: $0 [pool_address1] [pool_address2] ..."
        echo "Example: $0 Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"
        exit 1
    fi
else
    # Process each pool address from command line arguments
    for pool_address in "$@"; do
        load_pool --verbose "$pool_address"
    done
fi

echo -e "${GREEN}Pool loading completed.${NC}"