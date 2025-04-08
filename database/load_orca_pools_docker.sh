#!/bin/bash
# Docker-optimized script to load multiple Orca Whirlpool pools into the database

# Exit on error
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Print header
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Orca Whirlpool Pool Loader Utility${NC}"
echo -e "${GREEN}  Docker Edition${NC}"
echo -e "${GREEN}========================================${NC}"
echo
# Count pools in the configuration file
echo -e "${YELLOW}Counting pools in configuration file...${NC}"
POOLS_IN_CONFIG=0
if [ -f "/app/database/config/orca_pools.txt" ]; then
    # Count non-empty, non-comment lines in the pools file
    POOLS_IN_CONFIG=$(grep -v '^#' /app/database/config/orca_pools.txt | grep -v '^[[:space:]]*$' | wc -l)
    echo -e "${GREEN}Found ${POOLS_IN_CONFIG} pools in configuration file.${NC}"
else
    echo -e "${YELLOW}No pool configuration file found at /app/database/config/orca_pools.txt${NC}"
fi

# Check if pools have already been loaded
echo -e "${YELLOW}Checking for pools in the database...${NC}"
if [ -n "$DATABASE_URL" ]; then
    # Use psql to get an exact pool count from the database
    # We trim whitespace and make sure we have a clean number
    DB_POOL_COUNT=$(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM apestrong.orca_whirlpool_pools" | tr -d ' \n\t')
    
    # Make sure we have a valid number
    if [[ ! "$DB_POOL_COUNT" =~ ^[0-9]+$ ]]; then
        echo -e "${RED}Invalid pool count returned from database: '${DB_POOL_COUNT}'. Assuming 0.${NC}"
        DB_POOL_COUNT=0
    fi
    
    echo -e "${GREEN}Found ${DB_POOL_COUNT} pools in database.${NC}"
    
    # Compare counts
    if [ "$DB_POOL_COUNT" -ge "$POOLS_IN_CONFIG" ] && [ "$POOLS_IN_CONFIG" -gt 0 ]; then
        echo -e "${GREEN}Database already has at least as many pools as config file. Skipping initialization.${NC}"
        exit 0
    else
        echo -e "${GREEN}Database has fewer pools (${DB_POOL_COUNT}) than config (${POOLS_IN_CONFIG}) or config is empty. Proceeding with initialization.${NC}"
    fi
else
    echo -e "${YELLOW}DATABASE_URL not set. Unable to check database state. Proceeding anyway.${NC}"
fi

# In Docker, we don't need to build the binary as it's pre-built
# and we don't need to source .env as environment is passed via Docker

# Function to load a single pool
load_pool() {
    local pool_address=$1
    echo -e "${GREEN}Loading pool: ${pool_address}${NC}"
    
    # Run the loader utility with additional args if provided
    if [ $# -gt 1 ]; then
        # Pass additional arguments after the pool address
        shift
        /app/load_orca_pool "$pool_address" "$@" || {
            echo -e "${RED}Failed to load pool: ${pool_address}${NC}"
            return 1
        }
    else
        # Just pass the pool address (env vars will be read from environment)
        /app/load_orca_pool "$pool_address" || {
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
    if [ -f "/app/database/config/orca_pools.txt" ]; then
        echo "Loading pools from /app/database/config/orca_pools.txt..."
        while IFS= read -r pool_address || [ -n "$pool_address" ]; do
            # Skip empty lines and comments
            if [[ -z "$pool_address" || "$pool_address" =~ ^# ]]; then
                continue
            fi
            load_pool "$pool_address"
        done < "/app/database/config/orca_pools.txt"
    else
        echo -e "${YELLOW}No pool list found at /app/database/config/orca_pools.txt${NC}"
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