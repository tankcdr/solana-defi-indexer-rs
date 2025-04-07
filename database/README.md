# Database Setup Utility

This directory contains tools for setting up and managing the indexer's database.

## Schema Overview

The `schema.sql` file defines the database schema with the following main tables:

- `orca_whirlpool_events`: Base table for all Orca Whirlpool events
- `orca_traded_events`: Stores details for trading events
- `orca_liquidity_increased_events`: Stores details for liquidity increase events
- `orca_liquidity_decreased_events`: Stores details for liquidity decrease events

Additionally, there are views that join these tables for easier querying:

- `v_orca_whirlpool_traded`
- `v_orca_whirlpool_liquidity_increased`
- `v_orca_whirlpool_liquidity_decreased`

## Setup Utility

The database setup utility (`setup_db`) provides an easy way to create and update the database schema.

### Prerequisites

- Rust toolchain installed
- PostgreSQL database credentials
- Environment configuration (`.env` file or environment variables)

### Configuration

Configuration is loaded from the following sources, in order of precedence:

1. Command-line arguments
2. Environment variables
3. `.env` file (if present)

Required configuration:

- `DATABASE_URL`: PostgreSQL connection string (e.g., `postgres://user:password@localhost:5432/dbname`)

Optional configuration:

- `DATABASE_MAX_CONNECTIONS`: Maximum number of database connections (default: 5)
- `DATABASE_CONNECT_TIMEOUT`: Connection timeout in seconds (default: 30)

### Usage

#### Using the Shell Script (Recommended)

```bash
# Run with default settings (uses .env file)
./database/setup_db.sh

# Run with verbose output
./database/setup_db.sh --verbose

# Use a custom schema file
./database/setup_db.sh --schema-file path/to/custom-schema.sql

# Override database URL
./database/setup_db.sh --database-url postgres://user:password@host:port/dbname

# Use a specific schema (instead of public)
./database/setup_db.sh --schema myschema

# Drop existing tables and recreate them (CAUTION: DESTROYS DATA)
./database/setup_db.sh --drop-existing
```

#### Using Cargo Directly

```bash
# Run with default settings
cargo run --bin setup_db

# Run with verbose output
cargo run --bin setup_db -- --verbose

# Use a custom schema file
cargo run --bin setup_db -- --schema-file path/to/custom-schema.sql

# Override database URL
cargo run --bin setup_db -- --database-url postgres://user:password@host:port/dbname

# Use a specific schema
cargo run --bin setup_db -- --schema myschema

# Drop existing tables and recreate them (CAUTION: DESTROYS DATA)
cargo run --bin setup_db -- --drop-existing
```

### Example `.env` File

Create a `.env` file in the project root with the following content:

```
# Database connection
DATABASE_URL=postgres://username:password@localhost:5432/indexer
DATABASE_MAX_CONNECTIONS=5
DATABASE_CONNECT_TIMEOUT=30

# Solana RPC settings
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
SOLANA_WS_URL=wss://api.mainnet-beta.solana.com
```

## Troubleshooting

### Common Issues

1. **Tables Not Created**:

   If the script runs successfully but tables aren't created, try:

   - Run with `--verbose` to see exactly what SQL is being executed
   - Check if you need to specify a schema with `--schema yourschema`
   - Try running with `--drop-existing` if tables might exist but have issues
   - Check PostgreSQL logs for any errors not shown in the output

2. **Connection Error**:

   ```
   Error: Failed to connect to database
   ```

   Ensure your PostgreSQL server is running and the connection string is correct:

   - Check if the host and port are correct
   - Verify username and password
   - Make sure the database exists (may need to create it first)
   - Check if network access is allowed (pg_hba.conf settings)

3. **Permission Error**:

   ```
   Error: permission denied for database "indexer"
   ```

   Ensure your database user has the necessary permissions:

   - User needs CREATE permission on the database
   - For using schemas, user needs CREATE permission on the schema
   - For dropping tables, user needs DROP permission

4. **Schema File Not Found**:

   ```
   Error: Failed to read schema file: database/schema.sql
   ```

   Make sure you're running the command from the project root directory.

5. **SQL Syntax Error**:

   ```
   SQL Error: syntax error at or near...
   ```

   Check the schema.sql file for syntax errors. The utility outputs the exact SQL statement causing the error when run with `--verbose`.

### Validating Success

To verify that the tables were created successfully, you can:

1. Connect to your database and list tables:

   ```sql
   \dt orca_*
   ```

2. Check the indexer log output, which should show:

   ```
   Successfully applied database schema!
   Created tables: orca_whirlpool_events, orca_traded_events, ...
   ```

3. Try running a simple query:
   ```sql
   SELECT COUNT(*) FROM orca_whirlpool_events;
   ```

### Getting Help

Run the setup utility with the `--help` flag for a list of available options:

```bash
./database/setup_db.sh --help
```

## Technical Details

### Database Schema Management

The setup utility handles the creation of the database schema by:

1. Reading the SQL file and splitting it into individual statements
2. Creating the schema if specified and it doesn't exist
3. Executing each SQL statement individually
4. Verifying that all tables were created
5. Reporting success or detailed error information

### SQL Statement Splitting

The utility intelligently parses the SQL file to handle:

- Multiple statements separated by semicolons
- Comments (both single-line and block comments)
- String literals containing semicolons
- Complex statements with nested syntax

This allows complex schema files to be properly executed.

## Schema Migrations

Currently, the setup utility applies the full schema in `schema.sql`. For future versions, a more comprehensive migration system will be implemented.

## Advanced Usage

### Integration with Docker

When using Docker, you can pass the database URL as an environment variable:

```bash
docker run -e DATABASE_URL=postgres://user:password@host:port/dbname indexer setup_db
```

### CI/CD Integration

For continuous integration environments, use the `--database-url` flag to provide credentials securely:

```bash
./database/setup_db.sh --database-url "$DATABASE_URL" --verbose
```
