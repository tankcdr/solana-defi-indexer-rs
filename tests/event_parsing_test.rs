use solana_client::rpc_response::RpcLogsResponse;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use indexer::{
    TRADED_EVENT_DISCRIMINATOR,
    LIQUIDITY_INCREASED_DISCRIMINATOR,
    LIQUIDITY_DECREASED_DISCRIMINATOR,
    OrcaWhirlpoolEventType,
};

// Mock function to build a log that contains a program mention
fn build_program_log(program_id: &str, contains_event: bool) -> RpcLogsResponse {
    let mut logs = vec![
        format!("Program {} invoke [1]", program_id),
        "Program log: Some operation".to_string()
    ];

    if contains_event {
        logs.push("Program log: Swap".to_string());
    }

    RpcLogsResponse {
        signature: "mock_signature".to_string(),
        err: None,
        logs,
    }
}

// This test verifies the logic for detecting interesting events by keywords
#[test]
fn test_event_keyword_detection() {
    // Mock function similar to the contains_event_keywords in DexIndexer trait
    fn contains_event_keywords(log: &RpcLogsResponse, keywords: &[&str]) -> bool {
        log.logs.iter().any(|line| { keywords.iter().any(|&keyword| line.contains(keyword)) })
    }

    // Create mock logs with and without event keywords
    let whirlpool_program = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
    let log_with_event = build_program_log(whirlpool_program, true);
    let log_without_event = build_program_log(whirlpool_program, false);

    // Event keywords to search for
    let keywords = ["Swap", "IncreaseLiquidity", "DecreaseLiquidity"];

    // Test the detection
    assert!(contains_event_keywords(&log_with_event, &keywords));
    assert!(!contains_event_keywords(&log_without_event, &keywords));
}

// Build mock binary data for testing
fn build_mock_event_data(discriminator: &[u8; 8], additional_data: &[u8]) -> Vec<u8> {
    let mut data = discriminator.to_vec();
    data.extend_from_slice(additional_data);
    data
}

// Test the discriminator matching logic
#[test]
fn test_discriminator_matching() {
    // Mock function similar to the one in DexIndexer trait
    fn matches_discriminator(data: &[u8], discriminator: &[u8; 8]) -> bool {
        data.len() >= 8 && &data[0..8] == discriminator
    }

    // Create mock data for different event types
    let traded_data = build_mock_event_data(&TRADED_EVENT_DISCRIMINATOR, &[0, 1, 2, 3]);
    let liquidity_increased_data = build_mock_event_data(
        &LIQUIDITY_INCREASED_DISCRIMINATOR,
        &[0, 1, 2, 3]
    );
    let liquidity_decreased_data = build_mock_event_data(
        &LIQUIDITY_DECREASED_DISCRIMINATOR,
        &[0, 1, 2, 3]
    );
    let invalid_data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]; // Doesn't match any discriminator

    // Test discriminator matching
    assert!(matches_discriminator(&traded_data, &TRADED_EVENT_DISCRIMINATOR));
    assert!(matches_discriminator(&liquidity_increased_data, &LIQUIDITY_INCREASED_DISCRIMINATOR));
    assert!(matches_discriminator(&liquidity_decreased_data, &LIQUIDITY_DECREASED_DISCRIMINATOR));

    // Test negative cases
    assert!(!matches_discriminator(&traded_data, &LIQUIDITY_INCREASED_DISCRIMINATOR));
    assert!(!matches_discriminator(&liquidity_increased_data, &LIQUIDITY_DECREASED_DISCRIMINATOR));
    assert!(!matches_discriminator(&invalid_data, &TRADED_EVENT_DISCRIMINATOR));

    // Test with too short data
    let short_data = vec![1, 2, 3]; // Less than 8 bytes
    assert!(!matches_discriminator(&short_data, &TRADED_EVENT_DISCRIMINATOR));
}

