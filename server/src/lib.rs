/// Re-export internal modules for integration test access.
///
/// Integration tests in `server/tests/` are compiled as separate crates
/// and can only access public items from the `server` crate's lib target.
/// This file makes `server::auth`, `server::db`, `server::net`, and
/// `server::session` available to those tests.
pub mod auth;
pub mod character;
pub mod config;
pub mod db;
pub mod net;
pub mod session;
pub mod world;
