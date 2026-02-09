mod commands;
mod watcher;
mod workspace;

use std::fs;

use commands::WorkspacePath;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Determine workspace path
            let app_data = dirs::document_dir()
                .expect("Failed to get Documents directory")
                .join("Live Scratch");
            fs::create_dir_all(&app_data).expect("Failed to create workspace directory");

            log::info!("[live-scratch] workspace: {:?}", app_data);

            // Find default-project directory, trying multiple locations:
            // 1. Tauri resource dir (production bundle)
            // 2. Project root via CARGO_MANIFEST_DIR (dev mode)
            // 3. Relative to the executable (fallback)
            let default_project_dir = {
                let mut candidates: Vec<std::path::PathBuf> = Vec::new();

                // Production: resource_dir/default-project/
                if let Ok(res_dir) = app.path().resource_dir() {
                    eprintln!("[live-scratch] resource_dir: {:?}", res_dir);
                    candidates.push(res_dir);
                }

                // Dev mode: CARGO_MANIFEST_DIR (src-tauri) -> parent (project root)
                let manifest_parent = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .map(|p| p.to_path_buf());
                if let Some(ref project_root) = manifest_parent {
                    eprintln!("[live-scratch] project root (CARGO_MANIFEST_DIR parent): {:?}", project_root);
                    candidates.push(project_root.clone());
                }

                // Find the first candidate that contains default-project/
                let found = candidates.iter().find(|p| p.join("default-project").exists());
                match found {
                    Some(dir) => {
                        eprintln!("[live-scratch] default-project found at: {:?}", dir.join("default-project"));
                        dir.clone()
                    }
                    None => {
                        eprintln!("[live-scratch] WARNING: default-project not found in any candidate: {:?}", candidates);
                        // Last resort: use project root even if default-project doesn't exist there
                        manifest_parent.unwrap_or_default()
                    }
                }
            };

            // Ensure default project exists
            workspace::ensure_default_project(&app_data, &default_project_dir);

            // Always write CLAUDE.md (overwrite to stay current)
            workspace::ensure_claude_md(&app_data, &default_project_dir);

            // Store workspace path in state
            app.manage(WorkspacePath(app_data.clone()));

            // Build initial SB3
            match workspace::build_sb3(&app_data) {
                Some(_) => log::info!("[live-scratch] initial SB3 built successfully"),
                None => log::warn!("[live-scratch] initial SB3 build failed"),
            }

            // Start file watcher
            watcher::start_watcher(app.handle().clone(), app_data);

            // Build macOS menu
            let app_handle = app.handle();

            let open_sb3 = MenuItemBuilder::with_id("open_sb3", "Open SB3...")
                .accelerator("CmdOrCtrl+O")
                .build(app_handle)?;
            let export_sb3 = MenuItemBuilder::with_id("export_sb3", "Export SB3...")
                .accelerator("CmdOrCtrl+S")
                .build(app_handle)?;
            let show_workspace =
                MenuItemBuilder::with_id("show_workspace", "Show Workspace in Finder")
                    .accelerator("CmdOrCtrl+Shift+O")
                    .build(app_handle)?;

            let file_menu = SubmenuBuilder::new(app_handle, "File")
                .item(&open_sb3)
                .item(&export_sb3)
                .separator()
                .item(&show_workspace)
                .build()?;

            let edit_menu = SubmenuBuilder::new(app_handle, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let view_menu = SubmenuBuilder::new(app_handle, "View")
                .fullscreen()
                .build()?;

            let window_menu = SubmenuBuilder::new(app_handle, "Window")
                .minimize()
                .close_window()
                .build()?;

            let menu = MenuBuilder::new(app_handle)
                .item(&file_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .item(&window_menu)
                .build()?;

            app.set_menu(menu)?;

            // Handle menu events
            let app_handle_clone = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                let id = event.id().0.as_str();
                match id {
                    "open_sb3" => {
                        let handle = app_handle_clone.clone();
                        std::thread::spawn(move || {
                            let state = handle.state::<WorkspacePath>();
                            use tauri_plugin_dialog::DialogExt;

                            let file_path = handle
                                .dialog()
                                .file()
                                .add_filter("Scratch 3 Project", &["sb3"])
                                .blocking_pick_file();

                            if let Some(file_resp) = file_path {
                                let Some(path) = file_resp.as_path() else { return; };
                                if let Ok(data) = fs::read(path) {
                                    watcher::set_ignore(true);
                                    workspace::extract_sb3(&state.0, &data);

                                    if let Some(sb3) = workspace::build_sb3(&state.0) {
                                        let encoded = base64::Engine::encode(
                                            &base64::engine::general_purpose::STANDARD,
                                            &sb3,
                                        );
                                        let _ = handle.emit("sb3-updated", &encoded);
                                    }

                                    std::thread::spawn(|| {
                                        std::thread::sleep(std::time::Duration::from_millis(1000));
                                        watcher::set_ignore(false);
                                    });
                                }
                            }
                        });
                    }
                    "export_sb3" => {
                        let handle = app_handle_clone.clone();
                        std::thread::spawn(move || {
                            let state = handle.state::<WorkspacePath>();
                            use tauri_plugin_dialog::DialogExt;

                            let file_path = handle
                                .dialog()
                                .file()
                                .add_filter("Scratch 3 Project", &["sb3"])
                                .set_file_name("project.sb3")
                                .blocking_save_file();

                            if let Some(file_resp) = file_path {
                                if let Some(sb3) = workspace::build_sb3(&state.0) {
                                    if let Some(path) = file_resp.as_path() {
                                        let _ = fs::write(path, &sb3);
                                        log::info!("[live-scratch] exported SB3 to {:?}", path);
                                    }
                                }
                            }
                        });
                    }
                    "show_workspace" => {
                        let handle = app_handle_clone.clone();
                        let state = handle.state::<WorkspacePath>();
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open")
                                .arg(&*state.0)
                                .spawn();
                        }
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_initial_sb3,
            commands::save_project_from_editor,
            commands::get_workspace_path,
            commands::open_sb3_file,
            commands::export_sb3_file,
            commands::open_workspace_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
