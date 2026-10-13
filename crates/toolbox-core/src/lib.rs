//! Toolbox's tool logic. Each tool is a module with no GUI code, so it can be unit-tested
//! and reused by any frontend.

pub mod case;
pub mod cron;
mod data;
pub mod encoding;
pub mod fake;
pub mod formats;
pub mod hash;
pub mod ids;
pub mod jwt;
pub mod lorem;
pub mod names;
pub mod number_base;
pub mod passphrase;
pub mod password;
pub mod qr;
pub mod strength;
pub mod timestamp;
