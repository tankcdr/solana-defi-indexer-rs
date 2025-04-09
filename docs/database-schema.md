# Database Schema Documentation

This document provides comprehensive documentation of the database schema used in the DEX Event Indexer. The schema is designed to support multiple DEXes with a modular approach.

## Schema Overview

The database schema is organized into two main parts:

1. **Common Schema**: Shared tables used across all DEX types
2. **DEX-specific Schemas**: Tables specific to each supported DEX (currently Orca and Raydium)

All database objects are stored in the `apestrong` schema. The database follows these design principles:

1. **Modular Design**: Each DEX has its own set of tables that can be created or dropped independently
2. **Hierarchical Structure**: Within each DEX schema, a base table stores common fields, and specialized tables store event-specific data
3. **Convenience Views**: Views join the base tables with specialized tables for easier querying
4. **Common Tracking**: Shared tables track tokens, subscribed pools, and processed signatures across all DEXes

## Common Schema Tables

### `apestrong.token_metadata`

Stores information about tokens across all DEXes.

| Column     | Type         | Description                      |
| ---------- | ------------ | -------------------------------- |
| address    | VARCHAR(44)  | Token mint address (primary key) |
| name       | VARCHAR(128) | Token name                       |
| symbol     | VARCHAR(32)  | Token symbol                     |
| decimals   | INT          | Token decimal places             |
| logo_uri   | TEXT         | URI to token logo (optional)     |
| updated_at | TIMESTAMPTZ  | When the record was last updated |

### `apestrong.subscribed_pools`

Tracks all pools being monitored by the indexer.

| Column   | Type        | Description                          |
| -------- | ----------- | ------------------------------------ |
| id       | SERIAL      | Primary key                          |
| address  | VARCHAR(44) | Pool address                         |
| dex      | VARCHAR(32) | DEX type (e.g., "orca", "raydium")   |
| token_a  | VARCHAR(44) | Token A address                      |
| token_b  | VARCHAR(44) | Token B address                      |
| added_at | TIMESTAMPTZ | When the pool was added for indexing |

### `apestrong.last_signatures`

Tracks the last processed transaction signature for each pool.

| Column     | Type        | Description                          |
| ---------- | ----------- | ------------------------------------ |
| pool       | VARCHAR(44) | Pool address (primary key)           |
| dex        | VARCHAR(32) | DEX type                             |
| signature  | VARCHAR(88) | Last processed transaction signature |
| updated_at | TIMESTAMPTZ | When the signature was last updated  |

## Orca Schema Tables

### Base Table: `apestrong.orca_whirlpool_events`

This table stores common information shared by all Orca Whirlpool events.

| Column     | Type        | Description                                      |
| ---------- | ----------- | ------------------------------------------------ |
| id         | SERIAL      | Primary key, auto-incrementing identifier        |
| signature  | VARCHAR(88) | Solana transaction signature (unique)            |
| whirlpool  | VARCHAR(44) | Whirlpool pool address                           |
| event_type | VARCHAR(32) | Type of event (Traded, LiquidityIncreased, etc.) |
| version    | INT         | Schema version (default: 1)                      |
| timestamp  | TIMESTAMPTZ | When the event occurred                          |

**Indexes:**

- `idx_orca_whirlpool_events_whirlpool_timestamp` on (whirlpool, timestamp) - Improves query performance for pool-specific time-series queries

### Event Table: `apestrong.orca_traded_events`

Stores details for swap/trade events.

| Column        | Type    | Description                                   |
| ------------- | ------- | --------------------------------------------- |
| event_id      | INT     | Primary key, references orca_whirlpool_events |
| a_to_b        | BOOLEAN | Direction of the swap (true = A to B)         |
| input_amount  | BIGINT  | Amount of input token                         |
| output_amount | BIGINT  | Amount of output token                        |
| liquidity     | BIGINT  | Pool liquidity at the time of swap            |
| tick          | INT     | Price tick after the swap                     |

### Event Table: `apestrong.orca_liquidity_increased_events`

Stores details for liquidity provision events.

