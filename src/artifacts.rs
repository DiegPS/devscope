use std::path::Path;

use crate::project::{ArtifactKind, ProjectArtifact};
use crate::snapshot::DirSnapshot;

#[allow(dead_code)]
pub fn detect_artifacts(project_path: &Path, stack: &[String]) -> Vec<ProjectArtifact> {
    detect_artifacts_with_snapshot(&DirSnapshot::read(project_path), stack)
}

pub(crate) fn detect_artifacts_with_snapshot(
    snapshot: &DirSnapshot,
    stack: &[String],
) -> Vec<ProjectArtifact> {
    let project_path = snapshot.root();
    let mut artifacts = Vec::new();

    let is_flutter = stack.iter().any(|s| s.contains("Flutter"));
    let is_rust = stack_contains(stack, "Rust");
    let is_tauri = stack_contains(stack, "Tauri");
    let is_node = stack_contains(stack, "Node");

    if is_flutter {
        detect_flutter_artifacts(project_path, &mut artifacts);
    }
    if is_rust {
        detect_rust_artifacts(snapshot, &mut artifacts);
    }
    if is_tauri {
        detect_tauri_artifacts(project_path, &mut artifacts);
    }
    if is_node {
        detect_node_artifacts(project_path, &mut artifacts);
    }

    artifacts
}

fn detect_flutter_artifacts(project_path: &Path, artifacts: &mut Vec<ProjectArtifact>) {
    // Windows
    let win_dir = project_path.join("build/windows/x64/runner/Release");
    if win_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&win_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "exe") {
                    let name = p
                        .file_stem()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    artifacts.push(ProjectArtifact {
                        label: format!("Windows exe ({})", name),
                        path: p,
                        kind: ArtifactKind::Executable,
                        exists: true,
                    });
                }
            }
        }
        artifacts.push(ProjectArtifact {
            label: "Release dir".to_string(),
            path: win_dir,
            kind: ArtifactKind::Folder,
            exists: true,
        });
    } else {
        artifacts.push(ProjectArtifact::new(
            "Windows exe",
            project_path.join("build/windows/x64/runner/Release"),
            ArtifactKind::Folder,
        ));
    }

    // Android APK
    let apk_debug = project_path.join("build/app/outputs/flutter-apk/app-debug.apk");
    let apk_release = project_path.join("build/app/outputs/flutter-apk/app-release.apk");
    if apk_release.exists() {
        artifacts.push(ProjectArtifact::new(
            "Android release APK",
            apk_release,
            ArtifactKind::Apk,
        ));
    } else if apk_debug.exists() {
        artifacts.push(ProjectArtifact::new(
            "Android debug APK",
            apk_debug,
            ArtifactKind::Apk,
        ));
    } else {
        artifacts.push(ProjectArtifact::new(
            "Android APK",
            project_path.join("build/app/outputs/flutter-apk"),
            ArtifactKind::Folder,
        ));
    }

    // Web
    artifacts.push(ProjectArtifact::new(
        "Web build",
        project_path.join("build/web"),
        ArtifactKind::Web,
    ));

    // Linux
    #[cfg(target_os = "linux")]
    {
        artifacts.push(ProjectArtifact::new(
            "Linux bundle",
            project_path.join("build/linux/x64/release/bundle"),
            ArtifactKind::Bundle,
        ));
    }
    #[cfg(not(target_os = "linux"))]
    {
        artifacts.push(ProjectArtifact::new(
            "Linux bundle",
            project_path.join("build/linux/x64/release/bundle"),
            ArtifactKind::Bundle,
        ));
    }

    // macOS
    #[cfg(target_os = "macos")]
    {
        artifacts.push(ProjectArtifact::new(
            "macOS app",
            project_path.join("build/macos/Build/Products/Release"),
            ArtifactKind::Bundle,
        ));
    }
}

