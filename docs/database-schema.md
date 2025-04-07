# Database Schema Documentation

This document provides comprehensive documentation of the database schema used in the prediction market indexer, specifically focusing on the Orca Whirlpool events. This documentation is intended for TypeScript developers who need to interact with the database.

## Schema Overview

All database objects are stored in the `apestrong` schema. The database follows a hierarchical design pattern for events:

1. A base table (`orca_whirlpool_events`) stores common fields for all event types
2. Specialized tables store event-specific data for each event type
3. Views join the base table with specialized tables for convenient querying

## Tables

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

Here are TypeScript interfaces that correspond to the database schema:

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

## Usage Notes

1. **BigInt Handling**: Database fields with type `BIGINT` correspond to TypeScript's `bigint` type. When working with these values in a web context, you may need to convert them to strings since `bigint` doesn't serialize to JSON directly.

2. **Date Handling**: PostgreSQL `TIMESTAMPTZ` fields are returned as JavaScript `Date` objects when using libraries like `pg`, `node-postgres`, or `sqlx` with TypeScript.

3. **Solana Addresses**: Addresses are stored as strings rather than byte arrays for easier handling in the database. When using these in Solana transactions, you'll need to convert them to `PublicKey` objects.

4. **Query Performance**: When querying large date ranges, make use of the `idx_orca_whirlpool_events_whirlpool_timestamp` index by including the `whirlpool` column in your WHERE clause.

5. **Views vs. Direct Table Access**: For most application needs, using the views (`v_orca_whirlpool_traded`, etc.) is recommended as they provide all the necessary fields in a denormalized format. For complex queries, joining the base tables directly may provide more flexibility.
