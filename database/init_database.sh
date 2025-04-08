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
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$DIR/schema.sql" || {
    echo "Error: Failed to apply schema.sql"
    exit 1
}

echo "Database initialization completed successfully!"