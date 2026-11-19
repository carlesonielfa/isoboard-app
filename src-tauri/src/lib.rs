use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

mod app_sounds;
mod application_config;
mod builtin_inference_profiles;
mod capture;
mod capture_feedback;
mod capture_overlay;
mod changelog;
pub(crate) mod clipboard;
mod config;
mod config_jsonc;
pub mod contracts;
mod history;
mod inference_profiles;
mod launch;
mod licensing;
pub mod local_models;
pub mod managed_credentials;
mod model_catalog;
mod notifications;
mod product_config;
mod profiles;
mod rendering;
mod resource_usage;
mod screen_capture_permission;
mod tray_activity;
mod tray_menu_icons;
mod updates;
mod window_presentation;

#[cfg(feature = "api-probe")]
pub mod api_credentials;
pub mod inference;
pub mod secret_store;

fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Err(error) = window_presentation::reveal_main_window(app) {
        eprintln!("isoboard: could not reveal main window: {error}");
    }
}

fn show_history_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Err(error) = history::open_history_window(app, None) {
        eprintln!("isoboard: could not open History tab: {error}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if screen_capture_permission::run_probe_if_requested() {
        return;
    }
    let login_launch = std::env::args_os()
        .any(|argument| argument == std::ffi::OsStr::new("--isoboard-login-start"));
    let contract_builder = contracts::builder();
    let contract_handler = contract_builder.invoke_handler();
    let builder = tauri::Builder::default();

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--isoboard-login-start"]),
        ))
        .plugin(config::global_shortcut_plugin(|app, native_id| {
            let context = {
                let config_state = app.state::<config::SharedConfig>();
                let config = config_state.lock().expect("config lock poisoned");
                if config.is_open_shortcut_id(native_id) {
                    drop(config);
                    show_main_window(app);
                    return;
                }
                let Some(invocation) = config.resolve_capture_shortcut_id(native_id) else {
                    return;
                };
                config
                    .resolve_capture_context(&invocation.profile_id, Some(invocation.binding_id))
                    .expect("capture binding profile must exist")
            };
            let coordinator = app.state::<capture::CaptureCoordinator>().inner().clone();
            let inference = app
                .state::<std::sync::Arc<inference::InferenceService>>()
                .inner()
                .clone();
            let clipboard = app
                .state::<std::sync::Arc<dyn clipboard::ClipboardWriter>>()
                .inner()
                .clone();
            if let Err(error) = capture::start_region_capture_with_context(
                app.clone(),
                coordinator,
                context,
                inference,
                clipboard,
            ) {
                eprintln!("isoboard: could not start shortcut capture: {error}");
            }
        }))
        .manage(capture::CaptureCoordinator::default())
        .manage(history::PendingHistoryState::default())
        .manage(history::HistoryNavigationState::default())
        .manage(capture_overlay::CaptureOverlay::default())
        .manage(app_sounds::AppSounds::new())
        .manage(config::ShortcutRecordingState::default())
        .manage(updates::UpdateManager::new())
        .register_asynchronous_uri_scheme_protocol(
            history::THUMBNAIL_PROTOCOL,
            |context, request, responder| {
                let store = context
                    .app_handle()
                    .try_state::<std::sync::Arc<dyn history::HistoryStore>>()
                    .map(|state| state.inner().clone());
                tauri::async_runtime::spawn_blocking(move || {
                    let response = match store {
                        Some(store) => history::thumbnail_protocol_response(&*store, &request),
                        None => tauri::http::Response::builder()
                            .status(tauri::http::StatusCode::SERVICE_UNAVAILABLE)
                            .body(Vec::new())
                            .expect("static history thumbnail response is valid"),
                    };
                    responder.respond(response);
                });
            },
        )
        .setup(move |app| {
            contract_builder.mount_events(app);
            notifications::setup();
            #[cfg(target_os = "macos")]
            if let Err(error) = app
                .handle()
                .set_activation_policy(tauri::ActivationPolicy::Accessory)
            {
                eprintln!("isoboard: could not set macOS accessory activation policy: {error}");
            }

            config::setup(app)?;
            let config_path = app
                .state::<config::SharedConfig>()
                .lock()
                .expect("config lock poisoned")
                .config_path()
                .to_path_buf();
            let history = history::open_for_config(&config_path)
                .map_err(|error| format!("could not initialize clipboard history: {error}"))?;
            let snapshot = app
                .state::<config::SharedConfig>()
                .lock()
                .expect("config lock poisoned")
                .snapshot();
            let credentials = app.state::<config::ManagedCredentialsState>().0.clone();
            let commercial =
                licensing::commercial::CommercialService::open(&config_path, credentials.clone());
            let handle = app.handle().clone();
            commercial.set_publisher(std::sync::Arc::new(move || {
                use tauri_specta::Event;
                let _ = licensing::commercial::LicenseChangedEvent {}.emit(&handle);
            }));
            app.manage(commercial.clone());
            let handle = app.handle().clone();
            commercial.start_background(std::sync::Arc::new(move || {
                handle.state::<capture::CaptureCoordinator>().is_busy()
            }));
            let license_store = licensing::LicenseStore::open(&config_path, !login_launch);
            let needs_initial_setup = snapshot.needs_initial_setup()
                || licensing::needs_attention(&license_store, &commercial);
            app.manage(std::sync::Mutex::new(license_store));
            let capture_status = app.state::<capture::CaptureCoordinator>().snapshot();
            let menu = history::build_tray_menu(
                app.handle(),
                history.as_ref(),
                &snapshot,
                &capture_status,
            )
            .map_err(|error| format!("could not build history menu: {error}"))?;
            app.manage(history);
            let inference = inference::InferenceService::new(credentials)
                .map_err(|error| format!("could not initialize inference service: {error}"))?;
            #[cfg(feature = "local-inference")]
            let inference = inference
                .with_local(local_models::ModelStore::for_config(&config_path))
                .map_err(|error| format!("could not initialize local inference worker: {error}"))?;
            #[cfg(feature = "local-inference")]
            if let Some(worker) = inference.local_worker() {
                worker
                    .models()
                    .set_download_completion_handler(std::sync::Arc::new(
                        notifications::model_downloaded,
                    ));
                let handle = app.handle().clone();
                worker
                    .models()
                    .set_publisher(std::sync::Arc::new(move |snapshot| {
                        use tauri_specta::Event;
                        if let Err(error) = snapshot.emit(&handle) {
                            eprintln!("isoboard: could not emit model state: {error}");
                        }
                    }));
                app.manage(worker.models().clone());
            }
            app.manage(std::sync::Arc::new(inference));
            app.manage(clipboard::system(app.handle().clone()));

            let catalog_cache_path = match app.path().app_cache_dir() {
                Ok(directory) => Some(model_catalog::ModelCatalogService::default_cache_path(
                    &directory,
                )),
                Err(error) => {
                    eprintln!("isoboard: could not resolve model catalog cache directory: {error}");
                    None
                }
            };
            let catalog =
                std::sync::Arc::new(model_catalog::ModelCatalogService::new(catalog_cache_path));
            catalog.load_cache();
            catalog.emit_changed(app.handle());
            app.manage(catalog.clone());
            catalog.start_refresh(app.handle().clone());

            let idle_icon = app
                .default_window_icon()
                .map(|icon| icon.clone().to_owned());
            let icon = idle_icon.clone();
            let mut tray = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("isoboard")
                .on_menu_event(|app, event| {
                    let id = event.id().as_ref();
                    match id {
                        "capture-region" => {
                            if let Err(error) = capture::start_active_region_capture(app.clone()) {
                                eprintln!("isoboard: could not start menu capture: {error}");
                            }
                        }
                        "open" => show_main_window(app),
                        "view-all-history" => show_history_window(app),
                        "install-update" => {
                            let manager = app.state::<std::sync::Arc<updates::UpdateManager>>();
                            if let Err(error) = manager.install_and_restart(app) {
                                eprintln!("isoboard: could not install update: {error}");
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {
                            if let Some(profile_id) = history::profile_id_from_menu(id) {
                                if let Err(error) =
                                    capture::start_profile_region_capture(app.clone(), profile_id)
                                {
                                    eprintln!("isoboard: could not start profile capture: {error}");
                                }
                                // Native check items toggle on click; rebuild so the
                                // checkmark keeps marking the active Profile.
                                if let Err(error) = history::refresh_tray_menu(app) {
                                    eprintln!("isoboard: could not refresh tray menu: {error}");
                                }
                            } else if let Some(history_id) = history::history_id_from_menu(id) {
                                history::restore_from_tray(app, history_id);
                            }
                        }
                    }
                })
                .on_tray_icon_event(|app, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(app.app_handle());
                    }
                });

            if let Some(icon) = icon {
                tray = tray.icon(icon).icon_as_template(true);
            }

            let tray = tray.build(app)?;
            if let Err(error) = tray_menu_icons::set_menu(&tray, menu, capture_status) {
                eprintln!("isoboard: could not set menu icons: {error}");
            }
            let activity = tray_activity::TrayActivityController::new(tray, idle_icon)
                .map_err(|error| format!("could not initialize tray activity: {error}"))?;
            app.manage(activity);
            app.state::<std::sync::Arc<updates::UpdateManager>>()
                .start_background(app.handle().clone());
            let version = app.package_info().version.to_string();
            if launch::should_open(&config_path, &version, needs_initial_setup, login_launch) {
                match window_presentation::reveal_main_window(app.handle()) {
                    Ok(()) => {
                        if let Err(error) = launch::remember_opened(&config_path, &version) {
                            eprintln!("isoboard: could not remember launch version: {error}");
                        }
                    }
                    Err(error) => eprintln!("isoboard: could not reveal main window: {error}"),
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if matches!(
                event,
                WindowEvent::Focused(false) | WindowEvent::CloseRequested { .. }
            ) {
                let recording_state = window
                    .app_handle()
                    .state::<config::ShortcutRecordingState>();
                if let Some(config) = window.app_handle().try_state::<config::SharedConfig>() {
                    recording_state.set_recording(true);
                    let result = config
                        .lock()
                        .expect("config lock poisoned")
                        .resume_capture_shortcut();
                    match result {
                        Ok(_) => recording_state.set_recording(false),
                        Err(error) => {
                            eprintln!("isoboard: could not restore capture shortcut: {error}");
                        }
                    }
                } else {
                    recording_state.set_recording(false);
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("isoboard: could not hide main window: {error}");
                }
            }
        })
        .invoke_handler(contract_handler)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Some(service) =
                    app.try_state::<std::sync::Arc<inference::InferenceService>>()
                {
                    service.shutdown_local();
                }
            }
        });
}
