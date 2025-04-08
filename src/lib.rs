// Re-export core modules
pub mod models;
pub mod db;
pub mod indexers;
pub mod websocket_manager;
pub mod backfill_manager;

// Re-export common types and traits
pub use models::common::Protocol;
// DexEvent no longer exists as noted in models/common.rs
pub use db::{ Database, DbConfig };

// Re-export protocol-specific components
pub use models::orca::whirlpool::{
    TRADED_EVENT_DISCRIMINATOR,
    LIQUIDITY_INCREASED_DISCRIMINATOR,
    LIQUIDITY_DECREASED_DISCRIMINATOR,
    OrcaWhirlpoolEventType,
    OrcaWhirlpoolTradedEvent,
    OrcaWhirlpoolLiquidityIncreasedEvent,
    OrcaWhirlpoolLiquidityDecreasedEvent,
};

pub use db::repositories::{ OrcaWhirlpoolRepository, OrcaWhirlpoolPoolRepository };
pub use indexers::OrcaWhirlpoolIndexer;

pub use websocket_manager::{ WebSocketManager, WebSocketConfig };
pub use backfill_manager::{ BackfillManager, BackfillConfig };
pub use db::signature_store::SignatureStore;
