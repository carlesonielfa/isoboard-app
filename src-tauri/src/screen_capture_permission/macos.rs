pub(super) fn granted() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

pub(super) async fn request() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| unsafe {
        CGRequestScreenCaptureAccess();
    })
    .await
    .map_err(|error| format!("Could not request Screen Recording access: {error}"))
}

pub(super) fn fresh_process_granted() -> Result<bool, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let output = std::process::Command::new(executable)
        .arg(super::PROBE_ARGUMENT)
        .output()
        .map_err(|error| format!("Could not check updated screen access: {error}"))?;
    if !output.status.success() {
        return Err("Could not check updated screen access.".into());
    }
    match output.stdout.as_slice() {
        b"granted\n" => Ok(true),
        b"required\n" => Ok(false),
        _ => Err("The screen access check returned an unexpected response.".into()),
    }
}

pub(super) fn open_settings() -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .status()
        .map_err(|error| format!("Could not open Screen Recording settings: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not open Screen Recording settings.".into())
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}
