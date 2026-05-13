use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectCommandKind {
    Start,
    Build,
    Test,
    Install,
    Dev,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectCommand {
    pub label: String,
    pub command: String,
    pub kind: ProjectCommandKind,
}

/// Detect suggested commands for a project based on its tech stack.
/// Reads config files (package.json, Cargo.toml, etc.) but never executes anything.
pub fn detect_commands(project_path: &Path, detected_stack: &[String]) -> Vec<ProjectCommand> {
    let mut commands = Vec::new();

    detect_node_commands(project_path, &mut commands);
    detect_flutter_commands(project_path, &mut commands);
    detect_rust_commands(project_path, &mut commands);
    detect_go_commands(project_path, &mut commands);
    detect_python_commands(project_path, detected_stack, &mut commands);
    detect_docker_commands(project_path, &mut commands);
    detect_dotnet_commands(project_path, &mut commands);
    detect_java_commands(project_path, &mut commands);

    // Limit to 6 commands
    commands.truncate(6);
    commands
}

// ── Node ────────────────────────────────────────────────────────────────

fn detect_node_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    let pkg_path = path.join("package.json");
    if !pkg_path.exists() {
        return;
    }

    let scripts = match parse_package_json_scripts(&pkg_path) {
        Some(s) => s,
        None => return,
    };

    let pm = detect_node_pm(path);

    for (script_name, run_cmd) in &scripts {
        let (label, kind) = match script_name.as_str() {
            "dev" => ("dev", ProjectCommandKind::Dev),
            "start" => ("start", ProjectCommandKind::Start),
            "build" => ("build", ProjectCommandKind::Build),
            "test" => ("test", ProjectCommandKind::Test),
            _ => continue,
        };
        out.push(ProjectCommand {
            label: label.to_string(),
            command: run_cmd.clone(),
            kind,
        });
    }

    // Install command
    let install_cmd = pm_install_cmd(&pm);
    out.push(ProjectCommand {
        label: "install".to_string(),
        command: install_cmd,
        kind: ProjectCommandKind::Install,
    });
}