fn detect_rust_artifacts(snapshot: &DirSnapshot, artifacts: &mut Vec<ProjectArtifact>) {
    let Some(manifest) = snapshot
        .read_to_string("Cargo.toml")
        .and_then(|s| s.parse::<toml::Value>().ok())
    else {
        return;
    };
    let path = snapshot.root();
    let mut workspace = path.to_path_buf();
    if let Some(explicit) = manifest
        .get("package")
        .and_then(|p| p.get("workspace"))
        .and_then(toml::Value::as_str)
    {
        workspace = path.join(explicit);
    } else if !manifest.get("workspace").is_some_and(toml::Value::is_table) {
        for ancestor in path.ancestors().skip(1) {
            if std::fs::read_to_string(ancestor.join("Cargo.toml"))
                .ok()
                .and_then(|s| s.parse::<toml::Value>().ok())
                .is_some_and(|m| m.get("workspace").is_some_and(toml::Value::is_table))
            {
                workspace = ancestor.to_path_buf();
                break;
            }
        }
    }
    let configured_target = path.ancestors().find_map(|root| {
        [".cargo/config.toml", ".cargo/config"]
            .iter()
            .find_map(|name| {
                let config = std::fs::read_to_string(root.join(name))
                    .ok()?
                    .parse::<toml::Value>()
                    .ok()?;
                let target = config.get("build")?.get("target-dir")?.as_str()?;
                Some(root.join(target))
            })
    });
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .map(|target| {
            if target.is_absolute() {
                target
            } else {
                path.join(target)
            }
        })
        .or(configured_target)
        .unwrap_or_else(|| workspace.join("target"));
    let mut names = std::collections::BTreeSet::new();
    if let Some(bins) = manifest.get("bin").and_then(toml::Value::as_array) {
        for bin in bins {
            if let Some(name) = bin.get("name").and_then(toml::Value::as_str) {
                names.insert(name.to_owned());
            }
        }
    }
    if let Some(package) = manifest.get("package") {
        if package.get("autobins").and_then(toml::Value::as_bool) != Some(false) {
            if path.join("src/main.rs").is_file() {
                if let Some(name) = package.get("name").and_then(toml::Value::as_str) {
                    names.insert(name.to_owned());
                }
            }
            if let Ok(bins) = std::fs::read_dir(path.join("src/bin")) {
                for bin in bins.flatten() {
                    let path = bin.path();
                    if path.extension().is_some_and(|ext| ext == "rs")
                        || path.join("main.rs").is_file()
                    {
                        if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                            names.insert(name.to_owned());
                        }
                    }
                }
            }
        }
    }
    for name in names {
        // A manifest is data, never permission to escape the target directory.
        if name.contains(['/', '\\']) || name == "." || name == ".." {
            continue;
        }
        for profile in ["debug", "release"] {
            let filename = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                name.clone()
            };
            artifacts.push(ProjectArtifact::new(
                &format!("{profile}: {name}"),
                target.join(profile).join(filename),
                ArtifactKind::Executable,
            ));
        }
    }
    artifacts.push(ProjectArtifact::new(
        "Cargo target",
        target,
        ArtifactKind::Folder,
    ));
}

fn detect_tauri_artifacts(project_path: &Path, artifacts: &mut Vec<ProjectArtifact>) {
    let bundle = project_path.join("src-tauri/target/release/bundle");
    artifacts.push(ProjectArtifact::new(
        "Tauri bundle",
        bundle,
        ArtifactKind::Bundle,
    ));

    let release_dir = project_path.join("src-tauri/target/release");
    artifacts.push(ProjectArtifact::new(
        "Tauri release",
        release_dir,
        ArtifactKind::Folder,
    ));
}

fn detect_node_artifacts(project_path: &Path, artifacts: &mut Vec<ProjectArtifact>) {
    artifacts.push(ProjectArtifact::new(
        "dist/",
        project_path.join("dist"),
        ArtifactKind::Folder,
    ));

    artifacts.push(ProjectArtifact::new(
        "build/",
        project_path.join("build"),
        ArtifactKind::Folder,
    ));

    artifacts.push(ProjectArtifact::new(
        "out/",
        project_path.join("out"),
        ArtifactKind::Folder,
    ));
}

fn stack_contains(stack: &[String], needle: &str) -> bool {
    stack.iter().any(|entry| entry == needle)
}
