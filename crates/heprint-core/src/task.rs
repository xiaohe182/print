//! 打印任务（PrintTask）与任务管理器

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::item::PrintItem;
use crate::types::Orient;

/// 纸张配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageConfig {
    pub orient: Orient,
    /// 单位：mm
    pub width_mm: f64,
    /// 单位：mm（卷筒模式填 0）
    pub height_mm: f64,
    /// 系统纸张名（如 "A4"），优先级高于 width/height
    pub name: Option<String>,
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            orient: Orient::Portrait,
            width_mm: 210.0,
            height_mm: 297.0,
            name: Some("A4".to_string()),
        }
    }
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Building,
    Ready,
    Printing,
    Done,
    Error,
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub pages: Option<u32>,
}

impl TaskResult {
    pub fn success(task_id: String, pages: u32) -> Self {
        Self {
            task_id,
            success: true,
            error: None,
            pages: Some(pages),
        }
    }

    pub fn failure<S: Into<String>>(task_id: String, err: S) -> Self {
        Self {
            task_id,
            success: false,
            error: Some(err.into()),
            pages: None,
        }
    }
}

/// 单个打印任务
#[derive(Debug, Clone)]
pub struct PrintTask {
    pub task_id: String,
    /// 短 ID（T_001, T_002 ...），便于前端使用
    pub short_id: Option<String>,
    pub name: String,
    pub items: Vec<PrintItem>,
    pub page: PageConfig,
    pub printer_name: Option<String>,
    pub copies: u32,
    pub options: HashMap<String, serde_json::Value>,
    pub status: TaskStatus,
}

impl PrintTask {
    pub fn new(name: String) -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
            short_id: None,
            name,
            items: Vec::new(),
            page: PageConfig::default(),
            printer_name: None,
            copies: 1,
            options: HashMap::new(),
            status: TaskStatus::Building,
        }
    }

    pub fn push_item(&mut self, item: PrintItem) {
        self.items.push(item);
    }

    pub fn last_item_mut(&mut self) -> Option<&mut PrintItem> {
        self.items.last_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|i| matches!(i, PrintItem::PageBreak))
    }
}

/// 任务管理器（每个 WS 连接持有一个）
///
/// v1.1 改进：支持**多任务并行**（不再共享单一 current），
/// 任务通过 task_id 区分。
pub struct TaskManager {
    pub current: Option<PrintTask>,
    pub tasks: HashMap<String, PrintTask>,
    pub next_short_id: u32,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            current: None,
            tasks: HashMap::new(),
            next_short_id: 1,
        }
    }

    /// 旧 API 兼容：初始化新任务作为 current
    pub fn init(&mut self, name: String) -> &mut PrintTask {
        let task = PrintTask::new(name);
        self.current = Some(task);
        self.current.as_mut().unwrap()
    }

    /// v1.1 新 API：打开独立任务，返回 task_id（短 ID）
    pub fn open_task(&mut self, name: String) -> String {
        let mut task = PrintTask::new(name);
        let short_id = format!("T_{:03}", self.next_short_id);
        self.next_short_id += 1;
        // 同时设置 task_id 的"短别名"（用 task.short_id 字段）
        task.short_id = Some(short_id.clone());
        let id = task.task_id.clone();
        self.tasks.insert(id.clone(), task);
        short_id
    }

    /// v1.1 新 API：通过 task_id（短或长）获取任务
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut PrintTask> {
        // 优先按短 ID 找
        let found_id = self.tasks.iter()
            .find(|(_, t)| t.short_id.as_deref() == Some(task_id))
            .map(|(id, _)| id.clone());
        if let Some(id) = found_id {
            return self.tasks.get_mut(&id);
        }
        // 按完整 UUID 找
        self.tasks.get_mut(task_id)
    }

    pub fn current_mut(&mut self) -> Option<&mut PrintTask> {
        self.current.as_mut()
    }

    /// v1.1 新 API：取出指定任务（消费）
    pub fn take_task(&mut self, task_id: &str) -> Option<PrintTask> {
        // 按短 ID
        let found_id = self.tasks.iter()
            .find(|(_, t)| t.short_id.as_deref() == Some(task_id))
            .map(|(id, _)| id.clone());
        if let Some(id) = found_id {
            return self.tasks.remove(&id);
        }
        // 按完整 ID
        self.tasks.remove(task_id)
    }

    /// 旧 API 兼容：取出 current
    pub fn take(&mut self) -> Option<PrintTask> {
        self.current.take()
    }

    /// 关闭并移除任务
    pub fn close_task(&mut self, task_id: &str) -> bool {
        self.take_task(task_id).is_some()
    }

    /// 列出所有活跃任务
    pub fn list_tasks(&self) -> Vec<(String, String)> {
        self.tasks.iter()
            .map(|(id, t)| (id.clone(), t.short_id.clone().unwrap_or_default()))
            .collect()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PrintTask {
    fn default() -> Self {
        Self::new("untitled".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rect;
    use crate::style::PrintStyle;

    #[test]
    fn test_task_init_and_push() {
        let mut mgr = TaskManager::new();
        let task = mgr.init("test".to_string());
        assert_eq!(task.name, "test");
        assert!(task.items.is_empty());

        task.push_item(PrintItem::Text {
            bounds: Rect::new(0, 0, 100, 100),
            style: PrintStyle::default(),
            text: "Hello".to_string(),
        });
        assert_eq!(task.items.len(), 1);
    }
}