fn detect_node_pm(path: &Path) -> String {
    if path.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if path.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if path.join("bun.lockb").exists() || path.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

fn parse_package_json_scripts(path: &Path) -> Option<Vec<(String, String)>> {
    let content = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let scripts_obj = json.get("scripts")?.as_object()?;

    let pm = detect_node_pm(path.parent()?);
    let mut result = Vec::new();

    for key in ["dev", "start", "build", "test"] {
        if let Some(_val) = scripts_obj.get(key) {
            let cmd = match pm.as_str() {
                "pnpm" => format!("pnpm {}", key),
                "yarn" => format!("yarn {}", key),
                "bun" => format!("bun run {}", key),
                _ => {
                    if key == "start" || key == "test" {
                        format!("npm {}", key)
                    } else {
                        format!("npm run {}", key)
                    }
                }
            };
            result.push((key.to_string(), cmd));
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn pm_install_cmd(pm: &str) -> String {
    match pm {
        "pnpm" => "pnpm install".to_string(),
        "yarn" => "yarn install".to_string(),
        "bun" => "bun install".to_string(),
        _ => "npm install".to_string(),
    }
}

// ── Flutter ─────────────────────────────────────────────────────────────

fn detect_flutter_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    if !path.join("pubspec.yaml").exists() {
        return;
    }

    out.push(ProjectCommand {
        label: "pub get".to_string(),
        command: "flutter pub get".to_string(),
        kind: ProjectCommandKind::Install,
    });

    let has_windows = path.join("windows").exists();
    let has_web = path.join("web").exists();
    let has_android = path.join("android").exists();

    if has_windows {
        out.push(ProjectCommand {
            label: "run (win)".to_string(),
            command: "flutter run -d windows".to_string(),
            kind: ProjectCommandKind::Start,
        });
        out.push(ProjectCommand {
            label: "build win".to_string(),
            command: "flutter build windows".to_string(),
            kind: ProjectCommandKind::Build,
        });
    }

    if has_web {
        out.push(ProjectCommand {
            label: "run (web)".to_string(),
            command: "flutter run -d chrome".to_string(),
            kind: ProjectCommandKind::Start,
        });
        out.push(ProjectCommand {
            label: "build web".to_string(),
            command: "flutter build web".to_string(),
            kind: ProjectCommandKind::Build,
        });
    }

    if has_android {
        out.push(ProjectCommand {
            label: "build apk".to_string(),
            command: "flutter build apk".to_string(),
            kind: ProjectCommandKind::Build,
        });
    }

    // Default flutter run
    if !has_windows && !has_web {
        out.push(ProjectCommand {
            label: "run".to_string(),
            command: "flutter run".to_string(),
            kind: ProjectCommandKind::Start,
        });
    }
}

// ── Rust ────────────────────────────────────────────────────────────────

fn detect_rust_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    if !path.join("Cargo.toml").exists() {
        return;
    }

    out.push(ProjectCommand {
        label: "run".to_string(),
        command: "cargo run".to_string(),
        kind: ProjectCommandKind::Start,
    });
    out.push(ProjectCommand {
        label: "build".to_string(),
        command: "cargo build".to_string(),
        kind: ProjectCommandKind::Build,
    });
    out.push(ProjectCommand {
        label: "test".to_string(),
        command: "cargo test".to_string(),
        kind: ProjectCommandKind::Test,
    });
    out.push(ProjectCommand {
        label: "release".to_string(),
        command: "cargo build --release".to_string(),
        kind: ProjectCommandKind::Build,
    });
}

// ── Go ──────────────────────────────────────────────────────────────────

fn detect_go_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    if !path.join("go.mod").exists() {
        return;
    }

    out.push(ProjectCommand {
        label: "run".to_string(),
        command: "go run .".to_string(),
        kind: ProjectCommandKind::Start,
    });
    out.push(ProjectCommand {
        label: "build".to_string(),
        command: "go build".to_string(),
        kind: ProjectCommandKind::Build,
    });
    out.push(ProjectCommand {
        label: "test".to_string(),
        command: "go test ./...".to_string(),
        kind: ProjectCommandKind::Test,
    });
}

// ── Python ──────────────────────────────────────────────────────────────

fn detect_python_commands(path: &Path, stack: &[String], out: &mut Vec<ProjectCommand>) {
    let has_py = path.join("pyproject.toml").exists()
        || path.join("requirements.txt").exists()
        || path.join("Pipfile").exists()
        || path.join("setup.py").exists()
        || path.join("manage.py").exists();

    if !has_py {
        return;
    }

    // Django
    if path.join("manage.py").exists() {
        out.push(ProjectCommand {
            label: "runserver".to_string(),
            command: "python manage.py runserver".to_string(),
            kind: ProjectCommandKind::Start,
        });
    }

    // FastAPI
    if stack.iter().any(|s| s.contains("FastAPI")) {
        out.push(ProjectCommand {
            label: "serve".to_string(),
            command: "uvicorn main:app --reload".to_string(),
            kind: ProjectCommandKind::Start,
        });
    }

    // Flask
    if stack.iter().any(|s| s.contains("Flask")) {
        out.push(ProjectCommand {
            label: "serve".to_string(),
            command: "flask run".to_string(),
            kind: ProjectCommandKind::Start,
        });
    }

    // Install deps
    if path.join("requirements.txt").exists() {
        out.push(ProjectCommand {
            label: "install".to_string(),
            command: "pip install -r requirements.txt".to_string(),
            kind: ProjectCommandKind::Install,
        });
    }

    // Tests
    if path.join("pyproject.toml").exists() {
        out.push(ProjectCommand {
            label: "test".to_string(),
            command: "pytest".to_string(),
            kind: ProjectCommandKind::Test,
        });
    }
}

// ── Docker ──────────────────────────────────────────────────────────────

fn detect_docker_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    let has_compose =
        path.join("docker-compose.yml").exists() || path.join("docker-compose.yaml").exists();
    let has_dockerfile = path.join("Dockerfile").exists();

    if has_compose {
        out.push(ProjectCommand {
            label: "up".to_string(),
            command: "docker compose up".to_string(),
            kind: ProjectCommandKind::Start,
        });
        out.push(ProjectCommand {
            label: "up --build".to_string(),
            command: "docker compose up --build".to_string(),
            kind: ProjectCommandKind::Build,
        });
        out.push(ProjectCommand {
            label: "down".to_string(),
            command: "docker compose down".to_string(),
            kind: ProjectCommandKind::Other,
        });
    }

    if has_dockerfile && !has_compose {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("app");
        out.push(ProjectCommand {
            label: "build".to_string(),
            command: format!("docker build -t {} .", name),
            kind: ProjectCommandKind::Build,
        });
    }
}

// ── .NET ────────────────────────────────────────────────────────────────

fn detect_dotnet_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    let has_dotnet =
        path.join(".sln").exists() || has_extension(path, ".sln") || has_extension(path, ".csproj");

    if !has_dotnet {
        return;
    }

    out.push(ProjectCommand {
        label: "run".to_string(),
        command: "dotnet run".to_string(),
        kind: ProjectCommandKind::Start,
    });
    out.push(ProjectCommand {
        label: "build".to_string(),
        command: "dotnet build".to_string(),
        kind: ProjectCommandKind::Build,
    });
    out.push(ProjectCommand {
        label: "test".to_string(),
        command: "dotnet test".to_string(),
        kind: ProjectCommandKind::Test,
    });
}

