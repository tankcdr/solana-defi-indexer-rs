/******************************************************************************
 * IMPORTANT: DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
 *
 * This file is protected and should not be modified without explicit approval.
 * Any changes could break the indexer functionality.
 *
 * See .nooverwrite.json for more information on protected files.
 ******************************************************************************/

use anyhow::{ Context, Result };
use sqlx::{ PgPool, Postgres, Transaction, Row };

use crate::db::common::Repository;
use crate::models::orca::whirlpool::{
    OrcaWhirlpoolEvent,
    OrcaWhirlpoolTradedEventRecord,
    OrcaWhirlpoolLiquidityIncreasedEventRecord,
    OrcaWhirlpoolLiquidityDecreasedEventRecord,
};

/// Repository for Orca Whirlpool event database operations
pub struct OrcaWhirlpoolRepository {
    pool: PgPool,
}

impl OrcaWhirlpoolRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a base Orca Whirlpool event
    async fn insert_base_event<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        event: &OrcaWhirlpoolEvent
    ) -> Result<i32> {
        let row = sqlx
            ::query(
                "INSERT INTO apestrong.orca_whirlpool_events (signature, whirlpool, event_type, version) VALUES ($1, $2, $3, $4) RETURNING id"
            )
            .bind(&event.signature)
            .bind(&event.whirlpool)
            .bind(&event.event_type)
            .bind(event.version)
            .fetch_one(&mut **tx).await
            .context("Failed to insert base Orca Whirlpool event")?;

        let id: i32 = row.get("id");
        Ok(id)
    }

    /// Insert a traded event into the database
    pub async fn insert_traded_event(&self, event: OrcaWhirlpoolTradedEventRecord) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the traded-specific data
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_traded_events (event_id, a_to_b, pre_sqrt_price, post_sqrt_price, input_amount, output_amount, input_transfer_fee, output_transfer_fee, lp_fee, protocol_fee) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind(event_id)
            .bind(event.data.a_to_b)
            .bind(event.data.pre_sqrt_price)
            .bind(event.data.post_sqrt_price)
            .bind(event.data.input_amount)
            .bind(event.data.output_amount)
            .bind(event.data.input_transfer_fee)
            .bind(event.data.output_transfer_fee)
            .bind(event.data.lp_fee)
            .bind(event.data.protocol_fee)
            .execute(&mut *tx).await
            .context("Failed to insert Orca Whirlpool traded event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    /// Insert a liquidity increased event into the database
    pub async fn insert_liquidity_increased_event(
        &self,
        event: OrcaWhirlpoolLiquidityIncreasedEventRecord
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the liquidity data
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_liquidity_increased_events (event_id, position, tick_lower_index, tick_upper_index, liquidity, token_a_amount, token_b_amount, token_a_transfer_fee, token_b_transfer_fee) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
            )
            .bind(event_id)
            .bind(&event.data.position)
            .bind(event.data.tick_lower_index)
            .bind(event.data.tick_upper_index)
            .bind(event.data.liquidity)
            .bind(event.data.token_a_amount)
            .bind(event.data.token_b_amount)
            .bind(event.data.token_a_transfer_fee)
            .bind(event.data.token_b_transfer_fee)
            .execute(&mut *tx).await
            .context("Failed to insert Orca Whirlpool liquidity increased event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    /// Insert a liquidity decreased event into the database
    pub async fn insert_liquidity_decreased_event(
        &self,
        event: OrcaWhirlpoolLiquidityDecreasedEventRecord
    ) -> Result<i32> {
        let mut tx = self.pool.begin().await?;

        // Insert the base event
        let event_id = self.insert_base_event(&mut tx, &event.base).await?;

        // Insert the liquidity data
        sqlx
            ::query(
                "INSERT INTO apestrong.orca_liquidity_decreased_events (event_id, position, tick_lower_index, tick_upper_index, liquidity, token_a_amount, token_b_amount, token_a_transfer_fee, token_b_transfer_fee) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
            )
            .bind(event_id)
            .bind(&event.data.position)
            .bind(event.data.tick_lower_index)
            .bind(event.data.tick_upper_index)
            .bind(event.data.liquidity)
            .bind(event.data.token_a_amount)
            .bind(event.data.token_b_amount)
            .bind(event.data.token_a_transfer_fee)
            .bind(event.data.token_b_transfer_fee)
            .execute(&mut *tx).await
            .context("Failed to insert Orca Whirlpool liquidity decreased event")?;

        tx.commit().await?;
        Ok(event_id)
    }

    /// Get recent trade volume for a specific pool
    pub async fn get_recent_trade_volume(&self, pool_address: &str, hours: i64) -> Result<i64> {
        let row = sqlx
            ::query(
                "SELECT COALESCE(SUM(t.input_amount), 0) as volume FROM apestrong.orca_whirlpool_events e JOIN apestrong.orca_traded_events t ON e.id = t.event_id WHERE e.whirlpool = $1 AND e.event_type = 'traded' AND e.timestamp > NOW() - INTERVAL '1 hour' * $2"
            )
            .bind(pool_address)
            .bind(hours)
            .fetch_one(&self.pool).await
            .context("Failed to get recent trade volume")?;

        let volume: Option<i64> = row.get("volume");
        Ok(volume.unwrap_or(0))
    }
}

impl Repository for OrcaWhirlpoolRepository {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
}
