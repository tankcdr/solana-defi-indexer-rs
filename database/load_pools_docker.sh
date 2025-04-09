#!/bin/bash
# Docker-optimized script to load multiple DEX pools into the database

# Exit on error
set -e

# Default DEX type if not specified
DEX_TYPE=${1:-all}

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Print header
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  DEX Pool Loader Utility${NC}"
echo -e "${GREEN}  Docker Edition${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Loading pools for DEX: ${DEX_TYPE}${NC}"
echo

# Validate DEX type
if [[ "$DEX_TYPE" != "all" && "$DEX_TYPE" != "orca" && "$DEX_TYPE" != "raydium" ]]; then
    echo -e "${RED}Invalid DEX type: ${DEX_TYPE}${NC}"
    echo "Valid DEX types: orca, raydium, all"
    exit 1
fi

# Determine which DEXes to process
DEX_TYPES=()
if [ "$DEX_TYPE" == "all" ]; then
    DEX_TYPES=("orca" "raydium")
else
    DEX_TYPES=("$DEX_TYPE")
fi

# Process each DEX
for current_dex in "${DEX_TYPES[@]}"; do
    echo -e "${GREEN}Processing ${current_dex} pools...${NC}"
    
    # Count pools in the configuration file
    echo -e "${YELLOW}Counting ${current_dex} pools in configuration file...${NC}"
    POOLS_IN_CONFIG=0
    CONFIG_FILE="/app/database/schema/${current_dex}/subscribed_pools.txt"
    
    if [ -f "$CONFIG_FILE" ]; then
        # Count non-empty, non-comment lines in the pools file
        POOLS_IN_CONFIG=$(grep -v '^#' "$CONFIG_FILE" | grep -v '^[[:space:]]*$' | wc -l)
        echo -e "${GREEN}Found ${POOLS_IN_CONFIG} ${current_dex} pools in configuration file.${NC}"
    else
        echo -e "${YELLOW}No pool configuration file found at ${CONFIG_FILE}${NC}"
        continue
    fi
    
    # Check if pools have already been loaded
    echo -e "${YELLOW}Checking for ${current_dex} pools in the database...${NC}"
    if [ -n "$DATABASE_URL" ]; then
        # Use psql to get an exact pool count from the database for this DEX
        DB_POOL_COUNT=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM apestrong.subscribed_pools WHERE dex = '${current_dex}'" | tr -d ' \n\t')
        
        # Make sure we have a valid number
        if [[ ! "$DB_POOL_COUNT" =~ ^[0-9]+$ ]]; then
            echo -e "${RED}Invalid pool count returned from database: '${DB_POOL_COUNT}'. Assuming 0.${NC}"
            DB_POOL_COUNT=0
        fi
        
        echo -e "${GREEN}Found ${DB_POOL_COUNT} ${current_dex} pools in database.${NC}"
        
        # Compare counts
        if [ "$DB_POOL_COUNT" -ge "$POOLS_IN_CONFIG" ] && [ "$POOLS_IN_CONFIG" -gt 0 ]; then
            echo -e "${GREEN}Database already has at least as many ${current_dex} pools as config file. Skipping.${NC}"
            continue
        else
            echo -e "${GREEN}Database has fewer ${current_dex} pools (${DB_POOL_COUNT}) than config (${POOLS_IN_CONFIG}) or config is empty. Proceeding with initialization.${NC}"
        fi
    else
        echo -e "${YELLOW}DATABASE_URL not set. Unable to check database state. Proceeding anyway.${NC}"
    fi
done

# In Docker, we don't need to build the binary as it's pre-built
# and we don't need to source .env as environment is passed via Docker

# Check for any valid pool configurations to process
if [ ${#DEX_TYPES[@]} -eq 0 ]; then
    echo -e "${RED}No valid DEX configurations found. Exiting.${NC}"
    exit 1
fi

# Run the loader utility with the specified DEX type
echo -e "${GREEN}Running pool loader for ${DEX_TYPE}...${NC}"

# Verbose flag handling
VERBOSE_FLAG=""
if [[ "$*" == *"--verbose"* ]]; then
    VERBOSE_FLAG="--verbose"
fi

/app/load_pools --dex "$DEX_TYPE" $VERBOSE_FLAG || {
    echo -e "${RED}Failed to load pools for ${DEX_TYPE}${NC}"
    exit 1
}

echo -e "${GREEN}Successfully loaded pools for ${DEX_TYPE}${NC}"
echo -e "${GREEN}Pool loading completed.${NC}"