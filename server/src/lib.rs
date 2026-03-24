/// Re-export internal modules for integration test access.
///
/// Integration tests in `server/tests/` are compiled as separate crates
/// and can only access public items from the `server` crate's lib target.
/// This file makes `server::auth`, `server::db`, `server::net`, and
/// `server::session` available to those tests.
pub mod auth;
pub mod character;
pub mod combat;
pub mod config;
pub mod db;
pub mod dungeon;
pub mod inventory;
pub mod net;
pub mod session;
pub mod social;
pub mod world;