| Column               | Type        | Description                                   |
| -------------------- | ----------- | --------------------------------------------- |
| event_id             | INT         | Primary key, references orca_whirlpool_events |
| position             | VARCHAR(44) | Position NFT address                          |
| tick_lower_index     | INT         | Lower price tick of the position              |
| tick_upper_index     | INT         | Upper price tick of the position              |
| liquidity            | BIGINT      | Amount of liquidity added                     |
| token_a_amount       | BIGINT      | Amount of token A added                       |
| token_b_amount       | BIGINT      | Amount of token B added                       |
| token_a_transfer_fee | BIGINT      | Fee charged for token A transfer              |
| token_b_transfer_fee | BIGINT      | Fee charged for token B transfer              |

### Event Table: `apestrong.orca_liquidity_decreased_events`

Stores details for liquidity removal events.

| Column               | Type        | Description                                   |
| -------------------- | ----------- | --------------------------------------------- |
| event_id             | INT         | Primary key, references orca_whirlpool_events |
| position             | VARCHAR(44) | Position NFT address                          |
| tick_lower_index     | INT         | Lower price tick of the position              |
| tick_upper_index     | INT         | Upper price tick of the position              |
| liquidity            | BIGINT      | Amount of liquidity removed                   |
| token_a_amount       | BIGINT      | Amount of token A removed                     |
| token_b_amount       | BIGINT      | Amount of token B removed                     |
| token_a_transfer_fee | BIGINT      | Fee charged for token A transfer              |
| token_b_transfer_fee | BIGINT      | Fee charged for token B transfer              |

## Raydium Schema Tables

### Base Table: `apestrong.raydium_concentrated_events`

Stores common information for all Raydium concentrated liquidity events.

| Column     | Type        | Description                                 |
| ---------- | ----------- | ------------------------------------------- |
| id         | SERIAL      | Primary key, auto-incrementing identifier   |
| signature  | VARCHAR(88) | Solana transaction signature (unique)       |
| pool       | VARCHAR(44) | Pool address                                |
| event_type | VARCHAR(32) | Type of event (Swap, PositionCreated, etc.) |
| version    | INT         | Schema version (default: 1)                 |
| timestamp  | TIMESTAMPTZ | When the event occurred                     |

**Indexes:**

- `idx_raydium_concentrated_events_pool_timestamp` on (pool, timestamp)

### Event Table: `apestrong.raydium_swap_events`

Stores details for Raydium swap events.

| Column           | Type    | Description                               |
| ---------------- | ------- | ----------------------------------------- |
| event_id         | INT     | Primary key, references base events table |
| in_token_amount  | BIGINT  | Amount of input token                     |
| out_token_amount | BIGINT  | Amount of output token                    |
| fee_amount       | BIGINT  | Fee amount charged for the swap           |
| price            | NUMERIC | Price at which the swap occurred          |

### Additional Raydium Tables

Additional tables for Raydium events follow a similar pattern to the Orca tables, with event-specific fields for different event types.

## Cross-DEX Queries

Since all event data is stored in the same database, you can perform queries across multiple DEXes to compare activity.

```sql
-- Compare swap volume between Orca and Raydium for a specific token pair
WITH orca_volume AS (
    SELECT SUM(t.input_amount) as volume
    FROM apestrong.v_orca_whirlpool_traded t
    JOIN apestrong.subscribed_pools p ON t.whirlpool = p.address
    WHERE p.token_a = 'TokenAAddress' AND p.token_b = 'TokenBAddress'
    AND t.timestamp > NOW() - INTERVAL '24 hours'
),
raydium_volume AS (
    SELECT SUM(s.in_token_amount) as volume
    FROM apestrong.raydium_concentrated_events e
    JOIN apestrong.raydium_swap_events s ON e.id = s.event_id
    JOIN apestrong.subscribed_pools p ON e.pool = p.address
    WHERE p.token_a = 'TokenAAddress' AND p.token_b = 'TokenBAddress'
    AND e.timestamp > NOW() - INTERVAL '24 hours'
)
SELECT 'Orca' as dex, volume FROM orca_volume
UNION ALL
SELECT 'Raydium' as dex, volume FROM raydium_volume;
```

