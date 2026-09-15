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

use tauri::Manager;

use state::AppState;
use tray::{create_tray, handle_tray_event, update_tray_menu};

/// How often the auto-lock timer checks for idleness.
const LOCK_CHECK_INTERVAL_SECS: u64 = 15;

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

            // The app lives in the tray, so an unlocked vault would otherwise
            // stay unlocked for as long as the machine is on.
            let handle = app.handle();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(LOCK_CHECK_INTERVAL_SECS));

                let state = handle.state::<AppState>();
                if !state.has_master_password() {
                    continue;
                }

                if state.idle_secs() >= commands::AUTO_LOCK_SECS {
                    state.lock();
                    tracing::info!("Vault auto-locked after inactivity");
                    // Tell the UI to drop back to the password prompt; without
                    // this it would keep rendering stale codes from its own
                    // React state.
                    let _ = handle.emit_all("vault-locked", ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::has_master_password,
            commands::verify_master_password,
            commands::touch_activity,
            commands::lock_vault,
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
