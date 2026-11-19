use super::*;
use crate::application_config::APPLICATION;
use crate::profiles::{
    ConversionSettings, ConversionSettingsApplicationMode, ProcessingProfile,
    ProcessingProfileDraft,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct RuntimeState {
    active: HashSet<String>,
    blocked_register: HashSet<String>,
    blocked_unregister: HashSet<String>,
    calls: Vec<String>,
}

struct FakeRuntime(Arc<Mutex<RuntimeState>>);

impl ShortcutRuntime for FakeRuntime {
    fn register(&mut self, shortcut: &str) -> Result<(), String> {
        let mut state = self.0.lock().unwrap();
        state.calls.push(format!("register:{shortcut}"));
        if state.blocked_register.contains(shortcut) {
            return Err("shortcut is already registered".to_string());
        }
        state.active.insert(shortcut.to_string());
        Ok(())
    }

    fn unregister(&mut self, shortcut: &str) -> Result<(), String> {
        let mut state = self.0.lock().unwrap();
        state.calls.push(format!("unregister:{shortcut}"));
        if state.blocked_unregister.contains(shortcut) {
            return Err("shortcut could not be released".to_string());
        }
        state.active.remove(shortcut);
        Ok(())
    }
}

fn test_path() -> PathBuf {
    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "isoboard-bindings-test-{}-{}-{stamp}",
        std::process::id(),
        NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn controller(path: &Path) -> (ConfigController, Arc<Mutex<RuntimeState>>) {
    let runtime_state = Arc::new(Mutex::new(RuntimeState::default()));
    let runtime = Box::new(FakeRuntime(runtime_state.clone()));
    let (controller, startup_error) = ConfigController::load_or_default(path, runtime).unwrap();
    assert!(startup_error.is_none());
    (controller, runtime_state)
}

fn finish(directory: PathBuf) {
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn default_binding_is_active_profile_and_snapshot_uses_binding_list() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    let snapshot = controller.activate().unwrap();
    assert_eq!(snapshot.capture_bindings.len(), 1);
    assert_eq!(snapshot.capture_bindings[0].id, "default");
    assert_eq!(snapshot.capture_bindings[0].profile_id, None);
    assert_eq!(
        controller.resolve_capture_shortcut_id(
            super::super::bindings::native_shortcut_id(DEFAULT_CAPTURE_SHORTCUT).unwrap()
        ),
        Some(super::super::bindings::CaptureInvocation {
            binding_id: "default".to_string(),
            profile_id: "default".to_string(),
        })
    );
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains(DEFAULT_CAPTURE_SHORTCUT));
    let revision = controller.snapshot().revision;
    let updated = controller
        .set_binding(
            "default".to_string(),
            "CommandOrControl+Alt+7".to_string(),
            revision,
        )
        .unwrap();
    assert_eq!(
        updated.capture_bindings[0].shortcut,
        "CommandOrControl+Alt+7"
    );
    assert!(controller
        .delete_binding("default".to_string(), updated.revision)
        .is_err());
    finish(directory);
}

#[test]
fn open_shortcut_is_registered_separately_and_cannot_collide_with_capture() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    let snapshot = controller.activate().unwrap();
    assert_eq!(snapshot.open_shortcut, DEFAULT_OPEN_SHORTCUT);
    assert!(controller.is_open_shortcut_id(
        super::super::bindings::native_shortcut_id(DEFAULT_OPEN_SHORTCUT).unwrap()
    ));
    assert_eq!(runtime.lock().unwrap().active.len(), 2);

    let revision = snapshot.revision;
    let error = controller
        .set_open_shortcut(DEFAULT_CAPTURE_SHORTCUT.to_string(), revision)
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Validation);
    assert_eq!(controller.snapshot().open_shortcut, DEFAULT_OPEN_SHORTCUT);
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains(DEFAULT_OPEN_SHORTCUT));
    finish(directory);
}

#[test]
fn open_shortcut_replacement_is_atomic_when_native_registration_fails() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();

    let first = "CommandOrControl+Shift+Backslash".to_string();
    let snapshot = controller
        .set_open_shortcut(first.clone(), controller.snapshot().revision)
        .unwrap();
    assert_eq!(snapshot.open_shortcut, first);
    assert!(!runtime
        .lock()
        .unwrap()
        .active
        .contains(DEFAULT_OPEN_SHORTCUT));
    assert!(runtime.lock().unwrap().active.contains(&first));

    let second = "CommandOrControl+Shift+BracketLeft".to_string();
    runtime
        .lock()
        .unwrap()
        .blocked_register
        .insert(second.clone());
    let error = controller
        .set_open_shortcut(second.clone(), snapshot.revision)
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Conflict);
    assert_eq!(controller.snapshot().open_shortcut, first);
    assert!(runtime.lock().unwrap().active.contains(&first));
    assert!(!runtime.lock().unwrap().active.contains(&second));
    finish(directory);
}

#[test]
fn ui_mode_defaults_to_basic_and_persists_revision_checked_changes() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut config_controller, _) = controller(&path);
    assert_eq!(config_controller.snapshot().ui_mode, UiMode::Basic);

    let snapshot = config_controller
        .set_ui_mode(UiMode::Advanced, config_controller.snapshot().revision)
        .unwrap();
    assert_eq!(snapshot.ui_mode, UiMode::Advanced);
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("mode = \"advanced\""));

    let (restored, _) = controller(&path);
    assert_eq!(restored.snapshot().ui_mode, UiMode::Advanced);

    let debug_snapshot = config_controller
        .set_ui_mode(UiMode::Debug, snapshot.revision)
        .unwrap();
    assert_eq!(debug_snapshot.ui_mode, UiMode::Debug);
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("mode = \"debug\""));
    let (restored_debug, _) = controller(&path);
    assert_eq!(restored_debug.snapshot().ui_mode, UiMode::Debug);

    fs::write(&path, "version = 1\n").unwrap();
    let (defaulted, _) = controller(&path);
    assert_eq!(defaulted.snapshot().ui_mode, UiMode::Basic);

    let error = config_controller
        .set_ui_mode(UiMode::Basic, snapshot.revision.saturating_sub(1))
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Conflict);
    assert_eq!(config_controller.snapshot().ui_mode, UiMode::Debug);
    finish(directory);
}

