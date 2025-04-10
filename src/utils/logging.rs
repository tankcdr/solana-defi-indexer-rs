use chrono;
use std::fmt::Debug;

/// Standard format for activity logs: [timestamp] component - message: details
pub fn log_activity(component: &str, message: &str, details: Option<&str>) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let details_str = details.unwrap_or("");
    println!("[{}] {} - {}: {}", timestamp, component, message, details_str);
}

/// Standard format for error logs: [timestamp] component - ERROR: message
pub fn log_error(component: &str, context: &str, err: &anyhow::Error) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let error_message = format!("ERROR - {}: {}", context, err);
    eprintln!("[{}] {} - {}", timestamp, component, error_message);
}

/// Log statistics with standard format
pub fn log_stats(component: &str, context: &str, stats: &str) {
    log_activity(component, context, Some(stats));
}

/// Log debug information
pub fn log_debug<T: Debug>(component: &str, context: &str, details: &T) {
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("[{}] {} - Details: {:?}", component, context, details);
    }
}

/// Enhanced format for activity logs with DEX name: [timestamp] component (dex) - message: details
pub fn log_dex_activity(component: &str, dex: &str, message: &str, details: Option<&str>) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let details_str = details.unwrap_or("");
    println!("[{}] {} ({}) - {}: {}", timestamp, component, dex, message, details_str);
}

/// Enhanced format for error logs with DEX name: [timestamp] component (dex) - ERROR: message
pub fn log_dex_error(component: &str, dex: &str, context: &str, err: &anyhow::Error) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let error_message = format!("ERROR - {}: {}", context, err);
    eprintln!("[{}] {} ({}) - {}", timestamp, component, dex, error_message);
}

/// Enhanced format for statistics logs with DEX name: [timestamp] component (dex) - context: stats
pub fn log_dex_stats(component: &str, dex: &str, context: &str, stats: &str) {
    log_dex_activity(component, dex, context, Some(stats));
}
