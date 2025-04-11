use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use chrono::Utc;

// Import the event type and pool from the public API
use indexer::{ OrcaWhirlpoolEventType, OrcaWhirlpoolPoolRecord };

// Import the database models directly from the modules
use indexer::models::orca::whirlpool::{
    OrcaWhirlpoolEvent,
    OrcaWhirlpoolTradedRecord,
    OrcaWhirlpoolLiquidityRecord,
    OrcaWhirlpoolTradedEventRecord,
    OrcaWhirlpoolLiquidityIncreasedEventRecord,
    OrcaWhirlpoolLiquidityDecreasedEventRecord,
};

// Test the creation and properties of base event records
#[test]
fn test_base_event_creation() {
    // Create a base event
    let base_event = OrcaWhirlpoolEvent {
        id: 0, // Will be set by database
        signature: "test_signature".to_string(),
        whirlpool: "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string(),
        event_type: OrcaWhirlpoolEventType::Traded.to_string(),
        version: 1,
        timestamp: Utc::now(),
    };

    // Verify the properties
    assert_eq!(base_event.id, 0);
    assert_eq!(base_event.signature, "test_signature");
    assert_eq!(base_event.whirlpool, "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");
    assert_eq!(base_event.event_type, "Traded");
    assert_eq!(base_event.version, 1);
}

// Test the creation of a traded event record
#[test]
fn test_traded_event_record() {
    // Create a base event
    let base_event = OrcaWhirlpoolEvent {
        id: 1, // Simulating database ID
        signature: "test_traded_signature".to_string(),
        whirlpool: "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string(),
        event_type: OrcaWhirlpoolEventType::Traded.to_string(),
        version: 1,
        timestamp: Utc::now(),
    };

    // Create the traded record data
    let data = OrcaWhirlpoolTradedRecord {
        event_id: 1, // Matching the base event ID
        a_to_b: true,
        pre_sqrt_price: 1000000,
        post_sqrt_price: 1010000,
        input_amount: 100,
        output_amount: 95,
        input_transfer_fee: 1,
        output_transfer_fee: 1,
        lp_fee: 3,
        protocol_fee: 1,
    };

    // Create the combined record
    let event_record = OrcaWhirlpoolTradedEventRecord {
        base: base_event.clone(),
        data: data.clone(),
    };

    // Verify the properties
    assert_eq!(event_record.base.id, base_event.id);
    assert_eq!(event_record.base.signature, base_event.signature);
    assert_eq!(event_record.data.event_id, 1);
    assert_eq!(event_record.data.a_to_b, true);
    assert_eq!(event_record.data.input_amount, 100);
    assert_eq!(event_record.data.output_amount, 95);
}

// Test the creation of a liquidity increased event record
#[test]
fn test_liquidity_increased_event_record() {
    // Create a base event
    let base_event = OrcaWhirlpoolEvent {
        id: 2, // Simulating database ID
        signature: "test_liq_inc_signature".to_string(),
        whirlpool: "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string(),
        event_type: OrcaWhirlpoolEventType::LiquidityIncreased.to_string(),
        version: 1,
        timestamp: Utc::now(),
    };

    // Create the liquidity record data
    let data = OrcaWhirlpoolLiquidityRecord {
        event_id: 2, // Matching the base event ID
        position: "3puktQ8QwKUXskgvz9k7poxMgqHe6bmRFQJaSzBvc4uN".to_string(),
        tick_lower_index: -100,
        tick_upper_index: 100,
        liquidity: 5000,
        token_a_amount: 200,
        token_b_amount: 300,
        token_a_transfer_fee: 1,
        token_b_transfer_fee: 1,
    };

    // Create the combined record
    let event_record = OrcaWhirlpoolLiquidityIncreasedEventRecord {
        base: base_event.clone(),
        data: data.clone(),
    };

    // Verify the properties
    assert_eq!(event_record.base.id, base_event.id);
    assert_eq!(event_record.base.signature, base_event.signature);
    assert_eq!(event_record.base.event_type, "LiquidityIncreased");
    assert_eq!(event_record.data.event_id, 2);
    assert_eq!(event_record.data.position, "3puktQ8QwKUXskgvz9k7poxMgqHe6bmRFQJaSzBvc4uN");
    assert_eq!(event_record.data.tick_lower_index, -100);
    assert_eq!(event_record.data.tick_upper_index, 100);
    assert_eq!(event_record.data.liquidity, 5000);
    assert_eq!(event_record.data.token_a_amount, 200);
    assert_eq!(event_record.data.token_b_amount, 300);
}

