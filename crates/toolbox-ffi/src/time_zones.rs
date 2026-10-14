//! Time zone names, shared by the tools that show times (Timestamps, Cron).

use toolbox_core::timestamp;

/// Every IANA time zone name, sorted.
#[uniffi::export]
pub fn time_zone_names() -> Vec<String> {
    timestamp::zone_names().to_vec()
}

/// This Mac's time zone, if it has an IANA name.
#[uniffi::export]
pub fn time_zone_system() -> Option<String> {
    timestamp::system_zone_name()
}