#[test]
fn sounds_default_on_persist_and_follow_reload_without_accepting_stale_writes() {
    let directory = test_path();
    let path = directory.join("config.toml");
    // Existing configs that specify only UI mode retain sound playback.
    fs::create_dir_all(&directory).unwrap();
    fs::write(&path, "[ui]\nmode = \"advanced\"\n").unwrap();
    let (mut config, _) = controller(&path);
    assert!(config.sounds_enabled());
    let revision = config.snapshot().revision;
    let muted = config.set_sounds_enabled(false, revision).unwrap();
    assert!(!muted.sounds_enabled);
    let (restored, _) = controller(&path);
    assert!(!restored.sounds_enabled());
    assert_eq!(restored.snapshot().ui_mode, UiMode::Advanced);
    let error = config.set_sounds_enabled(true, revision).unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Conflict);
    assert!(!config.sounds_enabled());

    fs::write(&path, "[ui]\nsounds_enabled = true\n").unwrap();
    assert!(matches!(
        config.reload_from_disk(),
        ConfigTransition::Changed(_)
    ));
    assert!(config.sounds_enabled());
    fs::write(&path, "[ui]\nsounds_enabled = \"invalid\"\n").unwrap();
    config.reload_from_disk();
    assert!(config.sounds_enabled());
    finish(directory);
}

#[test]
fn overlay_position_persists_and_respects_config_revisions() {
    use super::super::types::OverlayPosition;

    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut config_controller, _) = controller(&path);
    assert_eq!(
        config_controller.snapshot().overlay_position,
        OverlayPosition::BottomCenter
    );

    let snapshot = config_controller
        .set_overlay_position(
            OverlayPosition::TopLeft,
            config_controller.snapshot().revision,
        )
        .unwrap();
    assert_eq!(snapshot.overlay_position, OverlayPosition::TopLeft);
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("overlay_position = \"top_left\""));
    let (restored, _) = controller(&path);
    assert_eq!(
        restored.snapshot().overlay_position,
        OverlayPosition::TopLeft
    );

    let error = config_controller
        .set_overlay_position(OverlayPosition::Center, snapshot.revision.saturating_sub(1))
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Conflict);
    assert_eq!(
        config_controller.snapshot().overlay_position,
        OverlayPosition::TopLeft
    );
    finish(directory);
}

#[test]
fn complete_processing_selection_is_a_frozen_capture_snapshot() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let created = controller
        .create_processing_profile_draft(
            ProcessingProfileDraft {
                name: "Visual".into(),
                model_settings: None,
                rules_model_settings: None,
                rules_model_mode: crate::profiles::RulesModelMode::Same,
                conversion: ConversionSettings {
                    tables: false,
                    colors: true,
                    images: false,
                },
                additional_rules: "  Keep {{ instructions }}\n  ".into(),
                additional_rules_separate_pass: false,
                application_mode_override: Some(ConversionSettingsApplicationMode::Prompt),
                conversion_prompt_template: Some("Keep {{ tables }}".into()),
                postprocessing_prompt_template: Some("Keep {{ intermediate_html }}".into()),
            },
            controller.snapshot().revision,
        )
        .unwrap();
    let profile_id = created
        .processing
        .profiles
        .iter()
        .find(|profile| profile.id != "default")
        .unwrap()
        .id
        .clone();
    let frozen = controller
        .resolve_capture_context(&profile_id, Some("fixed-binding".into()))
        .expect("profile should resolve");

    controller
        .update_processing_profile(
            profile_id.clone(),
            ProcessingProfileDraft {
                name: "Visual".into(),
                model_settings: None,
                rules_model_settings: None,
                rules_model_mode: crate::profiles::RulesModelMode::Same,
                conversion: ConversionSettings::default(),
                additional_rules: "Different instructions".into(),
                additional_rules_separate_pass: true,
                application_mode_override: Some(ConversionSettingsApplicationMode::HtmlCleanup),
                conversion_prompt_template: None,
                postprocessing_prompt_template: None,
            },
            created.revision,
        )
        .unwrap();
    assert_eq!(
        (frozen.conversion, frozen.application_mode),
        (
            ConversionSettings {
                tables: false,
                colors: true,
                images: false,
            },
            ConversionSettingsApplicationMode::Prompt,
        )
    );
    assert_eq!(frozen.additional_rules, "  Keep {{ instructions }}\n  ");
    assert!(!frozen.additional_rules_separate_pass);
    assert_eq!(
        frozen.conversion_prompt_template.as_deref(),
        Some("Keep {{ tables }}")
    );
    assert_eq!(
        frozen.postprocessing_prompt_template.as_deref(),
        Some("Keep {{ intermediate_html }}")
    );
    assert_eq!(frozen.profile_id, profile_id);
    assert_eq!(frozen.binding_id.as_deref(), Some("fixed-binding"));
    let current = controller
        .resolve_capture_context(&profile_id, None)
        .unwrap();
    assert_eq!(current.additional_rules, "Different instructions");
    assert!(current.additional_rules_separate_pass);
    finish(directory);
}