// Test the creation of a liquidity decreased event record
#[test]
fn test_liquidity_decreased_event_record() {
    // Create a base event
    let base_event = OrcaWhirlpoolEvent {
        id: 3, // Simulating database ID
        signature: "test_liq_dec_signature".to_string(),
        whirlpool: "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string(),
        event_type: OrcaWhirlpoolEventType::LiquidityDecreased.to_string(),
        version: 1,
        timestamp: Utc::now(),
    };

    // Create the liquidity record data
    let data = OrcaWhirlpoolLiquidityRecord {
        event_id: 3, // Matching the base event ID
        position: "3puktQ8QwKUXskgvz9k7poxMgqHe6bmRFQJaSzBvc4uN".to_string(),
        tick_lower_index: -100,
        tick_upper_index: 100,
        liquidity: 3000, // Less liquidity than before
        token_a_amount: 120,
        token_b_amount: 180,
        token_a_transfer_fee: 1,
        token_b_transfer_fee: 1,
    };

    // Create the combined record
    let event_record = OrcaWhirlpoolLiquidityDecreasedEventRecord {
        base: base_event.clone(),
        data: data.clone(),
    };

    // Verify the properties
    assert_eq!(event_record.base.id, base_event.id);
    assert_eq!(event_record.base.signature, base_event.signature);
    assert_eq!(event_record.base.event_type, "LiquidityDecreased");
    assert_eq!(event_record.data.event_id, 3);
    assert_eq!(event_record.data.position, "3puktQ8QwKUXskgvz9k7poxMgqHe6bmRFQJaSzBvc4uN");
    assert_eq!(event_record.data.liquidity, 3000);
    assert_eq!(event_record.data.token_a_amount, 120);
    assert_eq!(event_record.data.token_b_amount, 180);
}

// Test the OrcaWhirlpoolPool model
#[test]
fn test_orca_whirlpool_pool() {
    // Default SOL/USDC pool
    let pool = OrcaWhirlpoolPoolRecord {
        whirlpool: "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string(),
        token_mint_a: "So11111111111111111111111111111111111111112".to_string(), // SOL
        token_mint_b: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
        token_name_a: Some("SOL".to_string()),
        token_name_b: Some("USDC".to_string()),
        pool_name: Some("SOL/USDC".to_string()),
        decimals_a: 9,
        decimals_b: 6,
    };

    // Verify the properties
    assert_eq!(pool.whirlpool, "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");
    assert_eq!(pool.token_mint_a, "So11111111111111111111111111111111111111112");
    assert_eq!(pool.token_mint_b, "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    assert_eq!(pool.token_name_a, Some("SOL".to_string()));
    assert_eq!(pool.token_name_b, Some("USDC".to_string()));
    assert_eq!(pool.pool_name, Some("SOL/USDC".to_string()));
    assert_eq!(pool.decimals_a, 9);
    assert_eq!(pool.decimals_b, 6);
}

// Test comparing pubkeys from different sources
#[test]
fn test_pubkey_comparisons() {
    // The SOL mint
    let sol_mint_str = "So11111111111111111111111111111111111111112";
    let sol_mint_pubkey = Pubkey::from_str(sol_mint_str).unwrap();

    // Convert back to string and parse again
    let sol_mint_str2 = sol_mint_pubkey.to_string();
    let sol_mint_pubkey2 = Pubkey::from_str(&sol_mint_str2).unwrap();

    // They should be equal
    assert_eq!(sol_mint_str, sol_mint_str2);
    assert_eq!(sol_mint_pubkey, sol_mint_pubkey2);

    // Using different pubkeys
    let usdc_mint_str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let usdc_mint_pubkey = Pubkey::from_str(usdc_mint_str).unwrap();

    // Different pubkeys should not be equal
    assert_ne!(sol_mint_pubkey, usdc_mint_pubkey);
    assert_ne!(sol_mint_str, usdc_mint_str);
}

// Test conversions between pubkeys and strings
#[test]
fn test_pubkey_string_conversions() {
    // Default Orca pool
    let pool_str = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE";
    let pool_pubkey = Pubkey::from_str(pool_str).unwrap();

    // Test string conversion
    let pool_str2 = pool_pubkey.to_string();
    assert_eq!(pool_str, pool_str2);

    // Test pubkey parsing from string
    let pool_pubkey2 = Pubkey::from_str(&pool_str2).unwrap();
    assert_eq!(pool_pubkey, pool_pubkey2);

    // Test invalid pubkey
    let invalid_pubkey = Pubkey::from_str("invalid");
    assert!(invalid_pubkey.is_err());
}
