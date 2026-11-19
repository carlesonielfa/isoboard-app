use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppMemoryUsage {
    resident_bytes: Option<u64>,
}

fn resident_memory() -> Option<u64> {
    let pid = Pid::from_u32(std::process::id());
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory().without_tasks(),
    );
    system.process(pid).map(|process| process.memory())
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn get_app_memory_usage() -> AppMemoryUsage {
    let resident_bytes = tauri::async_runtime::spawn_blocking(resident_memory)
        .await
        .ok()
        .flatten();
    AppMemoryUsage { resident_bytes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_resident_memory_reads_the_current_process() {
        let bytes = resident_memory().expect("current process memory is available");
        assert!(bytes > 0);
        eprintln!("Current process resident RAM: {bytes} bytes");
    }
}
