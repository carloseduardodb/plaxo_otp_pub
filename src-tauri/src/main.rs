#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod google_drive;
mod otp;
mod qr;
mod state;
mod storage;
mod sync;
mod tray;
mod types;

use state::AppState;
use tray::{create_tray, handle_tray_event, update_tray_menu};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = AppState::new();

    tauri::Builder::<tauri::Wry>::new()
        .manage(state)
        .system_tray(create_tray())
        .on_system_tray_event(handle_tray_event)
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = event.window().hide();
                api.prevent_close();
            }
            _ => {}
        })
        .setup(|app| {
            // Initialize tray menu with correct autostart status
            let autostart_enabled = commands::get_autostart_status().unwrap_or(false);
            update_tray_menu(&app.handle(), autostart_enabled);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::has_master_password,
            commands::verify_master_password,
            commands::get_apps,
            commands::add_app,
            commands::edit_app_name,
            commands::delete_app,
            commands::generate_otp,
            commands::copy_to_clipboard,
            commands::import_2fas_file,
            commands::decode_qr_from_image,
            commands::decode_qr_from_clipboard,
            commands::set_autostart,
            commands::get_autostart_status,
            commands::reset_master_password,
            commands::google_drive_is_configured,
            commands::google_drive_auth_flow,
            commands::sync_with_google_drive,
            commands::restore_from_google_drive,
            commands::check_google_auth,
            commands::clear_google_auth,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
