use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[allow(dead_code)]
#[path = "src/changelog/parse.rs"]
mod changelog;

fn main() {
    validate_changelog();
    stage_legal();
    tauri_build::build();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var_os("CARGO_FEATURE_LOCAL_INFERENCE").is_some()
    {
        link_clang_runtime();
    }
}

/// The app renders CHANGELOG.md in Settings and the release publishes it in
/// the update manifest, so a malformed changelog fails the build.
fn validate_changelog() {
    println!("cargo:rerun-if-changed=../CHANGELOG.md");
    let source = fs::read_to_string("../CHANGELOG.md").expect("could not read CHANGELOG.md");
    if let Err(error) = changelog::parse(&source) {
        panic!("CHANGELOG.md: {error}");
    }
}

fn link_clang_runtime() {
    println!("cargo:rerun-if-env-changed=DEVELOPER_DIR");

    // llama.cpp's Metal availability checks need compiler-rt, which Rust's
    // -nodefaultlibs link does not include automatically.
    let output = std::process::Command::new("xcrun")
        .args(["clang", "--print-file-name=libclang_rt.osx.a"])
        .output()
        .expect("failed to locate the Xcode Clang runtime with xcrun");
    assert!(
        output.status.success(),
        "xcrun could not locate the Xcode Clang runtime: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let path = String::from_utf8(output.stdout).expect("Clang runtime path is not UTF-8");
    let path = std::path::Path::new(path.trim());
    assert!(
        path.is_file(),
        "Clang runtime not found: {}",
        path.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        path.parent()
            .expect("Clang runtime has no parent")
            .display()
    );
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}

// Stages the legal folder that the app bundles. The English source license is
// kept only at the repository root as LICENSE.md, so it is copied in as
// SOURCE-LICENSE.md and links to it are rewritten for the bundle layout.
fn stage_legal() {
    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets the manifest dir"));
    let source = manifest.join("legal");
    let license = manifest.join("../LICENSE.md");
    let notices = manifest.join("../THIRD_PARTY_NOTICES.txt");
    for path in [&source, &license, &notices] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let staged = manifest.join("gen/legal");
    if staged.exists() {
        fs::remove_dir_all(&staged).expect("failed to clear the staged legal folder");
    }
    stage_directory(&source, &staged, 0);
    let license = fs::read_to_string(&license).expect("failed to read LICENSE.md");
    write(
        &staged.join("SOURCE-LICENSE.md"),
        &license.replace("](src-tauri/legal/", "]("),
    );
    fs::copy(&notices, staged.join("THIRD_PARTY_NOTICES.txt"))
        .expect("failed to copy THIRD_PARTY_NOTICES.txt");
    check_links(&staged);
}

fn stage_directory(from: &Path, to: &Path, depth: usize) {
    fs::create_dir_all(to).expect("failed to create the staged legal folder");
    for entry in fs::read_dir(from).expect("failed to read the legal folder") {
        let path = entry.expect("failed to read a legal folder entry").path();
        let target = to.join(path.file_name().expect("legal entries have names"));
        if path.is_dir() {
            stage_directory(&path, &target, depth + 1);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            // From legal/ the root license is ../../LICENSE.md; from legal/es/ it
            // is ../../../LICENSE.md. In the bundle it sits at legal/SOURCE-LICENSE.md.
            let text = fs::read_to_string(&path).expect("failed to read a legal document");
            let from = format!("]({}LICENSE.md", "../".repeat(depth + 2));
            let to = format!("]({}SOURCE-LICENSE.md", "../".repeat(depth));
            write(&target, &text.replace(&from, &to));
        } else {
            fs::copy(&path, &target).expect("failed to copy a legal file");
        }
    }
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).expect("failed to write a staged legal document");
}

// Fails the build when a bundled document links to a file the bundle lacks.
fn check_links(directory: &Path) {
    for entry in fs::read_dir(directory).expect("failed to read the staged legal folder") {
        let path = entry.expect("failed to read a staged legal entry").path();
        if path.is_dir() {
            check_links(&path);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("failed to read a staged legal document");
        for link in text.split("](").skip(1) {
            let Some(end) = link.find(')') else { continue };
            let target = link[..end].split('#').next().unwrap_or_default();
            if target.is_empty() || target.contains(':') || target.starts_with('/') {
                continue;
            }
            let parent = path.parent().expect("staged documents have a folder");
            assert!(
                parent.join(target).exists(),
                "{} links to {target}, which is not in the bundled legal folder",
                path.display()
            );
        }
    }
}
