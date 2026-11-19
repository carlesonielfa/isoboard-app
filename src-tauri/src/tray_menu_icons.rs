//! SVG menu artwork with native macOS template tinting.

use crate::capture::CaptureStatus;
use tauri::{menu::Menu, tray::TrayIcon, Runtime};

pub(crate) fn set_menu<R: Runtime>(
    tray: &TrayIcon<R>,
    menu: Menu<R>,
    status: CaptureStatus,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return macos::set_menu(tray, menu, status);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = status;
        tray.set_menu(Some(menu)).map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSImage, NSMenu};
    use objc2_foundation::{NSData, NSSize};
    use tauri::menu::MenuItemKind;

    use super::*;

    fn artwork(id: &str, status: &CaptureStatus) -> Option<&'static [u8]> {
        Some(match id {
            "capture-region" | "capture-profile" => include_bytes!("../icons/menu/capture.svg"),
            "open" => include_bytes!("../icons/menu/app-window-mac.svg"),
            "history" => include_bytes!("../icons/menu/history.svg"),
            "view-all-history" => include_bytes!("../icons/menu/list.svg"),
            "quit" => include_bytes!("../icons/menu/power.svg"),
            "capture-status" => match status {
                CaptureStatus::Idle { .. } => include_bytes!("../icons/menu/info.svg"),
                CaptureStatus::Started { .. } => include_bytes!("../icons/menu/capture.svg"),
                CaptureStatus::ImageReady { .. }
                | CaptureStatus::DownloadingModel { .. }
                | CaptureStatus::LoadingModel { .. }
                | CaptureStatus::Processing { .. } => {
                    include_bytes!("../icons/menu/processing.svg")
                }
                CaptureStatus::Completed { .. } => include_bytes!("../icons/menu/success.svg"),
                CaptureStatus::Cancelled { .. } => include_bytes!("../icons/menu/cancelled.svg"),
                CaptureStatus::Failed { .. } => include_bytes!("../icons/menu/error.svg"),
            },
            id if crate::history::history_id_from_menu(id).is_some() => {
                include_bytes!("../icons/menu/clipboard.svg")
            }
            _ => return None,
        })
    }

    fn decorate<R: Runtime>(
        native: &NSMenu,
        items: Vec<MenuItemKind<R>>,
        status: &CaptureStatus,
    ) -> Result<(), String> {
        let native_items = native.itemArray();
        if native_items.len() != items.len() {
            return Err("Native menu items do not match the tray menu.".into());
        }
        for (native_item, item) in native_items.iter().zip(items) {
            if let Some(svg) = artwork(item.id().as_ref(), status) {
                let data = NSData::with_bytes(svg);
                let image = NSImage::initWithData(NSImage::alloc(), &data).ok_or_else(|| {
                    format!("Could not load menu icon for {}.", item.id().as_ref())
                })?;
                image.setSize(NSSize::new(16.0, 16.0));
                image.setTemplate(true);
                native_item.setImage(Some(&image));
            }
            if let MenuItemKind::Submenu(submenu) = item {
                if let Some(native_submenu) = native_item.submenu() {
                    decorate(
                        &native_submenu,
                        submenu.items().map_err(|error| error.to_string())?,
                        status,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn set_menu<R: Runtime>(
        tray: &TrayIcon<R>,
        menu: Menu<R>,
        status: CaptureStatus,
    ) -> Result<(), String> {
        let handle = tray.clone();
        tray.with_inner_tray_icon(move |inner| {
            // Install and decorate together so concurrent refreshes cannot pair
            // one menu's item order with another menu's native objects.
            handle
                .set_menu(Some(menu.clone()))
                .map_err(|error| error.to_string())?;
            // Tauri runs this closure on the main thread. Borrow native objects
            // through its status item so their ownership stays with the tray.
            let mtm = MainThreadMarker::new()
                .ok_or_else(|| "Menu icons require the main thread.".to_string())?;
            let native = inner
                .ns_status_item()
                .and_then(|item| item.menu(mtm))
                .ok_or_else(|| "Native tray menu is unavailable.".to_string())?;
            decorate(
                &native,
                menu.items().map_err(|error| error.to_string())?,
                &status,
            )
        })
        .map_err(|error| error.to_string())?
    }
}