#[test]
fn binding_crud_and_profile_delete_are_atomic() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    controller.activate().unwrap();
    let revision = controller.snapshot().revision;
    let snapshot = controller
        .create_processing_profile("Second".to_string(), revision)
        .unwrap();
    let profile_id = snapshot
        .processing
        .profiles
        .iter()
        .find(|profile| profile.name == "Second")
        .unwrap()
        .id
        .clone();
    let snapshot = controller
        .create_binding(
            profile_id.clone(),
            "CommandOrControl+Alt+9".to_string(),
            snapshot.revision,
        )
        .unwrap();
    assert_eq!(snapshot.capture_bindings.len(), 2);
    let binding_id = snapshot
        .capture_bindings
        .iter()
        .find(|binding| binding.id != "default")
        .unwrap()
        .id
        .clone();
    let snapshot = controller
        .set_binding(
            binding_id.clone(),
            "CommandOrControl+Alt+8".to_string(),
            snapshot.revision,
        )
        .unwrap();
    assert!(snapshot
        .capture_bindings
        .iter()
        .any(|binding| binding.shortcut == "CommandOrControl+Alt+8"));
    let snapshot = controller
        .delete_processing_profile(profile_id, snapshot.revision)
        .unwrap();
    assert_eq!(snapshot.capture_bindings.len(), 1);
    assert_eq!(snapshot.capture_bindings[0].id, "default");
    finish(directory);
}

#[test]
fn fixed_binding_profile_retarget_keeps_shortcut_registration_and_active_profile() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let second = controller
        .create_processing_profile("Second".to_string(), controller.snapshot().revision)
        .unwrap();
    let second_id = second
        .processing
        .profiles
        .iter()
        .find(|profile| profile.id != "default")
        .unwrap()
        .id
        .clone();
    let binding = controller
        .create_binding(
            second_id,
            "CommandOrControl+Alt+9".to_string(),
            second.revision,
        )
        .unwrap();
    let binding_id = binding
        .capture_bindings
        .iter()
        .find(|binding| binding.id != "default")
        .unwrap()
        .id
        .clone();
    let third = controller
        .create_processing_profile("Third".to_string(), binding.revision)
        .unwrap();
    let third_id = third
        .processing
        .profiles
        .iter()
        .find(|profile| profile.name == "Third")
        .unwrap()
        .id
        .clone();
    let calls_before = runtime.lock().unwrap().calls.len();

    let retargeted = controller
        .set_binding_profile(binding_id.clone(), third_id.clone(), third.revision)
        .unwrap();

    assert_eq!(retargeted.processing.active_profile_id, "default");
    assert_eq!(
        retargeted
            .capture_bindings
            .iter()
            .find(|binding| binding.id == binding_id)
            .unwrap()
            .profile_id,
        Some(third_id.clone())
    );
    assert_eq!(runtime.lock().unwrap().calls.len(), calls_before);
    assert_eq!(
        controller
            .resolve_capture_shortcut_id(
                super::super::bindings::native_shortcut_id("CommandOrControl+Alt+9").unwrap()
            )
            .unwrap()
            .profile_id,
        third_id
    );
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("profile_id = \"profile-2\""));
    finish(directory);
}

#[test]
fn default_or_missing_profile_retarget_is_rejected_without_changes() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let original_file = fs::read_to_string(&path).unwrap();
    let calls_before = runtime.lock().unwrap().calls.clone();
    let revision = controller.snapshot().revision;

    let default_error = controller
        .set_binding_profile("default".to_string(), "default".to_string(), revision)
        .unwrap_err();
    assert_eq!(default_error.kind, ConfigErrorKind::Validation);
    assert_eq!(fs::read_to_string(&path).unwrap(), original_file);
    assert_eq!(runtime.lock().unwrap().calls, calls_before);
    assert_eq!(controller.snapshot().capture_bindings[0].profile_id, None);

    let profile = controller
        .create_processing_profile("Second".to_string(), controller.snapshot().revision)
        .unwrap();
    let profile_id = profile
        .processing
        .profiles
        .iter()
        .find(|profile| profile.id != "default")
        .unwrap()
        .id
        .clone();
    let binding = controller
        .create_binding(
            profile_id.clone(),
            "CommandOrControl+Alt+9".to_string(),
            profile.revision,
        )
        .unwrap();
    let binding_id = binding
        .capture_bindings
        .iter()
        .find(|binding| binding.id != "default")
        .unwrap()
        .id
        .clone();
    let original_file = fs::read_to_string(&path).unwrap();
    let calls_before = runtime.lock().unwrap().calls.clone();
    let missing_error = controller
        .set_binding_profile(binding_id.clone(), "missing".to_string(), binding.revision)
        .unwrap_err();
    assert_eq!(missing_error.kind, ConfigErrorKind::Validation);
    assert_eq!(fs::read_to_string(&path).unwrap(), original_file);
    assert_eq!(runtime.lock().unwrap().calls, calls_before);
    assert_eq!(
        controller
            .snapshot()
            .capture_bindings
            .iter()
            .find(|binding| binding.id == binding_id)
            .unwrap()
            .profile_id,
        Some(profile_id)
    );
    finish(directory);
}

#[test]
fn duplicate_native_aliases_are_rejected() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    controller.activate().unwrap();
    let revision = controller.snapshot().revision;
    let error = controller
        .create_binding(
            "default".to_string(),
            "CmdOrCtrl+Shift+2".to_string(),
            revision,
        )
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Validation);
    assert_eq!(controller.snapshot().capture_bindings.len(), 1);
    finish(directory);
}

