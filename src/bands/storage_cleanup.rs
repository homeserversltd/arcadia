fn now_rfc3339_like() -> String {
    command_stdout("date", &["-Iseconds"]).unwrap_or_else(|| "Unknown".to_string())
}
fn system_time_string(t: SystemTime) -> Option<String> {
    let secs = t.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{}", secs))
}

fn storage_health(percent_used: u8) -> (&'static str, String, &'static str) {
    match percent_used {
        0..=74 => ("OK", "OK".to_string(), "good"),
        75..=89 => ("Getting Full", format!("{}%", percent_used), "warn"),
        90..=97 => ("Low Space", "Low".to_string(), "warn"),
        _ => ("Full", "Full".to_string(), "bad"),
    }
}

#[derive(Default)]
struct Usage {
    bytes: u64,
    files: u64,
}

fn path_usage(path: &Path) -> Usage {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Usage::default();
    };
    if metadata.file_type().is_symlink() {
        return Usage::default();
    }
    if metadata.is_file() {
        return Usage {
            bytes: metadata.len(),
            files: 1,
        };
    }
    if !metadata.is_dir() {
        return Usage::default();
    }
    let mut usage = Usage::default();
    let Ok(entries) = fs::read_dir(path) else {
        return usage;
    };
    for entry in entries.flatten() {
        let child = path_usage(&entry.path());
        usage.bytes = usage.bytes.saturating_add(child.bytes);
        usage.files = usage.files.saturating_add(child.files);
    }
    usage
}

fn remove_children(path: &Path) -> std::io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut removed = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let metadata = fs::symlink_metadata(&child)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&child)?;
        } else {
            fs::remove_file(&child)?;
        }
        removed += 1;
    }
    Ok(removed)
}

fn storage_remove_children(
    action: &'static str,
    path: &str,
    success: &str,
    failure: &str,
) -> (StatusCode, Json<ConsoleActionResponse>) {
    match remove_children(Path::new(path)) {
        Ok(count) => (
            StatusCode::OK,
            Json(ConsoleActionResponse {
                ok: true,
                action,
                command: "arcadia-storage",
                exit_code: Some(0),
                message: format!("{} Removed {} entries.", success, count),
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConsoleActionResponse {
                ok: false,
                action,
                command: "arcadia-storage",
                exit_code: Some(1),
                message: failure.to_string(),
                stdout: String::new(),
                stderr: err.to_string(),
            }),
        ),
    }
}

fn collect_ai_models(path: &Path, out: &mut Vec<(u64, String, PathBuf)>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            collect_ai_models(&path, out, depth + 1);
        } else if metadata.is_file() && is_ai_model_file(&path) {
            let filename = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("model")
                .to_string();
            out.push((metadata.len(), filename, path));
        }
    }
}

fn is_ai_model_file(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .map(|ext| {
            MODEL_EXTENSIONS
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

fn find_model_path_by_filename(filename: &str) -> Option<PathBuf> {
    let mut models = Vec::new();
    for root in model_roots() {
        collect_ai_models(Path::new(&root.path), &mut models, 0);
    }
    models
        .into_iter()
        .find(|(_, name, _)| name == filename)
        .map(|(_, _, path)| path)
}

fn friendly_model_name(filename: &str) -> String {
    let mut name = filename.to_string();
    for suffix in [".gguf", ".safetensors", ".onnx"] {
        if name.to_lowercase().ends_with(suffix) {
            let new_len = name.len().saturating_sub(suffix.len());
            name.truncate(new_len);
            break;
        }
    }
    name.replace(['_', '-'], " ")
}

fn percent(part: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((part.saturating_mul(100) / total).min(100)) as u8
}

pub(crate) fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else if value >= 100.0 {
        format!("{:.0} {}", value, UNITS[unit])
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}
