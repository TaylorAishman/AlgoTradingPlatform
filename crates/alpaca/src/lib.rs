//! Thin blocking REST client for the Alpaca paper trading and market data APIs.

pub mod account;
pub mod activities;
pub mod auth;
pub mod data;
pub mod endpoints;
pub mod error;
pub mod http;
pub mod limiter;
pub mod orders_read;
pub mod orders_write;
pub mod reference;
