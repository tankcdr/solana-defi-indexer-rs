#!/bin/bash
# Database initialization script that runs in Docker container
# This script:
# 1. Creates the database schema
# 2. Loads predefined Orca Whirlpool pools

set -e

# Get the directory where the script is located
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Print status message
echo "Initializing database..."

# 1. Create schema
echo "Creating database schema..."
PGPASSWORD=$POSTGRES_PASSWORD psql -h localhost -U $POSTGRES_USER -d $POSTGRES_DB -f "$DIR/schema.sql"

# 2. Wait a moment to ensure schema is fully applied
sleep 2

# 3. Load predefined Orca Whirlpool pools using the utility
if [ -f "$DIR/config/orca_pools.txt" ]; then
    echo "Loading Orca Whirlpool pools..."
    while IFS= read -r pool_address || [ -n "$pool_address" ]; do
        # Skip empty lines and comments
        if [[ -z "$pool_address" || "$pool_address" =~ ^# ]]; then
            continue
        fi
        echo "Loading pool: $pool_address"
        # Use psql to insert the pool (the real data will be fetched by the indexer later)
        # This is a simplified version just to populate the pool addresses
        PGPASSWORD=$POSTGRES_PASSWORD psql -h localhost -U $POSTGRES_USER -d $POSTGRES_DB -c \
            "INSERT INTO apestrong.orca_whirlpool_pools
             (whirlpool, token_mint_a, token_mint_b, token_name_a, token_name_b, pool_name, decimals_a, decimals_b)
             VALUES ('$pool_address', 'placeholder', 'placeholder', NULL, NULL, NULL, 0, 0)
             ON CONFLICT (whirlpool) DO NOTHING;"
    done < "$DIR/config/orca_pools.txt"
    echo "Pools loaded successfully!"
else
    echo "No pool list found at $DIR/config/orca_pools.txt"
    echo "Skipping pool loading."
fi

echo "Database initialization completed successfully!"