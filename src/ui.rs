pub mod config;
pub mod connection;
pub mod dns;
mod error_dialog;
pub mod groups;
mod latency;
pub mod login_dialog;
pub mod nodes;
pub mod overview;
pub mod proxy;
mod qr_code;
pub mod routing;
pub mod server;
mod traffic;
pub mod workspace;

#[derive(Debug)]
pub enum ServerEvent {
    Changed,
    Unauthorized,
}