fn has_extension(dir: &Path, ext: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().ends_with(ext) {
                return true;
            }
        }
    }
    false
}

// ── Java ────────────────────────────────────────────────────────────────

fn detect_java_commands(path: &Path, out: &mut Vec<ProjectCommand>) {
    if path.join("pom.xml").exists() {
        out.push(ProjectCommand {
            label: "run".to_string(),
            command: "mvn spring-boot:run".to_string(),
            kind: ProjectCommandKind::Start,
        });
        out.push(ProjectCommand {
            label: "test".to_string(),
            command: "mvn test".to_string(),
            kind: ProjectCommandKind::Test,
        });
        out.push(ProjectCommand {
            label: "package".to_string(),
            command: "mvn package".to_string(),
            kind: ProjectCommandKind::Build,
        });
    }

    if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
        let gradle = if cfg!(target_os = "windows") && path.join("gradlew.bat").exists() {
            "gradlew.bat"
        } else if path.join("gradlew").exists() {
            "./gradlew"
        } else {
            "gradle"
        };

        out.push(ProjectCommand {
            label: "bootRun".to_string(),
            command: format!("{} bootRun", gradle),
            kind: ProjectCommandKind::Start,
        });
        out.push(ProjectCommand {
            label: "test".to_string(),
            command: format!("{} test", gradle),
            kind: ProjectCommandKind::Test,
        });
        out.push(ProjectCommand {
            label: "build".to_string(),
            command: format!("{} build", gradle),
            kind: ProjectCommandKind::Build,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn node_package_json_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("package.json");
        fs::write(
            &pkg,
            r#"{"scripts": {"dev": "vite", "build": "tsc", "test": "vitest"}}"#,
        )
        .unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        assert!(cmds
            .iter()
            .any(|c| c.label == "dev" && c.command.contains("run dev")));
        assert!(cmds.iter().any(|c| c.label == "build"));
        assert!(cmds.iter().any(|c| c.label == "test"));
        assert!(cmds.iter().any(|c| c.label == "install"));
    }

    #[test]
    fn pnpm_lock_uses_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"dev": "vite"}}"#,
        )
        .unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        let dev_cmd = cmds.iter().find(|c| c.label == "dev").unwrap();
        assert!(dev_cmd.command.starts_with("pnpm "));
    }

    #[test]
    fn yarn_lock_uses_yarn() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"start": "node index"}}"#,
        )
        .unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        let start_cmd = cmds.iter().find(|c| c.label == "start").unwrap();
        assert!(start_cmd.command.starts_with("yarn "));
    }

    #[test]
    fn flutter_windows_platform() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pubspec.yaml"), "name: test").unwrap();
        fs::create_dir(dir.path().join("windows")).unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        assert!(cmds.iter().any(|c| c.command == "flutter run -d windows"));
        assert!(cmds.iter().any(|c| c.command == "flutter build windows"));
    }

    #[test]
    fn cargo_commands() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        assert!(cmds.iter().any(|c| c.command == "cargo run"));
        assert!(cmds.iter().any(|c| c.command == "cargo build"));
        assert!(cmds.iter().any(|c| c.command == "cargo test"));
    }

    #[test]
    fn go_commands() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("go.mod"), "module test").unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        assert!(cmds.iter().any(|c| c.command == "go run ."));
        assert!(cmds.iter().any(|c| c.command == "go build"));
        assert!(cmds.iter().any(|c| c.command == "go test ./..."));
    }

    #[test]
    fn docker_compose_commands() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("docker-compose.yml"), "services:").unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        assert!(cmds.iter().any(|c| c.command == "docker compose up"));
        assert!(cmds
            .iter()
            .any(|c| c.command == "docker compose up --build"));
        assert!(cmds.iter().any(|c| c.command == "docker compose down"));
    }

    #[test]
    fn corrupt_package_json_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "not valid json {{{").unwrap();
        let _cmds = detect_commands(dir.path(), &[]);
        // Should not panic — any result is fine
    }

    #[test]
    fn limits_to_six_commands() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"dev":"a","start":"b","build":"c","test":"d"}}"#,
        )
        .unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"t\"").unwrap();
        fs::write(dir.path().join("go.mod"), "module t").unwrap();
        let cmds = detect_commands(dir.path(), &[]);
        assert!(cmds.len() <= 6);
    }
}
