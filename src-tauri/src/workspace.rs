use std::fs;
use std::io::{Read as _, Write as _};
use std::path::Path;

/// Copy default project files into workspace if project.json doesn't exist
pub fn ensure_default_project(workspace: &Path, resource_dir: &Path) {
    let project_json = workspace.join("project.json");
    if project_json.exists() {
        eprintln!("[live-scratch] project.json already exists, skipping default project copy");
        return;
    }

    if let Err(err) = fs::create_dir_all(workspace) {
        eprintln!("[live-scratch] Failed to create workspace {:?}: {}", workspace, err);
        return;
    }

    let default_project = resource_dir.join("default-project");
    eprintln!("[live-scratch] Looking for default-project at: {:?} (exists: {})", default_project, default_project.exists());
    if !default_project.exists() {
        eprintln!("[live-scratch] ERROR: default-project not found at {:?}", default_project);
        return;
    }

    let entries = match fs::read_dir(&default_project) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("[live-scratch] Failed to read default-project dir: {}", err);
            return;
        }
    };

    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let dest = workspace.join(entry.file_name());
            match fs::copy(&path, &dest) {
                Ok(_) => {
                    count += 1;
                }
                Err(err) => {
                    eprintln!("[live-scratch] Failed to copy {:?} -> {:?}: {}", path, dest, err);
                }
            }
        }
    }
    eprintln!("[live-scratch] Copied {} files from default-project to {:?}", count, workspace);
}

/// Write CLAUDE.md to workspace from resource dir (always overwritten to stay current)
pub fn ensure_claude_md(workspace: &Path, resource_dir: &Path) {
    let src = resource_dir.join("default-project").join("CLAUDE.md");
    if !src.exists() {
        eprintln!("[live-scratch] CLAUDE.md not found at {:?}", src);
        return;
    }
    let dest = workspace.join("CLAUDE.md");
    match fs::copy(&src, &dest) {
        Ok(_) => eprintln!("[live-scratch] CLAUDE.md written to {:?}", dest),
        Err(err) => eprintln!("[live-scratch] Failed to copy CLAUDE.md: {}", err),
    }
}

/// Build an SB3 (ZIP with STORE compression) from workspace files.
/// Returns None if project.json has invalid JSON.
pub fn build_sb3(workspace: &Path) -> Option<Vec<u8>> {
    let entries = match fs::read_dir(workspace) {
        Ok(e) => e,
        Err(err) => {
            log::error!("Failed to read workspace: {}", err);
            return None;
        }
    };

    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip non-Scratch files (e.g. CLAUDE.md)
            if file_name_str.ends_with(".md") {
                continue;
            }

            let content = match fs::read(&path) {
                Ok(c) => c,
                Err(err) => {
                    log::error!("Failed to read {:?}: {}", path, err);
                    continue;
                }
            };

            // Validate project.json
            if file_name_str == "project.json" {
                let json_str = String::from_utf8_lossy(&content);
                if let Err(err) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    log::error!("JSON syntax error in project.json: {}", err);
                    return None;
                }
            }

            if let Err(err) = zip.start_file(file_name_str.as_ref(), options) {
                log::error!("Failed to start zip entry {:?}: {}", file_name_str, err);
                continue;
            }
            if let Err(err) = zip.write_all(&content) {
                log::error!("Failed to write zip entry {:?}: {}", file_name_str, err);
                continue;
            }
        }

        if let Err(err) = zip.finish() {
            log::error!("Failed to finish zip: {}", err);
            return None;
        }
    }

    Some(buf)
}

/// Extract an SB3 (ZIP) into the workspace directory.
/// project.json is pretty-printed.
pub fn extract_sb3(workspace: &Path, data: &[u8]) {
    fs::create_dir_all(workspace).ok();

    let cursor = std::io::Cursor::new(data);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(err) => {
            log::error!("Failed to open SB3 zip: {}", err);
            return;
        }
    };

    let mut count = 0;
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(err) => {
                log::error!("Failed to read zip entry {}: {}", i, err);
                continue;
            }
        };

        if file.is_dir() {
            continue;
        }

        let name = file.name().to_string();
        let out_path = workspace.join(&name);

        let mut content = Vec::new();
        if let Err(err) = file.read_to_end(&mut content) {
            log::error!("Failed to read zip entry {:?}: {}", name, err);
            continue;
        }

        if name == "project.json" {
            // Pretty-print project.json
            match serde_json::from_slice::<serde_json::Value>(&content) {
                Ok(json) => {
                    let pretty = serde_json::to_string_pretty(&json).unwrap_or_default();
                    if let Err(err) = fs::write(&out_path, pretty) {
                        log::error!("Failed to write {:?}: {}", out_path, err);
                    }
                }
                Err(_) => {
                    if let Err(err) = fs::write(&out_path, &content) {
                        log::error!("Failed to write {:?}: {}", out_path, err);
                    }
                }
            }
        } else if let Err(err) = fs::write(&out_path, &content) {
            log::error!("Failed to write {:?}: {}", out_path, err);
        }

        count += 1;
    }
    log::info!("Extracted {} files to workspace", count);
}
