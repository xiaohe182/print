//! JSON-RPC 方法分发
//!
//! v1.1：支持多任务并行（HE_OPEN_TASK / HE_ADD_TO_TASK / HE_PRINT_TASK）

use heprint_core::{
    BarcodeType, ErrorCode, HeError, ItemType, LineStyle, Orient, PageConfig,
    PrintItem, PrintStyle, Rect, Result,
};
use serde_json::{json, Value};

use crate::print_manager::PrintManager;
use crate::session::Session;
use crate::HE_VERSION;
use std::sync::Arc;

/// 主分发函数
pub async fn dispatch(
    method: &str,
    params: Value,
    session: &Session,
    mgr: &Arc<PrintManager>,
) -> Result<Value> {
    match method {
        // ===== 初始化 =====
        "HE_INIT" => he_init(params, session),
        "HE_OPEN_TASK" => he_open_task(params, session),
        "HE_CLOSE_TASK" => he_close_task(params, session),
        "HE_LIST_TASKS" => he_list_tasks(session),
        "HE_VERSION" => Ok(json!({ "version": HE_VERSION })),

        // ===== 添加内容（支持 task_id 参数）=====
        "HE_ADD_TEXT" => he_add_text(params, session, None),
        "HE_ADD_HTML" => he_add_html(params, session, None),
        "HE_ADD_TABLE" => he_add_table(params, session, None),
        "HE_ADD_IMAGE" => he_add_image(params, session, None),
        "HE_ADD_BARCODE" => he_add_barcode(params, session, None),
        "HE_ADD_PDF" => he_add_pdf(params, session, None),
        "HE_ADD_LINE" => he_add_line(params, session, None),
        "HE_ADD_RECT" => he_add_rect(params, session, None),

        // ===== 样式 =====
        "HE_SET_STYLE" => he_set_style(params, session),

        // ===== 全局参数 =====
        "HE_SET_PAGE" => he_set_page(params, session, None),
        "HE_SET_PRINTER" => he_set_printer(params, session, None),
        "HE_SET_COPIES" => he_set_copies(params, session, None),
        "HE_SET_OPTION" => he_set_option(params, session, None),

        // ===== 执行 =====
        "HE_PRINT" => he_print(session, false, mgr, None).await,
        "HE_PRINT_SILENT" => he_print(session, true, mgr, None).await,
        "HE_PRINT_TASK" => he_print_task(params, session, mgr).await,
        "HE_PREVIEW" => Ok(json!({ "ok": true, "message": "请用 HE_PRINT 后在系统对话框预览" })),
        "HE_NEW_PAGE" => he_new_page(session, None),

        // ===== 打印机查询 =====
        "HE_GET_PRINTERS" => he_get_printers().await,
        "HE_GET_DEFAULT_PRINTER" => he_get_default_printer().await,
        "HE_HAS_PRINTER" => he_has_printer(params).await,
        "HE_GET_INFO" => he_get_info(params, mgr),

        // ===== 回调 =====
        "HE_ON_RESULT" => Ok(json!({ "registered": true })),

        // ===== 原生 ESC/POS =====
        "HE_SEND_RAW" => he_send_raw(params).await,

        _ => Err(HeError::coded(
            ErrorCode::MethodNotFound,
            format!("未知方法: {method}"),
        )),
    }
}

// ============ 工具：根据 task_id 解析任务引用 ============

/// 从参数中提取 task_id（兼容空值 = 旧 API = current task）
fn extract_task_id(params: &Value) -> Option<String> {
    params.get("taskId").and_then(|v| v.as_str()).map(String::from)
}

// ============ 任务管理 ============

fn he_init(params: Value, session: &Session) -> Result<Value> {
    let task_name = params
        .get("taskName")
        .and_then(|v| v.as_str())
        .unwrap_or("untitled")
        .to_string();

    let mut mgr = session.task_manager.lock();
    let task = mgr.init(task_name.clone());
    Ok(json!({
        "ok": true,
        "taskId": task.short_id.clone().unwrap_or(task.task_id.clone()),
        "fullId": task.task_id,
        "taskName": task_name
    }))
}

