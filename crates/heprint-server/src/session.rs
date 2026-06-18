//! WebSocket 会话状态

use heprint_core::TaskManager;
use parking_lot::Mutex;
use std::sync::Arc;

/// 一个 WS 连接的会话状态
pub struct Session {
    pub task_manager: Arc<Mutex<TaskManager>>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            task_manager: Arc::new(Mutex::new(TaskManager::new())),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
