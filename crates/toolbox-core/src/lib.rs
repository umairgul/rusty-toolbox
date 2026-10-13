//! Toolbox's tool logic. Each tool is a module with no GUI code, so it can be unit-tested
//! and reused by any frontend.

pub mod case;
pub mod color;
pub mod cron;
mod data;
pub mod diff;
pub mod encoding;
pub mod fake;
pub mod formats;
pub mod hash;
pub mod ids;
pub mod json_format;
pub mod jwt;
pub mod lorem;
pub mod markdown;
pub mod names;
pub mod number_base;
pub mod passphrase;
pub mod password;
pub mod qr;
pub mod regex_test;
pub mod sql;
pub mod strength;
pub mod timestamp;
