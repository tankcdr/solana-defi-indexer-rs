#!/bin/bash
# Database initialization script that runs in Docker container
# This script:
# 1. Creates the common database schema
# 2. Creates the specific DEX schema (orca, raydium, or all if not specified)

set -e


DIR="/opt/indexer/database"
# Get the schema argument
SCHEMA_TYPE="${1:-all}"

# Print status message
echo "Initializing database for schema type: $SCHEMA_TYPE..."

# 1. Always create common schema first
echo "Creating common database schema..."
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DIR/schema/common/schema.sql" || {
    echo "Error: Failed to apply common schema"
    exit 1
}

# 2. Create specific DEX schema or all schemas
if [ "$SCHEMA_TYPE" = "all" ]; then
    # Apply all available schemas
    echo "Applying all DEX schemas..."
    
    # Apply Orca schema
    echo "Creating Orca schema..."
    psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DIR/schema/orca/schema.sql" || {
        echo "Error: Failed to apply Orca schema"
        exit 1
    }
    
    # Apply Raydium schema
    echo "Creating Raydium schema..."
    psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DIR/schema/raydium/schema.sql" || {
        echo "Error: Failed to apply Raydium schema"
        exit 1
    }
    
    # Add other DEX schemas here as needed
    
else
    # Apply specific schema
    if [ "$SCHEMA_TYPE" = "orca" ] || [ "$SCHEMA_TYPE" = "raydium" ]; then
        echo "Creating $SCHEMA_TYPE schema..."
        psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DIR/schema/$SCHEMA_TYPE/schema.sql" || {
            echo "Error: Failed to apply $SCHEMA_TYPE schema"
            exit 1
        }
    else
        echo "Error: Unknown schema type '$SCHEMA_TYPE'. Supported types: orca, raydium, all"
        exit 1
    fi
fi

echo "Database initialization completed successfully!"