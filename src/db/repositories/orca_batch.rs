use anyhow::{ Context, Result };
use sqlx::{ Postgres, Transaction, Row as _ };

use crate::db::repositories::OrcaWhirlpoolRepository;
use crate::models::orca::whirlpool::{
    OrcaWhirlpoolEvent,
    OrcaWhirlpoolTradedEventRecord,
    OrcaWhirlpoolLiquidityIncreasedEventRecord,
    OrcaWhirlpoolLiquidityDecreasedEventRecord,
};
use crate::db::common::Repository;

/// Extension trait to add batch operations to OrcaWhirlpoolRepository
pub trait OrcaWhirlpoolBatchRepository {
    /// Insert multiple traded events in a single transaction
    async fn batch_insert_traded_events(
        &self,
        events: Vec<OrcaWhirlpoolTradedEventRecord>
    ) -> Result<Vec<i32>>;

    /// Insert multiple liquidity increased events in a single transaction
    async fn batch_insert_liquidity_increased_events(
        &self,
        events: Vec<OrcaWhirlpoolLiquidityIncreasedEventRecord>
    ) -> Result<Vec<i32>>;

    /// Insert multiple liquidity decreased events in a single transaction
    async fn batch_insert_liquidity_decreased_events(
        &self,
        events: Vec<OrcaWhirlpoolLiquidityDecreasedEventRecord>
    ) -> Result<Vec<i32>>;

    /// Insert multiple base events in a single transaction
    async fn batch_insert_base_events<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        events: &[OrcaWhirlpoolEvent]
    ) -> Result<Vec<i32>>;
}

impl OrcaWhirlpoolBatchRepository for OrcaWhirlpoolRepository {
    async fn batch_insert_base_events<'a>(
        &self,
        tx: &mut Transaction<'a, Postgres>,
        events: &[OrcaWhirlpoolEvent]
    ) -> Result<Vec<i32>> {
        let mut event_ids = Vec::with_capacity(events.len());

        for event in events {
            // Create the query for each base event
            let row = sqlx
                ::query(
                    "INSERT INTO apestrong.orca_whirlpool_events (signature, whirlpool, event_type, version) VALUES ($1, $2, $3, $4) RETURNING id"
                )
                .bind(&event.signature)
                .bind(&event.whirlpool)
                .bind(&event.event_type)
                .bind(event.version)
                .fetch_one(&mut **tx).await
                .context("Failed to insert base Orca Whirlpool event in batch")?;

            let id: i32 = row.try_get("id")?;
            event_ids.push(id);
        }

        Ok(event_ids)
    }

    async fn batch_insert_traded_events(
        &self,
        events: Vec<OrcaWhirlpoolTradedEventRecord>
    ) -> Result<Vec<i32>> {
        // Early return if there are no events to process
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = Repository::pool(self).begin().await?;

        // Extract base events
        let base_events: Vec<OrcaWhirlpoolEvent> = events
            .iter()
            .map(|event| event.base.clone())
            .collect();

        // Insert all base events in batch
        let event_ids = self.batch_insert_base_events(&mut tx, &base_events).await?;

        // Insert all traded event details
        for (idx, event) in events.iter().enumerate() {
            let event_id = event_ids[idx];

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
                .context("Failed to insert Orca Whirlpool traded event in batch")?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(event_ids)
    }

    async fn batch_insert_liquidity_increased_events(
        &self,
        events: Vec<OrcaWhirlpoolLiquidityIncreasedEventRecord>
    ) -> Result<Vec<i32>> {
        // Early return if there are no events to process
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = Repository::pool(self).begin().await?;

        // Extract base events
        let base_events: Vec<OrcaWhirlpoolEvent> = events
            .iter()
            .map(|event| event.base.clone())
            .collect();

        // Insert all base events in batch
        let event_ids = self.batch_insert_base_events(&mut tx, &base_events).await?;

        // Insert all liquidity increased event details
        for (idx, event) in events.iter().enumerate() {
            let event_id = event_ids[idx];

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
                .context("Failed to insert Orca Whirlpool liquidity increased event in batch")?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(event_ids)
    }

    async fn batch_insert_liquidity_decreased_events(
        &self,
        events: Vec<OrcaWhirlpoolLiquidityDecreasedEventRecord>
    ) -> Result<Vec<i32>> {
        // Early return if there are no events to process
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = Repository::pool(self).begin().await?;

        // Extract base events
        let base_events: Vec<OrcaWhirlpoolEvent> = events
            .iter()
            .map(|event| event.base.clone())
            .collect();

        // Insert all base events in batch
        let event_ids = self.batch_insert_base_events(&mut tx, &base_events).await?;

        // Insert all liquidity decreased event details
        for (idx, event) in events.iter().enumerate() {
            let event_id = event_ids[idx];

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
                .context("Failed to insert Orca Whirlpool liquidity decreased event in batch")?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(event_ids)
    }
}