## Relationships

- Each record in the specialized event tables (`orca_traded_events`, `orca_liquidity_increased_events`, `orca_liquidity_decreased_events`) has a one-to-one relationship with a record in the base `orca_whirlpool_events` table.
- The relationship is enforced by foreign key constraints with `ON DELETE CASCADE`.

## Database Views

For convenience, the schema includes views that join the base event table with each specialized event table:

### View: `apestrong.v_orca_whirlpool_traded`

Provides a unified view of trade events with all relevant fields.

```sql
CREATE OR REPLACE VIEW apestrong.v_orca_whirlpool_traded AS
SELECT
    e.id, e.signature, e.whirlpool, e.timestamp,
    t.a_to_b, t.input_amount, t.output_amount, t.liquidity, t.tick
FROM
    apestrong.orca_whirlpool_events e
JOIN
    apestrong.orca_traded_events t ON e.id = t.event_id
WHERE
    e.event_type = 'traded';
```

### View: `apestrong.v_orca_whirlpool_liquidity_increased`

Provides a unified view of liquidity increase events with all relevant fields.

```sql
CREATE OR REPLACE VIEW apestrong.v_orca_whirlpool_liquidity_increased AS
SELECT
    e.id, e.signature, e.whirlpool, e.timestamp,
    l.position, l.tick_lower_index, l.tick_upper_index,
    l.liquidity, l.token_a_amount, l.token_b_amount,
    l.token_a_transfer_fee, l.token_b_transfer_fee
FROM
    apestrong.orca_whirlpool_events e
JOIN
    apestrong.orca_liquidity_increased_events l ON e.id = l.event_id
WHERE
    e.event_type = 'liquidity_increased';
```

### View: `apestrong.v_orca_whirlpool_liquidity_decreased`

Provides a unified view of liquidity decrease events with all relevant fields.

```sql
CREATE OR REPLACE VIEW apestrong.v_orca_whirlpool_liquidity_decreased AS
SELECT
    e.id, e.signature, e.whirlpool, e.timestamp,
    l.position, l.tick_lower_index, l.tick_upper_index,
    l.liquidity, l.token_a_amount, l.token_b_amount,
    l.token_a_transfer_fee, l.token_b_transfer_fee
FROM
    apestrong.orca_whirlpool_events e
JOIN
    apestrong.orca_liquidity_decreased_events l ON e.id = l.event_id
WHERE
    e.event_type = 'liquidity_decreased';
```

## Query Examples

### Getting Recent Trades for a Specific Pool

```sql
SELECT
    signature,
    timestamp,
    a_to_b,
    input_amount,
    output_amount
FROM
    apestrong.v_orca_whirlpool_traded
WHERE
    whirlpool = '[POOL_ADDRESS]'
ORDER BY
    timestamp DESC
LIMIT 10;
```

### Calculating Pool Volume Over a Time Period

```sql
SELECT
    SUM(input_amount) as total_volume
FROM
    apestrong.v_orca_whirlpool_traded
WHERE
    whirlpool = '[POOL_ADDRESS]'
    AND timestamp > NOW() - INTERVAL '24 hours';
```

### Finding Top Pools by Trade Count

```sql
SELECT
    whirlpool,
    COUNT(*) as trade_count
FROM
    apestrong.orca_whirlpool_events
WHERE
    event_type = 'traded'
    AND timestamp > NOW() - INTERVAL '7 days'
GROUP BY
    whirlpool
ORDER BY
    trade_count DESC
LIMIT 10;
```

### Analyzing Liquidity Provider Activity

```sql
SELECT
    position,
    SUM(CASE WHEN e.event_type = 'LiquidityIncreased' THEN l.liquidity ELSE -l.liquidity END) as net_liquidity_change
FROM
    apestrong.orca_whirlpool_events e
JOIN (
    SELECT event_id, position, liquidity FROM apestrong.orca_liquidity_increased_events
    UNION ALL
    SELECT event_id, position, liquidity FROM apestrong.orca_liquidity_decreased_events
) l ON e.id = l.event_id
WHERE
    e.whirlpool = '[POOL_ADDRESS]'
    AND e.timestamp > NOW() - INTERVAL '30 days'
GROUP BY
    position
ORDER BY
    net_liquidity_change DESC;
```