fn he_open_task(params: Value, session: &Session) -> Result<Value> {
    let task_name = params
        .get("taskName")
        .and_then(|v| v.as_str())
        .unwrap_or("untitled")
        .to_string();

    let mut mgr = session.task_manager.lock();
    let short_id = mgr.open_task(task_name.clone());
    // 找到 full id
    let full_id = mgr
        .tasks
        .values()
        .find(|t| t.short_id.as_deref() == Some(&short_id))
        .map(|t| t.task_id.clone())
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "taskId": short_id,
        "fullId": full_id,
        "taskName": task_name
    }))
}

fn he_close_task(params: Value, session: &Session) -> Result<Value> {
    let task_id = extract_task_id(&params).ok_or_else(|| {
        HeError::coded(ErrorCode::InvalidParam, "缺少 taskId")
    })?;
    let mut mgr = session.task_manager.lock();
    let closed = mgr.close_task(&task_id);
    Ok(json!({ "ok": closed }))
}

fn he_list_tasks(session: &Session) -> Result<Value> {
    let mgr = session.task_manager.lock();
    let list: Vec<_> = mgr.tasks.values().map(|t| json!({
        "taskId": t.short_id.clone().unwrap_or_default(),
        "fullId": t.task_id,
        "name": t.name,
        "items": t.items.len(),
        "status": t.status,
        "printer": t.printer_name,
    })).collect();
    Ok(json!({ "ok": true, "tasks": list }))
}

// ============ 添加内容（统一支持 task_id） ============

fn parse_rect(params: &Value) -> Result<Rect> {
    let top = params.get("top").and_then(|v| v.as_i64())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 top"))? as i32;
    let left = params.get("left").and_then(|v| v.as_i64())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 left"))? as i32;
    let width = params.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let height = params.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    Ok(Rect::new(top, left, width, height))
}

fn he_add_text(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Text {
        bounds,
        style: PrintStyle::default(),
        text,
    });
    Ok(json!({ "ok": true, "index": task.items.len() - 1 }))
}

fn he_add_html(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let html = params.get("html").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Html { bounds, style: PrintStyle::default(), html });
    Ok(json!({ "ok": true }))
}

fn he_add_table(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let html = params.get("tableHtml").or_else(|| params.get("html"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Table { bounds, style: PrintStyle::default(), html });
    Ok(json!({ "ok": true }))
}

fn he_add_image(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let src = params.get("src").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Image { bounds, style: PrintStyle::default(), src });
    Ok(json!({ "ok": true }))
}

fn he_add_barcode(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let btype_str = params.get("btype").or_else(|| params.get("type"))
        .and_then(|v| v.as_str()).unwrap_or("QRCode");
    let btype = parse_barcode_type(btype_str)?;
    let value = params.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Barcode { bounds, style: PrintStyle::default(), btype, value });
    Ok(json!({ "ok": true }))
}

fn he_add_pdf(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Pdf { bounds, style: PrintStyle::default(), content });
    Ok(json!({ "ok": true }))
}

fn he_add_line(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let top1 = params.get("top1").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let left1 = params.get("left1").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let top2 = params.get("top2").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let left2 = params.get("left2").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let style_str = params.get("lineStyle").and_then(|v| v.as_str()).unwrap_or("solid");
    let line_width = params.get("lineWidth").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Line {
        x1: left1, y1: top1, x2: left2, y2: top2,
        style: PrintStyle::default(),
        line_style: LineStyle::parse(style_str),
        line_width,
    });
    Ok(json!({ "ok": true }))
}

fn he_add_rect(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let bounds = parse_rect(&params)?;
    let style_str = params.get("lineStyle").and_then(|v| v.as_str()).unwrap_or("solid");
    let line_width = params.get("lineWidth").and_then(|v| v.as_f64()).unwrap_or(1.0);
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::Rect {
        bounds,
        style: PrintStyle::default(),
        line_style: LineStyle::parse(style_str),
        line_width,
    });
    Ok(json!({ "ok": true }))
}

fn he_set_style(params: Value, session: &Session) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let name = params.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 name"))?.to_string();
    let value = params.get("value").cloned().unwrap_or(Value::Null);
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    let last = task.last_item_mut()
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "无可设置样式的项"))?;
    let style = last.style_mut()
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "该项不支持设置样式"))?;
    style.set(&name, &value).map_err(|e| HeError::coded(ErrorCode::InvalidParam, e))?;
    Ok(json!({ "ok": true }))
}

