//! 全局打印管理器：跨 WS 会话的任务调度，并发 worker pool

use heprint_core::{ErrorCode, HeError, PrintTask};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Notify;

/// 打印作业（带元数据）
#[derive(Debug, Clone)]
pub struct PrintJob {
    pub job_id: String,
    pub task: PrintTask,
    pub silent: bool,
}

/// 全局打印管理器（单例）
pub struct PrintManager {
    queue: Mutex<VecDeque<PrintJob>>,
    notify: Notify,
    running: Mutex<Vec<PrintJob>>,
    #[allow(dead_code)]
    max_concurrent: usize,
}

impl PrintManager {
    pub fn new(max_concurrent: usize) -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(VecDeque::new()),
            notify: Notify::new(),
            running: Mutex::new(Vec::new()),
            max_concurrent,
        })
    }

    /// 提交一个打印任务到全局队列
    pub fn submit(&self, task: PrintTask, silent: bool) -> String {
        let job_id = task.task_id.clone();
        let job = PrintJob {
            job_id: job_id.clone(),
            task,
            silent,
        };
        self.queue.lock().push_back(job);
        // 唤醒一个 worker
        self.notify.notify_one();
        job_id
    }

    /// 取出下一个待打印任务（阻塞）
    pub async fn next_job(&self) -> PrintJob {
        loop {
            // 先尝试拿
            {
                let mut q = self.queue.lock();
                if let Some(job) = q.pop_front() {
                    self.running.lock().push(job.clone());
                    return job;
                }
            }
            // 等通知
            self.notify.notified().await;
        }
    }

    /// 标记任务完成
    pub fn finish(&self, job: PrintJob) {
        self.running.lock().retain(|j| j.job_id != job.job_id);
        // 通知下一个
        self.notify.notify_one();
    }

    /// 当前运行中数量
    pub fn running_count(&self) -> usize {
        self.running.lock().len()
    }

    /// 队列长度
    pub fn queue_len(&self) -> usize {
        self.queue.lock().len()
    }
}

/// 启动 worker pool（应在 main 中调用一次）
pub fn spawn_workers(manager: Arc<PrintManager>, n: usize) {
    for worker_id in 0..n {
        let mgr = manager.clone();
        tokio::spawn(async move {
            loop {
                let job = mgr.next_job().await;
                tracing::info!(
                    "[Worker #{}] 取出任务 taskId={}, name={}, printer={:?}",
                    worker_id,
                    job.task.task_id,
                    job.task.name,
                    job.task.printer_name
                );

                let silent = job.silent;
                let task = job.task.clone();
                let task_id = task.task_id.clone();

                // 在阻塞线程中调用 GDI（防止阻塞 tokio）
                let result =
                    tokio::task::spawn_blocking(move || heprint_print::print_task(task, silent))
                        .await
                        .unwrap_or_else(|e| {
                            Err(HeError::coded(ErrorCode::Unknown, format!("线程错误: {e}")))
                        });

                match result {
                    Ok(r) => tracing::info!(
                        "[Worker #{}] ✅ 任务完成: taskId={}, pages={:?}",
                        worker_id,
                        r.task_id,
                        r.pages
                    ),
                    Err(e) => tracing::error!(
                        "[Worker #{}] ❌ 任务失败: taskId={}, error={}",
                        worker_id,
                        task_id,
                        e
                    ),
                }

                mgr.finish(job);
            }
        });
    }
}
