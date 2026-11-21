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
    use std::{cell::RefCell, ptr::NonNull};

    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::ProtocolObject, AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSImage, NSMenu, NSMenuDidBeginTrackingNotification, NSStatusItem};
    use objc2_foundation::{
        NSData, NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue, NSSize,
    };
    use tauri::menu::MenuItemKind;

    use super::*;

    /// Artwork for one menu row, mirroring the menu's item order.
    struct Row {
        svg: Option<&'static [u8]>,
        children: Vec<Row>,
    }

    /// The installed tray menu's artwork and the status item that shows it.
    struct Installed {
        status_item: Retained<NSStatusItem>,
        rows: Vec<Row>,
    }

    thread_local! {
        // Main-thread only: AppKit objects and the tracking observer live here.
        static INSTALLED: RefCell<Option<Installed>> = const { RefCell::new(None) };
        static OBSERVER: RefCell<Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>> =
            const { RefCell::new(None) };
    }

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

    fn rows<R: Runtime>(
        items: Vec<MenuItemKind<R>>,
        status: &CaptureStatus,
    ) -> Result<Vec<Row>, String> {
        items
            .into_iter()
            .map(|item| {
                let children = match &item {
                    MenuItemKind::Submenu(submenu) => {
                        rows(submenu.items().map_err(|error| error.to_string())?, status)?
                    }
                    _ => Vec::new(),
                };
                Ok(Row {
                    svg: artwork(item.id().as_ref(), status),
                    children,
                })
            })
            .collect()
    }

    fn decorate(native: &NSMenu, rows: &[Row]) -> Result<(), String> {
        let native_items = native.itemArray();
        if native_items.len() != rows.len() {
            return Err("Native menu items do not match the tray menu.".into());
        }
        for (native_item, row) in native_items.iter().zip(rows) {
            if let Some(svg) = row.svg {
                let data = NSData::with_bytes(svg);
                let image = NSImage::initWithData(NSImage::alloc(), &data)
                    .ok_or_else(|| "Could not load a menu icon.".to_string())?;
                image.setSize(NSSize::new(16.0, 16.0));
                image.setTemplate(true);
                native_item.setImage(Some(&image));
            }
            if let Some(native_submenu) = native_item.submenu() {
                decorate(&native_submenu, &row.children)?;
            }
        }
        Ok(())
    }

    /// Decorates the tray menu as AppKit starts tracking it. Tauri's tray keeps
    /// the native menu private and attaches it to the status item only while
    /// the menu is open, so the menu is reachable from the status item here.
    fn on_menu_tracking(notification: &NSNotification, mtm: MainThreadMarker) {
        let Some(object) = notification.object() else {
            return;
        };
        INSTALLED.with_borrow(|installed| {
            let Some(installed) = installed else {
                return;
            };
            let Some(menu) = installed.status_item.menu(mtm) else {
                return;
            };
            let is_tray_menu = std::ptr::eq(
                Retained::as_ptr(&menu).cast::<()>(),
                Retained::as_ptr(&object).cast::<()>(),
            );
            if !is_tray_menu {
                return;
            }
            if let Err(error) = decorate(&menu, &installed.rows) {
                eprintln!("isoboard: could not set menu icons: {error}");
            }
        });
    }

    fn observe_menu_tracking() {
        OBSERVER.with_borrow_mut(|observer| {
            if observer.is_some() {
                return;
            }
            let block = RcBlock::new(|notification: NonNull<NSNotification>| {
                // The observer runs on the main queue.
                if let Some(mtm) = MainThreadMarker::new() {
                    on_menu_tracking(unsafe { notification.as_ref() }, mtm);
                }
            });
            // SAFETY: The block captures nothing and runs on the main queue,
            // and the observer is retained for the life of the app.
            *observer = Some(unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(NSMenuDidBeginTrackingNotification),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &block,
                )
            });
        });
    }

    pub(super) fn set_menu<R: Runtime>(
        tray: &TrayIcon<R>,
        menu: Menu<R>,
        status: CaptureStatus,
    ) -> Result<(), String> {
        let handle = tray.clone();
        tray.with_inner_tray_icon(move |inner| {
            // Tauri runs this closure on the main thread. Install the menu and
            // record its artwork together so concurrent refreshes cannot pair
            // one menu's rows with another menu's native items.
            handle
                .set_menu(Some(menu.clone()))
                .map_err(|error| error.to_string())?;
            let status_item = inner
                .ns_status_item()
                .ok_or_else(|| "Native tray status item is unavailable.".to_string())?;
            let rows = rows(menu.items().map_err(|error| error.to_string())?, &status)?;
            observe_menu_tracking();
            INSTALLED.set(Some(Installed { status_item, rows }));
            Ok(())
        })
        .map_err(|error| error.to_string())?
    }
}