#[test]
fn multi_registration_failure_rolls_back_successful_additions() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let revision = controller.snapshot().revision;
    runtime
        .lock()
        .unwrap()
        .blocked_register
        .insert("CommandOrControl+Alt+9".to_string());
    let error = controller
        .create_binding(
            "default".to_string(),
            "CommandOrControl+Alt+9".to_string(),
            revision,
        )
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Conflict);
    assert!(controller.snapshot().registered);
    assert_eq!(controller.snapshot().capture_bindings.len(), 1);
    assert_eq!(runtime.lock().unwrap().active.len(), 2);
    finish(directory);
}

#[test]
fn suspend_and_resume_register_all_bindings_transactionally() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let revision = controller.snapshot().revision;
    controller
        .create_binding(
            "default".to_string(),
            "CommandOrControl+Alt+9".to_string(),
            revision,
        )
        .unwrap();
    controller.suspend_capture_shortcut().unwrap();
    assert!(!controller.snapshot().registered);
    assert!(runtime.lock().unwrap().active.is_empty());
    controller.resume_capture_shortcut().unwrap();
    assert!(controller.snapshot().registered);
    assert_eq!(runtime.lock().unwrap().active.len(), 3);
    finish(directory);
}

fn config_toml(shortcut: &str) -> String {
    format!(
        "version = 1\n\n[capture]\n\n[[capture.bindings]]\nid = \"default\"\nshortcut = \"{shortcut}\"\n\n[processing]\nactive_profile = \"default\"\n\n[processing.profiles.default]\nname = \"Default\"\n\n[inference.providers]\n"
    )
}

#[test]
fn missing_file_gets_default_and_ui_transition_is_persisted() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    assert_eq!(
        controller.snapshot().capture_bindings[0].shortcut,
        DEFAULT_CAPTURE_SHORTCUT
    );
    controller.activate().unwrap();
    let snapshot = controller
        .set_binding(
            "default".to_string(),
            "CommandOrControl+Alt+9".to_string(),
            controller.snapshot().revision,
        )
        .unwrap();
    assert_eq!(
        snapshot.capture_bindings[0].shortcut,
        "CommandOrControl+Alt+9"
    );
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("CommandOrControl+Alt+9"));
    assert_eq!(runtime.lock().unwrap().active.len(), 2);
    finish(directory);
}

#[test]
fn empty_and_partial_configs_load_with_defaults_without_rewriting_the_file() {
    let cases = [
        "",
        "   \n# settings are intentionally empty\n",
        "version = 1\n",
        "version = 1\n\n[capture]\n\n[processing]\n\n[inference]\n",
    ];
    for raw in cases {
        let directory = test_path();
        let path = directory.join("config.toml");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, raw).unwrap();
        let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
        let (controller, startup_error) =
            ConfigController::load_or_default(&path, runtime).unwrap();
        assert!(startup_error.is_none(), "raw config should load: {raw:?}");
        assert_eq!(controller.snapshot().version, CONFIG_VERSION);
        assert_eq!(controller.snapshot().capture_bindings.len(), 1);
        assert_eq!(controller.snapshot().processing.profiles.len(), 1);
        assert!(controller.snapshot().inference.providers.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        finish(directory);
    }

    let directory = test_path();
    let path = directory.join("config.toml");
    let raw = "version = 1\n\n[processing.profiles.custom]\nname = \"Custom\"\n";
    fs::create_dir_all(&directory).unwrap();
    fs::write(&path, raw).unwrap();
    let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
    let (controller, startup_error) = ConfigController::load_or_default(&path, runtime).unwrap();
    assert!(startup_error.is_none());
    assert_eq!(controller.snapshot().processing.active_profile_id, "custom");
    assert_eq!(controller.snapshot().processing.profiles.len(), 1);
    assert_eq!(fs::read_to_string(&path).unwrap(), raw);
    finish(directory);
}

#[test]
fn live_reload_of_empty_file_resets_defaults_and_keeps_file_content() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let configured = controller
        .create_processing_profile("Configured".to_string(), controller.snapshot().revision)
        .unwrap();
    assert_eq!(configured.processing.profiles.len(), 2);
    let raw = "\n# reset to defaults\n";
    fs::write(&path, raw).unwrap();

    let transition = controller.reload_from_disk();
    let snapshot = match transition {
        ConfigTransition::Changed(snapshot) => snapshot,
        other => panic!("expected empty config reload to change state: {other:?}"),
    };
    assert!(snapshot.last_error.is_none());
    assert_eq!(snapshot.processing.profiles.len(), 1);
    assert!(snapshot.inference.providers.is_empty());
    assert!(snapshot.inference.providers.is_empty());
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains(DEFAULT_CAPTURE_SHORTCUT));
    assert_eq!(fs::read_to_string(&path).unwrap(), raw);
    finish(directory);
}

#[test]
fn failed_restore_keeps_recording_suspended_until_retry() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    controller.suspend_capture_shortcut().unwrap();
    runtime
        .lock()
        .unwrap()
        .blocked_register
        .insert(DEFAULT_CAPTURE_SHORTCUT.to_string());
    let error = controller.resume_capture_shortcut().unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Conflict);
    assert!(!controller.snapshot().registered);
    runtime
        .lock()
        .unwrap()
        .blocked_register
        .remove(DEFAULT_CAPTURE_SHORTCUT);
    assert!(controller.resume_capture_shortcut().unwrap().registered);
    finish(directory);
}

