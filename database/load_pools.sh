#!/bin/bash
# Script to load pools from multiple DEXes into the database

# Exit on error
set -e

# Change to the project root directory
cd "$(dirname "$0")/.." || exit 1

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Default DEX type if not specified
DEX_TYPE=${1:-all}

# Print header
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  DEX Pool Loader Utility${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Loading pools for DEX: ${DEX_TYPE}${NC}"
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
echo -e "${GREEN}Building load_pools utility...${NC}"
cargo build --bin load_pools || {
    echo -e "${RED}Failed to build load_pools utility.${NC}"
    echo "Check the error messages above and make sure your Rust environment is set up correctly."
    exit 1
}

# Validate DEX type
if [[ "$DEX_TYPE" != "all" && "$DEX_TYPE" != "orca" && "$DEX_TYPE" != "raydium" ]]; then
    echo -e "${RED}Invalid DEX type: ${DEX_TYPE}${NC}"
    echo "Valid DEX types: orca, raydium, all"
    exit 1
fi

# Run the loader utility
echo -e "${GREEN}Running pool loader for ${DEX_TYPE}...${NC}"

# Verbose flag handling
VERBOSE_FLAG=""
if [[ "$*" == *"--verbose"* ]]; then
    VERBOSE_FLAG="--verbose"
fi

target/debug/load_pools --dex "$DEX_TYPE" $VERBOSE_FLAG || {
    echo -e "${RED}Failed to load pools for ${DEX_TYPE}${NC}"
    exit 1
}

echo -e "${GREEN}Successfully loaded pools for ${DEX_TYPE}${NC}"
echo -e "${GREEN}Pool loading completed.${NC}"