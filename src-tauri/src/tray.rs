use tauri::{
    AppHandle, CustomMenuItem, Manager, Runtime, SystemTray, SystemTrayEvent, SystemTrayMenu,
};

use crate::commands::{get_autostart_status, set_autostart};

pub fn create_tray() -> SystemTray {
    let open = CustomMenuItem::new("open".to_string(), "Open");
    let autostart = CustomMenuItem::new("autostart".to_string(), "Start with system");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(open)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(autostart)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    SystemTray::new().with_menu(tray_menu)
}

pub fn handle_tray_event<R: Runtime>(app: &AppHandle<R>, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            if let Some(window) = app.get_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "open" => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "autostart" => {
                    // Toggle autostart
                    match get_autostart_status() {
                        Ok(current_status) => {
                            if let Err(e) = set_autostart(!current_status) {
                                tracing::error!("Failed to change autostart: {}", e);
                            } else {
                                // Update tray menu
                                update_tray_menu(app, !current_status);
                            }
                        }
                        Err(e) => tracing::error!("Failed to check autostart: {}", e),
                    }
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub fn update_tray_menu<R: Runtime>(app: &AppHandle<R>, autostart_enabled: bool) {
    let open = CustomMenuItem::new("open".to_string(), "Open");
    let autostart_text = if autostart_enabled {
        "✓ Start with system"
    } else {
        "Start with system"
    };
    let autostart = CustomMenuItem::new("autostart".to_string(), autostart_text);
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    
    let tray_menu = SystemTrayMenu::new()
        .add_item(open)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(autostart)
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(quit);
    
    if let Err(e) = app.tray_handle().set_menu(tray_menu) {
        tracing::error!("Failed to update tray menu: {}", e);
    }
}