#[test]
fn saving_a_draft_while_suspended_registers_only_the_saved_value() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    controller.suspend_capture_shortcut().unwrap();
    let snapshot = controller
        .set_binding(
            "default".to_string(),
            "CommandOrControl+Alt+9".to_string(),
            controller.snapshot().revision,
        )
        .unwrap();
    assert_eq!(
        snapshot.capture_bindings[0].shortcut,
        "CommandOrControl+Alt+9"
    );
    assert!(runtime.lock().unwrap().active.is_empty());
    assert!(controller.resume_capture_shortcut().unwrap().registered);
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains("CommandOrControl+Alt+9"));
    finish(directory);
}

#[test]
fn restoring_shortcuts_during_recording_does_not_stale_config_revision() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let revision = controller.snapshot().revision;

    controller.suspend_capture_shortcut().unwrap();
    controller.resume_capture_shortcut().unwrap();

    assert_eq!(controller.snapshot().revision, revision);
    let updated = controller
        .set_binding(
            "default".to_string(),
            "CommandOrControl+Alt+9".to_string(),
            revision,
        )
        .unwrap();
    assert_eq!(
        updated.capture_bindings[0].shortcut,
        "CommandOrControl+Alt+9"
    );
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains("CommandOrControl+Alt+9"));
    finish(directory);
}

#[test]
fn external_edit_during_recording_waits_for_resume_before_registering() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    controller.suspend_capture_shortcut().unwrap();
    fs::write(&path, config_toml("CommandOrControl+Alt+9")).unwrap();
    let transition = controller.reload_from_disk();
    assert!(matches!(
        transition,
        ConfigTransition::Changed(snapshot)
            if snapshot.capture_bindings[0].shortcut == "CommandOrControl+Alt+9"
    ));
    assert!(runtime.lock().unwrap().active.is_empty());
    assert!(controller.resume_capture_shortcut().unwrap().registered);
    finish(directory);
}

#[test]
fn malformed_external_edit_keeps_last_good_runtime_and_ui_state() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    fs::write(&path, "[capture\nbindings = [").unwrap();
    assert!(matches!(
        controller.reload_from_disk(),
        ConfigTransition::Rejected(ConfigError {
            kind: ConfigErrorKind::Parse,
            ..
        })
    ));
    assert_eq!(
        controller.snapshot().capture_bindings[0].shortcut,
        DEFAULT_CAPTURE_SHORTCUT
    );
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains(DEFAULT_CAPTURE_SHORTCUT));
    fs::write(&path, config_toml(DEFAULT_CAPTURE_SHORTCUT)).unwrap();
    assert!(matches!(
        controller.reload_from_disk(),
        ConfigTransition::Changed(snapshot) if snapshot.last_error.is_none()
    ));
    finish(directory);
}

#[test]
fn unknown_external_fields_are_rejected_without_overwrite() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let invalid = format!(
        "future_setting = true\n{}",
        config_toml(DEFAULT_CAPTURE_SHORTCUT)
    );
    fs::write(&path, invalid.clone()).unwrap();
    assert!(matches!(
        controller.reload_from_disk(),
        ConfigTransition::Rejected(ConfigError {
            kind: ConfigErrorKind::Parse,
            ..
        })
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
    finish(directory);
}

#[test]
fn config_version_errors_keep_parse_and_validation_classes() {
    let cases = [
        ("[capture]\nbindings = []\n", ConfigErrorKind::Validation),
        ("version = \"one\"\n", ConfigErrorKind::Parse),
        ("version = -1\n", ConfigErrorKind::Parse),
        ("version = 2\n", ConfigErrorKind::Validation),
    ];
    for (raw, expected_kind) in cases {
        let directory = test_path();
        let path = directory.join("config.toml");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, raw).unwrap();
        let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
        let (_, startup_error) = ConfigController::load_or_default(&path, runtime).unwrap();
        assert_eq!(startup_error.unwrap().kind, expected_kind);
        finish(directory);
    }
}

#[test]
fn valid_external_transition_replaces_all_active_shortcuts() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    fs::write(&path, config_toml("CommandOrControl+Alt+9")).unwrap();
    assert!(matches!(
        controller.reload_from_disk(),
        ConfigTransition::Changed(snapshot)
            if snapshot.capture_bindings[0].shortcut == "CommandOrControl+Alt+9"
    ));
    assert!(!runtime
        .lock()
        .unwrap()
        .active
        .contains(DEFAULT_CAPTURE_SHORTCUT));
    assert!(runtime
        .lock()
        .unwrap()
        .active
        .contains("CommandOrControl+Alt+9"));
    finish(directory);
}

#[test]
fn startup_error_snapshot_keeps_default_registration_state() {
    let directory = test_path();
    let path = directory.join("config.toml");
    fs::create_dir_all(&directory).unwrap();
    fs::write(&path, config_toml("not-a-shortcut")).unwrap();
    let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
    let (mut controller, startup_error) =
        ConfigController::load_or_default(&path, runtime).unwrap();
    let startup_error = controller.remember_error(startup_error.unwrap());
    assert!(!startup_error.registered);
    controller.activate().unwrap();
    assert!(controller.snapshot().registered);
    assert_eq!(
        controller
            .snapshot()
            .last_error
            .as_ref()
            .unwrap()
            .current_bindings
            .len(),
        1
    );
    finish(directory);
}

#[test]
fn profile_crud_persists_and_reloads_with_stable_ids() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let created = controller
        .create_processing_profile("  Tables  ".to_string(), controller.snapshot().revision)
        .unwrap();
    let profile_id = created
        .processing
        .profiles
        .iter()
        .find(|profile| profile.id != "default")
        .unwrap()
        .id
        .clone();
    let renamed = controller
        .rename_processing_profile(
            profile_id.clone(),
            "Tables and colors".to_string(),
            created.revision,
        )
        .unwrap();
    let selected = controller
        .select_processing_profile(profile_id, renamed.revision)
        .unwrap();
    let (reloaded, startup_error) = ConfigController::load_or_default(
        &path,
        Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default())))),
    )
    .unwrap();
    assert!(startup_error.is_none());
    assert_eq!(reloaded.snapshot().processing, selected.processing);
    finish(directory);
}

