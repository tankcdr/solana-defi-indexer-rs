use serde::{ Serialize, Deserialize };
use std::str::FromStr;

// Raydium CLMM event discriminators
pub const CLMM_CREATE_PERSONAL_POSITION_DISCRIMINATOR: [u8; 8] = [
    226, 245, 162, 196, 229, 232, 248, 211,
];
pub const CLMM_LIQUIDITY_INCREASED_DISCRIMINATOR: [u8; 8] = [
    200, 185, 247, 226, 211, 165, 182, 193,
];
pub const CLMM_LIQUIDITY_DECREASED_DISCRIMINATOR: [u8; 8] = [93, 127, 154, 27, 44, 62, 77, 95];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RaydiumCLMMEventType {
    CreatePosition,
    IncreaseLiquidity,
    DecreaseLiquidity,
}

impl ToString for RaydiumCLMMEventType {
    fn to_string(&self) -> String {
        match self {
            RaydiumCLMMEventType::CreatePosition => "CreatePosition".to_string(),
            RaydiumCLMMEventType::IncreaseLiquidity => "IncreaseLiquidity".to_string(),
            RaydiumCLMMEventType::DecreaseLiquidity => "DecreaseLiquidity".to_string(),
        }
    }
}

impl FromStr for RaydiumCLMMEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CreatePosition" => Ok(RaydiumCLMMEventType::CreatePosition),
            "IncreaseLiquidity" => Ok(RaydiumCLMMEventType::IncreaseLiquidity),
            "DecreaseLiquidity" => Ok(RaydiumCLMMEventType::DecreaseLiquidity),
            _ => Err(format!("Unknown Raydium CLMM event type: {}", s)),
        }
    }
}

#[derive(BorshDeserialize, Debug)]
pub struct RaydiumCLMMCreatePositionEvent {
    /// The pool for which liquidity was added
    pub pool_state: Pubkey,
    /// The address that create the position
    pub minter: Pubkey,
    /// The owner of the position and recipient of any minted liquidity
    pub nft_owner: Pubkey,
    /// The lower tick of the position
    pub tick_lower_index: i32,
    /// The upper tick of the position
    pub tick_upper_index: i32,
    /// The amount of liquidity minted to the position range
    pub liquidity: u128,
    /// The amount of token_0 was deposit for the liquidity
    pub deposit_amount_0: u64,
    /// The amount of token_1 was deposit for the liquidity
    pub deposit_amount_1: u64,
    /// The token transfer fee for deposit_amount_0
    pub deposit_amount_0_transfer_fee: u64,
    /// The token transfer fee for deposit_amount_1
    pub deposit_amount_1_transfer_fee: u64,
}

#[derive(BorshDeserialize, Debug)]
pub struct RaydiumCLMMIncreaseLiquidityEvent {
    /// The ID of the token for which liquidity was increased
    pub position_nft_mint: Pubkey,
    /// The amount by which liquidity for the NFT position was increased
    pub liquidity: u128,
    /// The amount of token_0 that was paid for the increase in liquidity
    pub amount_0: u64,
    /// The amount of token_1 that was paid for the increase in liquidity
    pub amount_1: u64,
    /// The token transfer fee for amount_0
    pub amount_0_transfer_fee: u64,
    /// The token transfer fee for amount_1
    pub amount_1_transfer_fee: u64,
}

#[derive(BorshDeserialize, Debug)]
pub struct RaydiumCLMMDecreaseLiquidityEvent {
    /// The ID of the token for which liquidity was decreased
    pub position_nft_mint: Pubkey,
    /// The amount by which liquidity for the position was decreased
    pub liquidity: u128,
    /// The amount of token_0 that was paid for the decrease in liquidity
    pub decrease_amount_0: u64,
    /// The amount of token_1 that was paid for the decrease in liquidity
    pub decrease_amount_1: u64,
    // The amount of token_0 fee
    pub fee_amount_0: u64,
    /// The amount of token_1 fee
    pub fee_amount_1: u64,
    /// The amount of rewards
    pub reward_amounts: [u64; 3],
    /// The amount of token_0 transfer fee
    pub transfer_fee_0: u64,
    /// The amount of token_1 transfer fee
    pub transfer_fee_1: u64,
}

#[derive(Debug, Clone, FromRow)]
pub struct RaydiumCLMMEvent {
    pub id: i32, // Auto-incremented by DB
    pub signature: String, // Transaction signature
    pub pool: String, // Pool address as string
    pub event_type: String, // Event type as string
    pub version: i32, // For schema versioning
    pub timestamp: DateTime<Utc>, // Event timestamp
}

impl RaydiumCLMMEvent {
    pub fn new(signature: String, pool: Pubkey, event_type: RaydiumCLMMEventType) -> Self {
        Self {
            id: 0, // Set by DB
            signature,
            pool: pool.to_string(),
            event_type: event_type.to_string(),
            version: 1,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct RaydiumCLMMCreatePositionRecord {
    pub event_id: i32,
    pub minter: String,
    pub nft_owner: String,
    pub output_amount: i64,
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
    pub liquidity: u128,
    pub deposit_amount_0: u64,
    pub deposit_amount_1: u64,
    pub deposit_amount_0_transfer_fee: u64,
    pub deposit_amount_1_transfer_fee: u64,
}

#[derive(Debug, Clone, FromRow)]
pub struct RaydiumCLMMIncreaseLiquidityRecord {
    pub event_id: i32,
    pub position_nft_mint: Pubkey,
    pub liquidity: u128,
    pub amount_0: u64,
    pub amount_1: u64,
    pub amount_0_transfer_fee: u64,
    pub amount_1_transfer_fee: u64,
}

#[derive(Debug, Clone, FromRow)]
pub struct RaydiumCLMMDecreaseLiquidityRecord {
    pub event_id: i32,
    pub position_nft_mint: Pubkey,
    pub liquidity: u128,
    pub decrease_amount_0: u64,
    pub decrease_amount_1: u64,
    pub fee_amount_0: u64,
    pub fee_amount_1: u64,
    pub reward_amounts: [u64; 3],
    pub transfer_fee_0: u64,
    pub transfer_fee_1: u64,
}

// Composite types for inserting events with their specific data
#[derive(Debug)]
pub enum RaydiumCLMMEventRecord {
    CreatePosition(RaydiumCLMMCreatePositionEvent),
    IncreaseLiquidity(RaydiumCLMMIncreaseLiquidityEventRecord),
    DecreaseLiquidity(RaydiumCLMMDecreaseLiquidityEventRecord),
}

#[derive(Debug)]
pub struct RaydiumCLMMCreatePostionEventRecord {
    pub base: RaydiumCLMMEvent,
    pub data: RaydiumCLMMCreatePositionRecord,
}

#[derive(Debug)]
pub struct RaydiumCLMMIncreaseLiquidityEventRecord {
    pub base: RaydiumCLMMEvent,
    pub data: RaydiumCLMMIncreaseLiquidityRecord,
}

#[derive(Debug)]
pub struct RaydiumCLMMDecreaseLiquidityEventRecord {
    pub base: RaydiumCLMMEvent,
    pub data: RaydiumCLMMDecreaseLiquidityRecord,
}
