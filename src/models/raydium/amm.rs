/******************************************************************************
 * IMPORTANT: DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
 *
 * This file is protected and should not be modified without explicit approval.
 * Any changes could break the indexer functionality.
 *
 * See .nooverwrite.json for more information on protected files.
 ******************************************************************************/

use chrono::{ DateTime, Utc };
use borsh::BorshDeserialize;
use serde::{ Deserialize, Serialize };
use sqlx::FromRow;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Orca Whirlpool event discriminators
pub const TRADED_EVENT_DISCRIMINATOR: [u8; 8] = [225, 202, 73, 175, 147, 43, 160, 150];
pub const LIQUIDITY_INCREASED_DISCRIMINATOR: [u8; 8] = [30, 7, 144, 181, 102, 254, 155, 161];
pub const LIQUIDITY_DECREASED_DISCRIMINATOR: [u8; 8] = [166, 1, 36, 71, 112, 202, 181, 171];

/// Types of events emitted by Orca Whirlpool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OrcaWhirlpoolEventType {
    Traded,
    LiquidityIncreased,
    LiquidityDecreased,
}

impl ToString for OrcaWhirlpoolEventType {
    fn to_string(&self) -> String {
        match self {
            OrcaWhirlpoolEventType::Traded => "Traded".to_string(),
            OrcaWhirlpoolEventType::LiquidityIncreased => "LiquidityIncreased".to_string(),
            OrcaWhirlpoolEventType::LiquidityDecreased => "LiquidityDecreased".to_string(),
        }
    }
}

impl FromStr for OrcaWhirlpoolEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Traded" => Ok(OrcaWhirlpoolEventType::Traded),
            "LiquidityIncreased" => Ok(OrcaWhirlpoolEventType::LiquidityIncreased),
            "LiquidityDecreased" => Ok(OrcaWhirlpoolEventType::LiquidityDecreased),
            _ => Err(format!("Unknown Orca Whirlpool event type: {}", s)),
        }
    }
}

// On-chain event structures (as deserialized from Solana transactions)
#[derive(BorshDeserialize, Debug)]
pub struct OrcaWhirlpoolPoolInitializedEvent {
    pub whirlpool: Pubkey,
    pub whirlpools_config: Pubkey,
    pub token_mint_a: Pubkey,
    pub token_mint_b: Pubkey,
    pub tick_spacing: u16,
    pub token_program_a: Pubkey,
    pub token_program_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub initial_sqrt_price: u128,
}

#[derive(BorshDeserialize, Debug)]
pub struct OrcaWhirlpoolTradedEvent {
    pub whirlpool: Pubkey,
    pub a_to_b: bool,
    pub pre_sqrt_price: u128,
    pub post_sqrt_price: u128,
    pub input_amount: u64,
    pub output_amount: u64,
    pub input_transfer_fee: u64,
    pub output_transfer_fee: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
}

// IMPORTANT: LiquidityIncreased and LiquidityDecreased events must remain separate structures
// even if their fields are identical. This allows for future divergence in their implementations
// and ensures proper semantic distinction between the two operations.

/// Event emitted when liquidity is increased in a whirlpool
#[derive(BorshDeserialize, Debug)]
pub struct OrcaWhirlpoolLiquidityIncreasedEvent {
    pub whirlpool: Pubkey,
    pub position: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub token_a_transfer_fee: u64,
    pub token_b_transfer_fee: u64,
}

/// Event emitted when liquidity is decreased in a whirlpool
#[derive(BorshDeserialize, Debug)]
pub struct OrcaWhirlpoolLiquidityDecreasedEvent {
    pub whirlpool: Pubkey,
    pub position: Pubkey,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub token_a_transfer_fee: u64,
    pub token_b_transfer_fee: u64,
}

// Database models matching the orca_whirlpool_events table

// Database models matching the orca_whirlpool_events table

#[derive(Debug, Clone, FromRow)]
pub struct OrcaWhirlpoolEvent {
    pub id: i32,
    pub signature: String,
    pub whirlpool: String,
    pub event_type: String,
    pub version: i32,
    pub timestamp: DateTime<Utc>,
}

impl OrcaWhirlpoolEvent {
    pub fn new(signature: String, whirlpool: Pubkey, event_type: OrcaWhirlpoolEventType) -> Self {
        Self {
            id: 0, // Will be set by the database
            signature,
            whirlpool: whirlpool.to_string(),
            event_type: event_type.to_string(),
            version: 1,
            timestamp: Utc::now(),
        }
    }
}

// Database models for specific event types

#[derive(Debug, Clone, FromRow)]
pub struct OrcaWhirlpoolTradedRecord {
    pub event_id: i32,
    pub a_to_b: bool,
    pub pre_sqrt_price: i64,
    pub post_sqrt_price: i64,
    pub input_amount: i64,
    pub output_amount: i64,
    pub input_transfer_fee: i64,
    pub output_transfer_fee: i64,
    pub lp_fee: i64,
    pub protocol_fee: i64,
}

// IMPORTANT: LiquidityIncreasedRecord and LiquidityDecreasedRecord must remain separate structures
// even if their fields are identical. This allows for future divergence in their implementations,
// ensures proper semantic distinction, and reflects the fact that they are stored in different database tables.

#[derive(Debug, Clone, FromRow)]
pub struct OrcaWhirlpoolLiquidityIncreasedRecord {
    pub event_id: i32,
    pub position: String,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: i64,
    pub token_a_amount: i64,
    pub token_b_amount: i64,
    pub token_a_transfer_fee: i64,
    pub token_b_transfer_fee: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct OrcaWhirlpoolLiquidityDecreasedRecord {
    pub event_id: i32,
    pub position: String,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: i64,
    pub token_a_amount: i64,
    pub token_b_amount: i64,
    pub token_a_transfer_fee: i64,
    pub token_b_transfer_fee: i64,
}

// Legacy record structure for backwards compatibility with existing code
// DO NOT USE IN NEW CODE - Use OrcaWhirlpoolLiquidityIncreasedRecord or OrcaWhirlpoolLiquidityDecreasedRecord instead
//
// COMPATIBILITY NOTICE: This structure exists for backward compatibility with the indexer code
// that uses a single record structure for both liquidity increase and decrease events. Future code
// should use the separate record structures above to properly distinguish between event types.
#[derive(Debug, Clone, FromRow)]
pub struct OrcaWhirlpoolLiquidityRecord {
    pub event_id: i32,
    pub position: String,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: i64,
    pub token_a_amount: i64,
    pub token_b_amount: i64,
    pub token_a_transfer_fee: i64,
    pub token_b_transfer_fee: i64,
}

// Composite types for inserting events with their specific data

#[derive(Debug)]
pub struct OrcaWhirlpoolTradedEventRecord {
    pub base: OrcaWhirlpoolEvent,
    pub data: OrcaWhirlpoolTradedRecord,
}

#[derive(Debug)]
pub struct OrcaWhirlpoolLiquidityIncreasedEventRecord {
    pub base: OrcaWhirlpoolEvent,
    pub data: OrcaWhirlpoolLiquidityRecord,
}

#[derive(Debug)]
pub struct OrcaWhirlpoolLiquidityDecreasedEventRecord {
    pub base: OrcaWhirlpoolEvent,
    pub data: OrcaWhirlpoolLiquidityRecord,
}