#[test]
fn invalid_profile_names_references_and_stale_revisions_are_rejected() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let revision = controller.snapshot().revision;
    assert_eq!(
        controller
            .create_processing_profile("   ".to_string(), revision)
            .unwrap_err()
            .kind,
        ConfigErrorKind::Validation
    );
    let revision = controller.snapshot().revision;
    assert_eq!(
        controller
            .create_processing_profile("Default".to_string(), revision)
            .unwrap_err()
            .kind,
        ConfigErrorKind::Validation
    );
    let revision = controller.snapshot().revision;
    assert_eq!(
        controller
            .select_processing_profile("missing".to_string(), revision)
            .unwrap_err()
            .kind,
        ConfigErrorKind::Validation
    );
    assert_eq!(
        controller
            .create_processing_profile("Other".to_string(), revision)
            .unwrap_err()
            .kind,
        ConfigErrorKind::Conflict
    );
    finish(directory);
}

#[test]
fn deleting_active_profile_selects_first_remaining_and_last_is_protected() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let first = controller
        .create_processing_profile("First".to_string(), controller.snapshot().revision)
        .unwrap();
    let second = controller
        .create_processing_profile("Second".to_string(), first.revision)
        .unwrap();
    let selected = controller
        .select_processing_profile("profile-1".to_string(), second.revision)
        .unwrap();
    let deleted = controller
        .delete_processing_profile("profile-1".to_string(), selected.revision)
        .unwrap();
    assert_eq!(deleted.processing.active_profile_id, "default");
    let deleted = controller
        .delete_processing_profile("default".to_string(), deleted.revision)
        .unwrap();
    assert_eq!(deleted.processing.active_profile_id, "profile-2");
    assert_eq!(
        controller
            .delete_processing_profile("profile-2".to_string(), deleted.revision)
            .unwrap_err()
            .kind,
        ConfigErrorKind::Validation
    );
    finish(directory);
}

#[test]
fn failed_config_write_keeps_registered_shortcuts_and_config_state() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, runtime) = controller(&path);
    controller.activate().unwrap();
    let original_file = fs::read_to_string(&path).unwrap();
    let mut candidate = controller.current.clone();
    candidate.processing.profiles.insert(
        "profile-1".to_string(),
        ProcessingProfile {
            name: "Tables".to_string(),
            model_settings: None,
            rules_model_settings: None,
            rules_model_mode: crate::profiles::RulesModelMode::Same,
            conversion: Default::default(),
            additional_rules: String::new(),
            additional_rules_separate_pass: false,
            application_mode_override: None,
            conversion_prompt_template: None,
            postprocessing_prompt_template: None,
        },
    );
    let blocked_parent = directory.join("write-blocker");
    fs::create_dir_all(&directory).unwrap();
    fs::write(&blocked_parent, "a file, not a directory").unwrap();
    controller.path = blocked_parent.join("config.toml");
    let error = controller.apply_candidate(candidate, true).unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Io);
    assert_eq!(controller.current.processing.profiles.len(), 1);
    assert!(controller.is_fully_registered());
    assert_eq!(runtime.lock().unwrap().active.len(), 2);
    assert_eq!(fs::read_to_string(path).unwrap(), original_file);
    finish(directory);
}

#[test]
fn processing_profile_draft_round_trips_defaults_and_full_snapshot() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let snapshot = controller
        .create_processing_profile_draft(
            ProcessingProfileDraft {
                name: "  Rich  ".into(),
                model_settings: None,
                rules_model_settings: None,
                rules_model_mode: crate::profiles::RulesModelMode::Same,
                conversion: ConversionSettings {
                    tables: false,
                    colors: true,
                    images: false,
                },
                additional_rules: "  \n".into(),
                additional_rules_separate_pass: true,
                application_mode_override: Some(ConversionSettingsApplicationMode::HtmlCleanup),
                conversion_prompt_template: None,
                postprocessing_prompt_template: None,
            },
            controller.snapshot().revision,
        )
        .unwrap();
    let profile = snapshot
        .processing
        .profiles
        .iter()
        .find(|profile| profile.id != "default")
        .unwrap();
    assert_eq!(profile.name, "Rich");
    assert_eq!(profile.additional_rules, "");
    assert!(profile.additional_rules_separate_pass);
    assert!(!profile.conversion.tables);
    assert_eq!(
        profile.application_mode_override,
        Some(ConversionSettingsApplicationMode::HtmlCleanup)
    );
    let wire = serde_json::to_value(profile).unwrap();
    assert_eq!(
        wire["conversion"],
        serde_json::json!({
            "tables": false,
            "colors": true,
            "images": false,
        })
    );
    assert!(wire.get("additionalRulesSeparatePass").is_some());

    let (reloaded, startup_error) = ConfigController::load_or_default(
        &path,
        Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default())))),
    )
    .unwrap();
    assert!(startup_error.is_none());
    assert_eq!(reloaded.snapshot().processing, snapshot.processing);
    finish(directory);
}

