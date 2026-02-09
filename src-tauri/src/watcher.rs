use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use tauri::{AppHandle, Emitter};

use crate::workspace;

/// Flag to prevent file-change → rebuild → emit loop when we ourselves write to workspace
static IGNORE_FILE_CHANGE: AtomicBool = AtomicBool::new(false);

pub fn set_ignore(val: bool) {
    IGNORE_FILE_CHANGE.store(val, Ordering::SeqCst);
}

pub fn is_ignoring() -> bool {
    IGNORE_FILE_CHANGE.load(Ordering::SeqCst)
}

/// Start watching the workspace directory. On change, build SB3 and emit to frontend.
pub fn start_watcher(app: AppHandle, workspace_path: PathBuf) {
    std::thread::spawn(move || {
        let app_handle = app.clone();
        let ws_path = workspace_path.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(300),
            None,
            move |result: DebounceEventResult| {
                if is_ignoring() {
                    return;
                }

                match result {
                    Ok(events) => {
                        // Ignore changes to non-Scratch files (e.g. CLAUDE.md)
                        let has_scratch_change = events.iter().any(|e| {
                            e.paths.iter().any(|p| {
                                p.extension().map_or(true, |ext| ext != "md")
                            })
                        });
                        if events.is_empty() || !has_scratch_change {
                            return;
                        }
                        log::info!("[live-scratch] file change detected");

                        match workspace::build_sb3(&ws_path) {
                            Some(sb3) => {
                                let encoded = base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &sb3,
                                );
                                if let Err(err) = app_handle.emit("sb3-updated", &encoded) {
                                    log::error!("Failed to emit sb3-updated: {}", err);
                                }
                                log::info!("[live-scratch] emitted sb3-updated ({} bytes)", sb3.len());
                            }
                            None => {
                                log::warn!("[live-scratch] skipping emit (build error)");
                            }
                        }
                    }
                    Err(errors) => {
                        for err in errors {
                            log::error!("Watch error: {}", err);
                        }
                    }
                }
            },
        )
        .expect("Failed to create file watcher");

        debouncer
            .watch(&workspace_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch workspace directory");

        log::info!("[live-scratch] watching {:?}", workspace_path);

        // Keep the watcher alive
        loop {
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}
