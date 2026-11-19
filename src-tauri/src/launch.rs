use std::path::{Path, PathBuf};

fn marker(config_path: &Path) -> PathBuf {
    config_path.with_file_name("last-opened-version")
}

fn pending_open_marker(config_path: &Path) -> PathBuf {
    config_path.with_file_name("open-on-next-launch")
}

pub(crate) fn open_on_next_launch(config_path: &Path) -> std::io::Result<()> {
    let path = pending_open_marker(config_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, [])
}

pub(crate) fn should_open(
    config_path: &Path,
    version: &str,
    needs_setup: bool,
    login_launch: bool,
) -> bool {
    pending_open_marker(config_path).is_file()
        || !login_launch
            && (needs_setup
                || std::fs::read_to_string(marker(config_path)).ok().as_deref() != Some(version))
}

pub(crate) fn remember_opened(config_path: &Path, version: &str) -> std::io::Result<()> {
    let path = marker(config_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, version)?;
    match std::fs::remove_file(pending_open_marker(config_path)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_first_launch_and_new_versions_without_consuming_login_launches() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        assert!(!should_open(&path, "0.5.0", false, true));
        assert!(should_open(&path, "0.5.0", false, false));
        remember_opened(&path, "0.5.0").unwrap();
        assert!(!should_open(&path, "0.5.0", false, false));
        assert!(should_open(&path, "0.5.0", true, false));
        assert!(!should_open(&path, "0.6.0", true, true));
        assert!(should_open(&path, "0.6.0", false, false));
        remember_opened(&path, "0.6.0").unwrap();
        assert!(!should_open(&path, "0.6.0", false, false));
    }

    #[test]
    fn explicit_return_opens_a_configured_app_once_even_with_retained_login_arguments() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        remember_opened(&path, "0.5.0").unwrap();
        assert!(!should_open(&path, "0.5.0", false, false));

        open_on_next_launch(&path).unwrap();
        assert!(should_open(&path, "0.5.0", false, false));
        assert!(should_open(&path, "0.5.0", false, true));

        remember_opened(&path, "0.5.0").unwrap();
        assert!(!should_open(&path, "0.5.0", false, false));
        assert!(!should_open(&path, "0.5.0", false, true));
    }
}