#[test]
fn processing_profile_partial_conversion_defaults_and_invalid_fields_are_strict() {
    let cases = [
        "[processing.profiles.default.conversion]\ntables = false\n",
        "[processing.profiles.default.conversion]\nunknown = true\n",
        "[processing.profiles.default]\napplication_mode_override = \"later\"\n",
        "[processing.profiles.default]\nadditional_rules = 42\n",
    ];
    for (index, suffix) in cases.iter().enumerate() {
        let directory = test_path();
        let path = directory.join("config.toml");
        let raw =
            format!("version = 1\n\n[processing.profiles.default]\nname = \"Default\"\n\n{suffix}");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, &raw).unwrap();
        let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
        let (controller, startup_error) =
            ConfigController::load_or_default(&path, runtime).unwrap();
        if index == 0 {
            assert!(startup_error.is_none());
            let conversion = &controller.snapshot().processing.profiles[0].conversion;
            assert!(!conversion.tables);
            assert!(conversion.colors);
            assert!(conversion.images);
        } else {
            assert_eq!(startup_error.unwrap().kind, ConfigErrorKind::Parse);
            assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        }
        finish(directory);
    }
}

#[test]
fn additional_rules_bound_and_control_validation_are_transactional() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let original = fs::read_to_string(&path).unwrap();
    let oversized = ProcessingProfileDraft {
        name: "Default".into(),
        model_settings: None,
        rules_model_settings: None,
        rules_model_mode: crate::profiles::RulesModelMode::Same,
        conversion: Default::default(),
        additional_rules: "x".repeat(APPLICATION.limits.additional_rules_bytes + 1),
        additional_rules_separate_pass: false,
        application_mode_override: None,
        conversion_prompt_template: None,
        postprocessing_prompt_template: None,
    };
    let error = controller
        .update_processing_profile("default".into(), oversized, controller.snapshot().revision)
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Validation);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    let control = ProcessingProfileDraft {
        name: "Default".into(),
        model_settings: None,
        rules_model_settings: None,
        rules_model_mode: crate::profiles::RulesModelMode::Same,
        conversion: Default::default(),
        additional_rules: "bad\u{0000}rule".into(),
        additional_rules_separate_pass: false,
        application_mode_override: None,
        conversion_prompt_template: None,
        postprocessing_prompt_template: None,
    };
    let error = controller
        .update_processing_profile("default".into(), control, error.revision)
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Validation);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    finish(directory);
}

#[test]
fn prompt_template_overrides_validate_transactionally_and_canonicalize_defaults() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let revision = controller.snapshot().revision;
    let default_template = crate::inference::params::default_conversion_prompt_template();
    let snapshot = controller
        .update_processing_profile(
            "default".into(),
            ProcessingProfileDraft {
                name: "Default".into(),
                model_settings: None,
                rules_model_settings: None,
                rules_model_mode: crate::profiles::RulesModelMode::Same,
                conversion: Default::default(),
                additional_rules: String::new(),
                additional_rules_separate_pass: false,
                application_mode_override: None,
                conversion_prompt_template: Some(default_template.to_owned()),
                postprocessing_prompt_template: Some(
                    crate::inference::params::default_postprocessing_prompt_template().to_owned(),
                ),
            },
            revision,
        )
        .unwrap();
    let profile = &snapshot.processing.profiles[0];
    assert!(profile.conversion_prompt_template.is_none());
    assert!(profile.postprocessing_prompt_template.is_none());

    let original = fs::read_to_string(&path).unwrap();
    let error = controller
        .update_processing_profile(
            "default".into(),
            ProcessingProfileDraft {
                name: "Default".into(),
                model_settings: None,
                rules_model_settings: None,
                rules_model_mode: crate::profiles::RulesModelMode::Same,
                conversion: Default::default(),
                additional_rules: String::new(),
                additional_rules_separate_pass: false,
                application_mode_override: None,
                conversion_prompt_template: Some("{% if".into()),
                postprocessing_prompt_template: None,
            },
            snapshot.revision,
        )
        .unwrap_err();
    assert_eq!(error.kind, ConfigErrorKind::Validation);
    assert!(error.message.contains("conversion prompt template"));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    finish(directory);
}

#[test]
fn prompt_template_defaults_are_normalized_and_external_invalid_sources_rejected() {
    let directory = test_path();
    let path = directory.join("config.toml");
    let raw = format!(
        "version = 1\n\n[processing.profiles.default]\nname = \"Default\"\nconversion_prompt_template = {:?}\n",
        crate::inference::params::default_conversion_prompt_template()
    );
    fs::create_dir_all(&directory).unwrap();
    fs::write(&path, &raw).unwrap();
    let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
    let (controller, startup_error) = ConfigController::load_or_default(&path, runtime).unwrap();
    assert!(startup_error.is_none());
    assert!(controller.snapshot().processing.profiles[0]
        .conversion_prompt_template
        .is_none());
    finish(directory);

    for invalid in ["", "{% if"] {
        let directory = test_path();
        let path = directory.join("config.toml");
        let raw = format!(
            "version = 1\n\n[processing.profiles.default]\nname = \"Default\"\nconversion_prompt_template = {invalid:?}\n"
        );
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, &raw).unwrap();
        let runtime = Box::new(FakeRuntime(Arc::new(Mutex::new(RuntimeState::default()))));
        let (_, startup_error) = ConfigController::load_or_default(&path, runtime).unwrap();
        assert_eq!(startup_error.unwrap().kind, ConfigErrorKind::Validation);
        assert_eq!(fs::read_to_string(&path).unwrap(), raw);
        finish(directory);
    }
}

fn model_settings(model: &str) -> crate::inference_profiles::ModelSettings {
    crate::inference_profiles::ModelSettings {
        provider: crate::inference_profiles::InferenceBackend::OpenaiApi,
        model: model.into(),
        options: Some(crate::inference_profiles::InferenceOptions {
            request_timeout_seconds: Some(45),
            ..Default::default()
        }),
    }
}