## TypeScript Interfaces

Here are TypeScript interfaces that correspond to the Orca database schema:

```typescript
// Base event interface
interface OrcaWhirlpoolEvent {
  id: number;
  signature: string;
  whirlpool: string;
  event_type: "Traded" | "LiquidityIncreased" | "LiquidityDecreased";
  version: number;
  timestamp: Date;
}

// Traded event interface
interface OrcaTradedEvent {
  event_id: number;
  a_to_b: boolean;
  input_amount: bigint;
  output_amount: bigint;
  liquidity: bigint;
  tick: number;
}

// Common interface for liquidity events
interface OrcaLiquidityEvent {
  event_id: number;
  position: string;
  tick_lower_index: number;
  tick_upper_index: number;
  liquidity: bigint;
  token_a_amount: bigint;
  token_b_amount: bigint;
  token_a_transfer_fee: bigint;
  token_b_transfer_fee: bigint;
}

// Specialized interfaces for liquidity events
interface OrcaLiquidityIncreasedEvent extends OrcaLiquidityEvent {}
interface OrcaLiquidityDecreasedEvent extends OrcaLiquidityEvent {}

// Combined view interfaces
interface OrcaWhirlpoolTradedView extends OrcaWhirlpoolEvent, OrcaTradedEvent {}
interface OrcaWhirlpoolLiquidityIncreasedView
  extends OrcaWhirlpoolEvent,
    OrcaLiquidityIncreasedEvent {}
interface OrcaWhirlpoolLiquidityDecreasedView
  extends OrcaWhirlpoolEvent,
    OrcaLiquidityDecreasedEvent {}
```

## Raydium TypeScript Interfaces

Here are TypeScript interfaces that correspond to the Raydium database schema:

```typescript
// Base event interface
interface RaydiumConcentratedEvent {
  id: number;
  signature: string;
  pool: string;
  event_type: "Swap" | "PositionCreated" | "PositionClosed";
  version: number;
  timestamp: Date;
}

// Swap event interface
interface RaydiumSwapEvent {
  event_id: number;
  in_token_amount: bigint;
  out_token_amount: bigint;
  fee_amount: bigint;
  price: number;
}

// Combined view interfaces
interface RaydiumSwapView extends RaydiumConcentratedEvent, RaydiumSwapEvent {}
```

## Common Tables TypeScript Interfaces

```typescript
// Token metadata interface
interface TokenMetadata {
  address: string;
  name: string;
  symbol: string;
  decimals: number;
  logo_uri?: string;
  updated_at: Date;
}

// Subscribed pool interface
interface SubscribedPool {
  id: number;
  address: string;
  dex: "orca" | "raydium";
  token_a: string;
  token_b: string;
  added_at: Date;
}

// Last signature interface
interface LastSignature {
  pool: string;
  dex: string;
  signature: string;
  updated_at: Date;
}
```

## Usage Notes

1. **BigInt Handling**: Database fields with type `BIGINT` correspond to TypeScript's `bigint` type. When working with these values in a web context, you may need to convert them to strings since `bigint` doesn't serialize to JSON directly.

2. **Date Handling**: PostgreSQL `TIMESTAMPTZ` fields are returned as JavaScript `Date` objects when using libraries like `pg`, `node-postgres`, or `sqlx` with TypeScript.

3. **Solana Addresses**: Addresses are stored as strings rather than byte arrays for easier handling in the database. When using these in Solana transactions, you'll need to convert them to `PublicKey` objects.

4. **Query Performance**: When querying large date ranges, make use of the indexes on pool and timestamp columns by including the pool address in your WHERE clause.

5. **Views vs. Direct Table Access**: For most application needs, using the views is recommended as they provide all the necessary fields in a denormalized format. For complex queries, joining the base tables directly may provide more flexibility.

6. **Cross-DEX Queries**: For queries that span multiple DEXes, join through the `subscribed_pools` table to find related pools across different DEXes.