fn he_set_page(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let orient = params.get("orient").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let width = params.get("width").and_then(|v| v.as_f64()).unwrap_or(210.0);
    let height = params.get("height").and_then(|v| v.as_f64()).unwrap_or(297.0);
    let name = params.get("name").and_then(|v| v.as_str()).map(String::from);
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.page = PageConfig {
        orient: Orient::from(orient),
        width_mm: width,
        height_mm: height,
        name,
    };
    Ok(json!({ "ok": true }))
}

fn he_set_printer(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let printer_val = params.get("printer").unwrap_or(&Value::Null);
    let printer_name = match printer_val {
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            let idx = n.as_i64().unwrap_or(-1);
            if idx < 0 {
                heprint_print::get_default_printer().unwrap_or_default()
            } else {
                let list = heprint_print::enum_printers().unwrap_or_default();
                list.get(idx as usize).map(|p| p.name.clone()).unwrap_or_default()
            }
        }
        _ => return Err(HeError::coded(ErrorCode::InvalidParam, "printer 参数必须是 string 或 number")),
    };
    if printer_name.is_empty() {
        return Err(HeError::code(ErrorCode::PrinterNotFound));
    }
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.printer_name = Some(printer_name.clone());
    Ok(json!({ "ok": true, "printerName": printer_name }))
}

fn he_set_copies(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(1).max(1) as u32;
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.copies = count;
    Ok(json!({ "ok": true, "copies": count }))
}

fn he_set_option(params: Value, session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id = extract_task_id(&params);
    let key = params.get("key").and_then(|v| v.as_str())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 key"))?.to_string();
    let value = params.get("value").cloned().unwrap_or(Value::Null);
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.options.insert(key, value);
    Ok(json!({ "ok": true }))
}

fn he_new_page(session: &Session, _force_task: Option<String>) -> Result<Value> {
    let task_id: Option<String> = None; // 旧 API
    let mut mgr = session.task_manager.lock();
    let task = if let Some(tid) = task_id {
        mgr.get_task_mut(&tid)
    } else {
        mgr.current_mut()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, "请先调用 HE_INIT 或 HE_OPEN_TASK"))?;
    task.push_item(PrintItem::PageBreak);
    Ok(json!({ "ok": true }))
}

// ============ 打印（v1.1：支持 task_id 指定） ============

async fn he_print(
    session: &Session,
    silent: bool,
    mgr: &Arc<PrintManager>,
    _force_task: Option<String>,
) -> Result<Value> {
    // 取出 current 任务
    let task = {
        let mut smgr = session.task_manager.lock();
        smgr.take()
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskEmpty, "无 current 任务"))?;

    let task_id = task.task_id.clone();
    let job_id = mgr.submit(task, silent);
    Ok(json!({
        "ok": true,
        "taskId": task_id,
        "jobId": job_id,
        "queued": true,
        "runningJobs": mgr.running_count(),
        "queueLength": mgr.queue_len(),
    }))
}

async fn he_print_task(
    params: Value,
    session: &Session,
    mgr: &Arc<PrintManager>,
) -> Result<Value> {
    let task_id = extract_task_id(&params).ok_or_else(|| {
        HeError::coded(ErrorCode::InvalidParam, "缺少 taskId")
    })?;
    let silent = params.get("silent").and_then(|v| v.as_bool()).unwrap_or(true);

    let task = {
        let mut smgr = session.task_manager.lock();
        smgr.take_task(&task_id)
    }.ok_or_else(|| HeError::coded(ErrorCode::TaskNotFound, format!("任务不存在: {task_id}")))?;

    let job_id = mgr.submit(task, silent);
    Ok(json!({
        "ok": true,
        "taskId": task_id,
        "jobId": job_id,
        "queued": true,
        "runningJobs": mgr.running_count(),
        "queueLength": mgr.queue_len(),
    }))
}

// ============ 打印机查询 ============

async fn he_get_printers() -> Result<Value> {
    let printers = tokio::task::spawn_blocking(|| heprint_print::enum_printers())
        .await
        .map_err(|e| HeError::coded(ErrorCode::Unknown, e.to_string()))??;
    let names: Vec<String> = printers.iter().map(|p| p.name.clone()).collect();
    Ok(json!({
        "printers": names,
        "details": printers,
    }))
}