#[test]
fn owned_models_round_trip_remembered_settings_and_freeze_independent_stages() {
    use crate::profiles::RulesModelMode;
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let mut candidate = controller.current.clone();
    let profile = candidate.processing.profiles.get_mut("default").unwrap();
    profile.model_settings = Some(model_settings("primary"));
    profile.rules_model_settings = Some(model_settings("rules"));
    profile.additional_rules = "Translate".into();
    profile.additional_rules_separate_pass = true;
    controller.apply_candidate(candidate, true).unwrap();
    let frozen = controller.resolve_capture_context("default", None).unwrap();
    assert_eq!(frozen.inference, frozen.postprocessing_inference);
    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("rules_model_settings"));
    assert!(raw.contains("rules_model_mode = \"same\""));
    let mut restored = parse_config(&path, &raw).unwrap().config;
    let copy = restored.processing.profiles["default"].clone();
    restored.processing.profiles.insert(
        "copy".into(),
        ProcessingProfile {
            name: "Copy".into(),
            ..copy
        },
    );
    let profile = restored.processing.profiles.get_mut("default").unwrap();
    profile.rules_model_mode = RulesModelMode::Independent;
    profile.model_settings.as_mut().unwrap().model = "changed".into();
    controller.apply_candidate(restored, true).unwrap();
    let current = controller.resolve_capture_context("default", None).unwrap();
    assert_eq!(current.inference.unwrap().model, "changed");
    assert_eq!(current.postprocessing_inference.unwrap().model, "rules");
    assert_eq!(frozen.inference.unwrap().model, "primary");
    assert_eq!(
        controller
            .resolve_inference_for_processing("copy")
            .unwrap()
            .model,
        "primary"
    );
    let mut missing = controller.current.clone();
    missing
        .processing
        .profiles
        .get_mut("default")
        .unwrap()
        .rules_model_settings = None;
    controller.apply_candidate(missing, true).unwrap();
    assert!(controller
        .resolve_capture_context("default", None)
        .unwrap()
        .postprocessing_inference
        .is_none());
    let before = fs::read_to_string(&path).unwrap();
    let mut invalid = controller.current.clone();
    invalid
        .processing
        .profiles
        .get_mut("default")
        .unwrap()
        .model_settings
        .as_mut()
        .unwrap()
        .options
        .as_mut()
        .unwrap()
        .temperature = Some(3.0);
    assert!(controller.apply_candidate(invalid, true).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    finish(directory);
}

#[test]
fn provider_profiles_are_persisted_once_and_respect_edits_and_deletion() {
    use crate::inference_profiles::{InferenceBackend, ProviderConfig};
    let directory = test_path();
    let path = directory.join("config.toml");
    let (mut controller, _) = controller(&path);
    let mut config = controller.current.clone();
    config.inference.providers.insert(
        "openai_api".into(),
        ProviderConfig {
            endpoint: None,
            credential_ref: Some("0123456789abcdef0123456789abcdef".into()),
        },
    );
    controller.apply_candidate(config, true).unwrap();
    assert_eq!(controller.current.processing.profiles.len(), 3);
    let mut edited = controller.current.clone();
    let ids: Vec<_> = edited
        .processing
        .profiles
        .keys()
        .filter(|id| *id != "default")
        .cloned()
        .collect();
    edited.processing.profiles.remove(&ids[0]);
    edited
        .processing
        .profiles
        .get_mut(&ids[1])
        .unwrap()
        .model_settings = Some(model_settings("custom"));
    controller.apply_candidate(edited, true).unwrap();
    let loaded = parse_config(&path, &fs::read_to_string(&path).unwrap()).unwrap();
    let mut reloaded = loaded.config;
    assert!(!reloaded.provision_profiles());
    assert_eq!(reloaded.processing.profiles.len(), 2);
    assert_eq!(
        reloaded.processing.profiles[&ids[1]]
            .model_settings
            .as_ref()
            .unwrap()
            .model,
        "custom"
    );
    assert_eq!(
        reloaded.inference.provisioned_providers,
        vec![InferenceBackend::OpenaiApi]
    );
    finish(directory);
}

#[test]
fn ocr_rules_automatically_require_an_independent_text_model() {
    let directory = test_path();
    let (mut controller, _) = controller(&directory.join("config.toml"));
    let mut config = controller.current.clone();
    let profile = config.processing.profiles.get_mut("default").unwrap();
    let mut model = model_settings("isocr1-0.8b");
    model.provider = crate::inference_profiles::InferenceBackend::Local;
    profile.model_settings = Some(model);
    profile.additional_rules = "Translate".into();
    controller.apply_candidate(config, true).unwrap();
    let context = controller.resolve_capture_context("default", None).unwrap();
    assert!(context.additional_rules_separate_pass);
    assert!(context.postprocessing_inference.is_none());
    let path = directory.join("config.toml");
    let external = fs::read_to_string(&path)
        .unwrap()
        .replace(
            "rules_model_mode = \"independent\"",
            "rules_model_mode = \"same\"",
        )
        .replace(
            "additional_rules_separate_pass = true",
            "additional_rules_separate_pass = false",
        );
    fs::write(&path, external).unwrap();
    assert!(matches!(
        controller.reload_from_disk(),
        ConfigTransition::Changed(_)
    ));
    let persisted = parse_config(&path, &fs::read_to_string(&path).unwrap())
        .unwrap()
        .config;
    assert_eq!(persisted, controller.current);
    assert!(matches!(
        controller.reload_from_disk(),
        ConfigTransition::Unchanged(_)
    ));
    finish(directory);
}