// Mock parsing the base64 data from log lines
#[test]
fn test_program_data_parsing() {
    // Create base64 encoded program data for each discriminator
    let encode_to_base64 = |data: &[u8]| -> String { STANDARD.encode(data) };

    // Encode sample data with discriminators
    let traded_data = build_mock_event_data(&TRADED_EVENT_DISCRIMINATOR, &[0, 1, 2, 3]);
    let liquidity_increased_data = build_mock_event_data(
        &LIQUIDITY_INCREASED_DISCRIMINATOR,
        &[0, 1, 2, 3]
    );
    let liquidity_decreased_data = build_mock_event_data(
        &LIQUIDITY_DECREASED_DISCRIMINATOR,
        &[0, 1, 2, 3]
    );

    let traded_base64 = encode_to_base64(&traded_data);
    let liquidity_increased_base64 = encode_to_base64(&liquidity_increased_data);
    let liquidity_decreased_base64 = encode_to_base64(&liquidity_decreased_data);

    // Create log lines with the encoded data
    let traded_log = format!("Program log: Program data: {}", traded_base64);
    let liquidity_increased_log =
        format!("Program log: Program data: {}", liquidity_increased_base64);
    let liquidity_decreased_log =
        format!("Program log: Program data: {}", liquidity_decreased_base64);

    // Extract function (same as in the previous test)
    fn extract_event_data(log_line: &str) -> Option<Vec<u8>> {
        let parts: Vec<&str> = log_line.split("Program data: ").collect();
        if parts.len() >= 2 {
            if let Ok(decoded) = STANDARD.decode(parts[1]) {
                return Some(decoded);
            }
        }
        None
    }

    // Test extraction and discriminator matching
    let extracted_traded = extract_event_data(&traded_log).unwrap();
    let extracted_increased = extract_event_data(&liquidity_increased_log).unwrap();
    let extracted_decreased = extract_event_data(&liquidity_decreased_log).unwrap();

    assert_eq!(&extracted_traded[0..8], &TRADED_EVENT_DISCRIMINATOR[..]);
    assert_eq!(&extracted_increased[0..8], &LIQUIDITY_INCREASED_DISCRIMINATOR[..]);
    assert_eq!(&extracted_decreased[0..8], &LIQUIDITY_DECREASED_DISCRIMINATOR[..]);
}

// Test the event type handling
#[test]
fn test_event_type_handling() {
    let event_types = vec![
        OrcaWhirlpoolEventType::Traded,
        OrcaWhirlpoolEventType::LiquidityIncreased,
        OrcaWhirlpoolEventType::LiquidityDecreased
    ];

    // Test roundtrip conversion (to_string -> from_str)
    for event_type in event_types {
        let event_str = event_type.to_string();
        let round_trip = OrcaWhirlpoolEventType::from_str(&event_str).unwrap();
        assert_eq!(event_type, round_trip);
    }
}

// Test pool pubkey monitoring
#[test]
fn test_pool_pubkey_validation() {
    // We can parse pubkeys from different string formats
    let default_orca_pool = Pubkey::from_str(
        "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"
    ).unwrap();
    let pool_from_base58 = Pubkey::from_str(
        "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"
    ).unwrap();

    // Test equality
    assert_eq!(default_orca_pool, pool_from_base58);

    // Test different formats for the same key
    let as_base58 = default_orca_pool.to_string();
    let reparsed = Pubkey::from_str(&as_base58).unwrap();
    assert_eq!(default_orca_pool, reparsed);
}

// Mock a complete log with event data
fn create_mock_trade_log() -> RpcLogsResponse {
    // Create a simple mock of Traded event data
    // This would normally include the full trading fields
    let mut event_data = TRADED_EVENT_DISCRIMINATOR.to_vec();

    // Add some dummy data to represent the event fields
    // In a real scenario, this would be serialized as per Borsh format
    let dummy_data = vec![0u8; 256]; // Just some placeholder data
    event_data.extend_from_slice(&dummy_data);

    // Encode the event data
    let base64_data = STANDARD.encode(&event_data);

    // Create a mock log with the Orca program ID and encoded event data
    RpcLogsResponse {
        signature: "mock_trade_signature".to_string(),
        err: None,
        logs: vec![
            "Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]".to_string(),
            "Program log: Instruction: Swap".to_string(),
            format!("Program log: Program data: {}", base64_data),
            "Program log: Swap successful".to_string()
        ],
    }
}

// Test for a mock trade log scenario
#[test]
fn test_mock_trade_log_detection() {
    let mock_log = create_mock_trade_log();

    // Check if the log contains program mentions
    let whirlpool_program = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
    let contains_program = mock_log.logs.iter().any(|line| line.contains(whirlpool_program));
    assert!(contains_program);

    // Check if it contains event keywords
    let keywords = ["Swap", "IncreaseLiquidity", "DecreaseLiquidity"];
    let contains_keywords = mock_log.logs
        .iter()
        .any(|line| { keywords.iter().any(|&keyword| line.contains(keyword)) });
    assert!(contains_keywords);

    // Check if we can extract program data
    let program_data_line = mock_log.logs
        .iter()
        .find(|line| line.contains("Program data:"))
        .unwrap();

    let extracted_data = {
        let parts: Vec<&str> = program_data_line.split("Program data: ").collect();
        STANDARD.decode(parts[1]).unwrap()
    };

    // Verify the discriminator
    assert_eq!(&extracted_data[0..8], &TRADED_EVENT_DISCRIMINATOR[..]);
}
