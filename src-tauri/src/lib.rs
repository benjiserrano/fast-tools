//! Núcleo de fast-tools.
//!
//! El binario (`main.rs`) solo llama a [`run`]. Toda la lógica vive aquí para
//! que los tests puedan enlazar contra la librería.

mod commands;
mod convert;
mod engines;
mod error;
mod portable;
mod registry;
mod tools;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const WINDOW_LABEL: &str = "main";
const OPEN_SEARCH_EVENT: &str = "open-search";
const SEARCH_SHORTCUT: &str = "Ctrl+Alt+T";

pub fn run() {
    // Resolver el almacenamiento antes de levantar la ventana: si hay que
    // degradar a LOCALAPPDATA, la UI debe poder avisarlo en el primer render.
    let storage = portable::storage();
    if let Some(reason) = &storage.degraded_reason {
        eprintln!("[Fast tools] modo no portable ({:?}): {reason}", storage.mode);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Err(error) = show_search(app) {
                            eprintln!("[Fast tools] no se pudo abrir el buscador: {error}");
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::list_tools,
            commands::get_tool,
            commands::app_info,
            commands::reveal_data_dir,
            commands::convert::list_formats,
            commands::convert::convert_targets,
            commands::convert::convert_text,
            commands::convert::detect_format,
            commands::convert::convert_files,
            commands::convert::load_text_file,
            commands::convert::save_text_file,
            commands::tools::format_document,
            commands::tools::format_sql,
            commands::tools::diff_text,
            commands::tools::list_hash_algorithms,
            commands::tools::hash_text,
            commands::tools::hash_file,
            commands::tools::verify_checksum,
            commands::tools::hmac_text,
            commands::tools::list_id_kinds,
            commands::tools::generate_ids,
            commands::tools::inspect_jwt,
            commands::tools::generate_passwords,
            commands::tools::hash_password,
            commands::tools::verify_password,
            commands::tools::inspect_certificate,
            commands::images::list_image_formats,
            commands::images::load_image,
            commands::images::process_image,
            commands::images::save_processed_image,
            commands::images::favicon_sizes,
            commands::images::generate_favicon,
            commands::images::extract_palette,
            commands::images::read_exif,
            commands::images::strip_exif,
            commands::images::qr_correction_levels,
            commands::images::generate_qr,
            commands::images::save_qr,
            commands::engines::list_engines,
            commands::engines::install_engine,
            commands::engines::remove_engine,
            commands::engines::verify_engine,
            commands::engines::list_doc_formats,
            commands::engines::list_media_formats,
            commands::engines::convert_documents,
            commands::engines::convert_media_files,
            commands::net::test_regex,
            commands::net::replace_regex,
            commands::net::parse_cron,
            commands::net::http_methods,
            commands::net::send_http,
        ])
        .setup(|app| {
            build_main_window(app)?;
            build_tray(app)?;
            app.global_shortcut().register(SEARCH_SHORTCUT)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != WINDOW_LABEL {
                return;
            }

            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar la ventana de Fast tools");
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open-search", "Abrir buscador", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    let mut tray = TrayIconBuilder::new()
        .tooltip("Fast tools - Ctrl+Alt+T")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open-search" => {
                if let Err(error) = show_search(app) {
                    eprintln!("[Fast tools] no se pudo abrir el buscador: {error}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                if let Err(error) = show_search(tray.app_handle()) {
                    eprintln!("[Fast tools] no se pudo abrir el buscador: {error}");
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

fn show_search(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
        app.emit_to(WINDOW_LABEL, OPEN_SEARCH_EVENT, ())?;
    }
    Ok(())
}

/// La ventana se crea aquí y no en `tauri.conf.json` porque en Windows hay que
/// fijarle el directorio de datos de WebView2.
///
/// Por defecto Tauri lo sitúa en `%LOCALAPPDATA%\<identificador>\EBWebView`, es
/// decir fuera de la carpeta portable: la caché, las cookies y el localStorage
/// se quedarían en la máquina en lugar de viajar con el ejecutable. Eso rompe
/// la promesa del producto, así que se redirige junto al resto de los datos.
fn build_main_window(app: &tauri::App) -> tauri::Result<()> {
    let mut builder = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::default())
        .title("Fast tools")
        .inner_size(1180.0, 780.0)
        .min_inner_size(900.0, 560.0)
        .decorations(false)
        .resizable(true)
        .center();

    #[cfg(windows)]
    {
        let webview_dir = portable::data_dir().join("webview2");
        std::fs::create_dir_all(&webview_dir)?;
        builder = builder.data_directory(webview_dir);
    }

    builder.build()?;
    Ok(())
}
