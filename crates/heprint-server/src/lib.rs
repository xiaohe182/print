//! HePrint HTTP/WS 服务器

pub mod print_manager;
pub mod router;
pub mod server;
pub mod session;
pub mod ws;

pub use print_manager::{spawn_workers, PrintJob, PrintManager};
pub use server::run;
pub use server::ServerConfig;

pub const HE_VERSION: &str = "1.1.1";
