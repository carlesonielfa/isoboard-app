use super::NotificationPermission;
use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::{define_class, msg_send, rc::Retained, runtime::ProtocolObject, AnyThread};
use objc2_foundation::{NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent, UNNotification,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationSettings,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::{cell::RefCell, ptr::NonNull, sync::Mutex};
use tokio::sync::oneshot;

fn available() -> bool {
    // An embedded identifier alone does not provide the app bundle proxy required by macOS.
    let bundle = NSBundle::mainBundle();
    bundle.bundleIdentifier().is_some() && bundle.bundlePath().to_string().ends_with(".app")
}

pub(super) fn open_settings() -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not open notification settings.".into())
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[name = "IsoboardNotificationDelegate"]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion
                .call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
        }
    }
);

thread_local! {
    // The notification center holds its delegate weakly.
    static DELEGATE: RefCell<Option<Retained<NotificationDelegate>>> = const { RefCell::new(None) };
}

pub(super) fn setup() {
    if !available() {
        return;
    }
    let delegate: Retained<NotificationDelegate> =
        unsafe { msg_send![NotificationDelegate::alloc(), init] };
    UNUserNotificationCenter::currentNotificationCenter()
        .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    DELEGATE.with(|retained| *retained.borrow_mut() = Some(delegate));
}

pub(super) async fn permission() -> Result<NotificationPermission, String> {
    if !available() {
        return Ok(NotificationPermission::Unsupported);
    }
    let (send, receive) = oneshot::channel();
    {
        let send = Mutex::new(Some(send));
        let callback = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
            // The notification center keeps settings alive for this callback.
            let status = unsafe { settings.as_ref() }.authorizationStatus();
            let status = match status {
                UNAuthorizationStatus::Authorized | UNAuthorizationStatus::Provisional => {
                    NotificationPermission::Granted
                }
                UNAuthorizationStatus::NotDetermined => NotificationPermission::Required,
                _ => NotificationPermission::Denied,
            };
            if let Some(send) = send.lock().expect("notification callback lock").take() {
                let _ = send.send(status);
            }
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationSettingsWithCompletionHandler(&callback);
    }
    receive.await.map_err(|error| error.to_string())
}

pub(super) async fn request_permission() -> Result<NotificationPermission, String> {
    if !available() {
        return Ok(NotificationPermission::Unsupported);
    }
    let (send, receive) = oneshot::channel();
    {
        let send = Mutex::new(Some(send));
        let callback = RcBlock::new(move |_: Bool, error: *mut NSError| {
            // A non-null error is valid for the duration of the native callback.
            let result = unsafe { error.as_ref() }.map_or(Ok(()), |error| {
                let message = format!(
                    "{} ({} {})",
                    error.localizedDescription(),
                    error.domain(),
                    error.code()
                );
                eprintln!("isoboard: notification authorization failed: {message}");
                Err(message)
            });
            if let Some(send) = send.lock().expect("notification callback lock").take() {
                let _ = send.send(result);
            }
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert,
                &callback,
            );
    }
    receive.await.map_err(|error| error.to_string())??;
    permission().await
}

pub(super) fn model_downloaded(model: &str) -> Result<(), String> {
    if !available() {
        return Ok(());
    }
    let name = crate::local_models::definition(model)
        .map(|definition| definition.name.as_str())
        .ok_or("Unknown local model.")?;
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str("Model download complete"));
    content.setBody(&NSString::from_str(&format!(
        "{name} is ready to use in isoboard."
    )));
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
        &NSString::from_str(&format!("model-download-{model}")),
        &content,
        None,
    );
    let callback = RcBlock::new(|error: *mut NSError| {
        // A non-null error is valid for the duration of the native callback.
        if let Some(error) = unsafe { error.as_ref() } {
            eprintln!(
                "isoboard: notification delivery failed: {}",
                error.localizedDescription()
            );
        }
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .addNotificationRequest_withCompletionHandler(&request, Some(&callback));
    Ok(())
}
