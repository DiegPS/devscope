use std::path::Path;

use crate::project::{ArtifactKind, ProjectArtifact};

pub fn detect_artifacts(project_path: &Path, stack: &[String]) -> Vec<ProjectArtifact> {
    let mut artifacts = Vec::new();

    let is_flutter = stack.iter().any(|s| s.contains("Flutter"));
    let is_rust = stack.contains(&"Rust".to_string());
    let is_tauri = stack.contains(&"Tauri".to_string());
    let is_node = stack.contains(&"Node".to_string());

    if is_flutter {
        detect_flutter_artifacts(project_path, &mut artifacts);
    }
    if is_rust {
        detect_rust_artifacts(project_path, &mut artifacts);
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

fn detect_rust_artifacts(project_path: &Path, artifacts: &mut Vec<ProjectArtifact>) {
    let project_name = project_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    #[cfg(target_os = "windows")]
    {
        let debug_exe = project_path.join(format!("target/debug/{}.exe", project_name));
        artifacts.push(ProjectArtifact::new(
            "Debug exe",
            debug_exe,
            ArtifactKind::Executable,
        ));

        let release_exe = project_path.join(format!("target/release/{}.exe", project_name));
        artifacts.push(ProjectArtifact::new(
            "Release exe",
            release_exe,
            ArtifactKind::Executable,
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        let debug_bin = project_path.join(format!("target/debug/{}", project_name));
        artifacts.push(ProjectArtifact::new(
            "Debug binary",
            debug_bin,
            ArtifactKind::Executable,
        ));

        let release_bin = project_path.join(format!("target/release/{}", project_name));
        artifacts.push(ProjectArtifact::new(
            "Release binary",
            release_bin,
            ArtifactKind::Executable,
        ));
    }
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