async fn he_get_default_printer() -> Result<Value> {
    let name = tokio::task::spawn_blocking(|| heprint_print::get_default_printer())
        .await
        .map_err(|e| HeError::coded(ErrorCode::Unknown, e.to_string()))??;
    Ok(json!({ "name": name }))
}

async fn he_has_printer(params: Value) -> Result<Value> {
    let name = params.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 name"))?
        .to_string();
    let exists = tokio::task::spawn_blocking(move || heprint_print::has_printer(&name))
        .await
        .map_err(|e| HeError::coded(ErrorCode::Unknown, e.to_string()))??;
    Ok(json!({ "exists": exists }))
}

fn he_get_info(params: Value, mgr: &Arc<PrintManager>) -> Result<Value> {
    let key = params.get("key").and_then(|v| v.as_str())
        .unwrap_or("version").to_string();
    let value = match key.as_str() {
        "version" => json!(HE_VERSION),
        "serverIp" => json!("127.0.0.1"),
        "clientIp" => json!("127.0.0.1"),
        "status" => json!("running"),
        "printerCount" => {
            let n = heprint_print::enum_printers().map(|p| p.len()).unwrap_or(0);
            json!(n)
        }
        "runningJobs" => json!(mgr.running_count()),
        "queueLength" => json!(mgr.queue_len()),
        _ => json!(null),
    };
    Ok(json!({ "key": key, "value": value }))
}

fn parse_barcode_type(s: &str) -> Result<BarcodeType> {
    Ok(match s {
        "QRCode" | "qrcode" => BarcodeType::QRCode,
        "Code128" => BarcodeType::Code128,
        "Code39" => BarcodeType::Code39,
        "EAN13" => BarcodeType::EAN13,
        "EAN8" => BarcodeType::EAN8,
        "UPC-A" => BarcodeType::UpcA,
        "UPC-E" => BarcodeType::UpcE,
        "PDF417" => BarcodeType::PDF417,
        "DataMatrix" => BarcodeType::DataMatrix,
        _ => return Err(HeError::coded(ErrorCode::InvalidBarcodeType, format!("不支持的条码类型: {s}"))),
    })
}

#[allow(dead_code)]
fn _unused() {
    let _ = ItemType::Normal;
}

// ============ 原生指令（ESC/POS） ============

/// HE_SEND_RAW：直接发送字节流到打印机（小票机/标签机/钱箱/切纸等）
/// params: { printerName, data, encoding: "base64" | "hex" }
async fn he_send_raw(params: Value) -> Result<Value> {
    let printer_name = params.get("printerName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 printerName"))?
        .to_string();
    let data = params.get("data")
        .and_then(|v| v.as_str())
        .ok_or_else(|| HeError::coded(ErrorCode::InvalidParam, "缺少 data"))?
        .to_string();
    let encoding = params.get("encoding").and_then(|v| v.as_str()).unwrap_or("base64");

    // 解码
    let bytes: Vec<u8> = match encoding {
        "hex" => {
            let s: String = data.chars().filter(|c| !c.is_whitespace()).collect();
            let mut out = Vec::with_capacity(s.len() / 2);
            let mut chars = s.chars();
            while let (Some(a), Some(b)) = (chars.next(), chars.next()) {
                let byte = u8::from_str_radix(&format!("{a}{b}"), 16)
                    .map_err(|e| HeError::coded(ErrorCode::InvalidParam, format!("hex 解析失败: {e}")))?;
                out.push(byte);
            }
            out
        }
        "base64" | _ => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(data.as_bytes())
                .map_err(|e| HeError::coded(ErrorCode::InvalidParam, format!("base64 解析失败: {e}")))?
        }
    };

    // 阻塞线程中调用 winspool
    let bytes_len = bytes.len();
    let pn = printer_name.clone();
    let result = tokio::task::spawn_blocking(move || {
        heprint_print::send_raw_to_printer(&pn, &bytes)
    })
    .await
    .map_err(|e| HeError::coded(ErrorCode::Unknown, format!("线程错误: {e}")))?;

    match result {
        Ok(_) => Ok(json!({
            "ok": true,
            "printer": printer_name,
            "bytes": bytes_len,
        })),
        Err(e) => Err(e),
    }
}
