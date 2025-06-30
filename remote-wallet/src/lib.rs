#![allow(clippy::arithmetic_side_effects)]
#![allow(dead_code)]

// 新的分层架构
// pub mod api;
// pub mod wallet_impl;
// pub mod transport;
// pub mod protocol;

// 现有模块（保持向后兼容）
pub mod ledger;
pub mod ledger_error;
pub mod locator;
pub mod remote_keypair;
pub mod remote_wallet;
pub mod errors;
