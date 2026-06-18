//! HePrint HTTP/WS 服务器

pub mod server;
pub mod ws;
pub mod router;
pub mod session;
pub mod print_manager;

pub use server::run;
pub use server::ServerConfig;
pub use print_manager::{PrintManager, PrintJob, spawn_workers};

pub const HE_VERSION: &str = "1.0.0";
