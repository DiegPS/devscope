use std::path::Path;

use crate::snapshot::DirSnapshot;

/// Detect the tech stack of a project by examining its files.
#[allow(dead_code)]
pub fn detect_stack(project_path: &Path) -> Vec<String> {
    detect_stack_with_snapshot(&DirSnapshot::read(project_path))
}

pub(crate) fn detect_stack_with_snapshot(snapshot: &DirSnapshot) -> Vec<String> {
    let project_path = snapshot.root();
    let mut stack = Vec::new();

    // Flutter/Dart
    if snapshot.has("pubspec.yaml") {
        stack.push("Flutter/Dart".to_string());

        if snapshot.has("windows") {
            stack.push("Windows".to_string());
        }
        if snapshot.has("android") {
            stack.push("Android".to_string());
        }
        if snapshot.has("ios") {
            stack.push("iOS".to_string());
        }
        if snapshot.has("web") {
            stack.push("Web".to_string());
        }
        if snapshot.has("linux") {
            stack.push("Linux".to_string());
        }
        if snapshot.has("macos") {
            stack.push("macOS".to_string());
        }
    }

    // Node.js ecosystem
    if snapshot.has("package.json") {
        stack.push("Node".to_string());

        if let Some(json) = snapshot
            .read_to_string("package.json")
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        {
            let names: Vec<_> = [
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ]
            .iter()
            .filter_map(|key| json.get(key).and_then(serde_json::Value::as_object))
            .flat_map(|deps| deps.keys().map(String::as_str))
            .collect();
            for (label, exact, prefix) in [
                ("React", &["react", "react-dom"][..], ""),
                ("Vue", &["vue"][..], "@vue/"),
                ("Svelte", &["svelte"][..], "@sveltejs/"),
                ("Next.js", &["next"][..], ""),
                ("Vite", &["vite"][..], "@vitejs/"),
                ("Tailwind", &["tailwindcss"][..], ""),
                ("Electron", &["electron"][..], ""),
                ("Tauri", &["tauri"][..], "@tauri-apps/"),
                ("Express", &["express"][..], ""),
                ("Fastify", &["fastify"][..], ""),
                ("Nuxt", &["nuxt"][..], ""),
                ("Angular", &["angular"][..], "@angular/"),
                ("TypeScript", &["typescript"][..], ""),
            ] {
                if names.iter().any(|name| {
                    exact.contains(name) || (!prefix.is_empty() && name.starts_with(prefix))
                }) {
                    stack.push(label.into());
                }
            }
        }
    }

    // pnpm / yarn / npm lock files
    if snapshot.has("pnpm-lock.yaml") {
        stack.push("pnpm".to_string());
    } else if snapshot.has("yarn.lock") {
        stack.push("yarn".to_string());
    } else if snapshot.has("package-lock.json") {
        stack.push("npm".to_string());
    }

    // Rust
    if snapshot.has("Cargo.toml") {
        stack.push("Rust".to_string());

        if let Some(manifest) = snapshot
            .read_to_string("Cargo.toml")
            .and_then(|s| s.parse::<toml::Value>().ok())
        {
            let mut names = std::collections::HashSet::new();
            collect_dependencies(&manifest, &mut names);
            for (name, label) in [
                ("ratatui", "Ratatui"),
                ("tauri", "Tauri"),
                ("axum", "Axum"),
                ("actix-web", "Actix"),
                ("bevy", "Bevy"),
                ("tokio", "Tokio"),
                ("serde", "Serde"),
            ] {
                if names.contains(name) {
                    stack.push(label.into());
                }
            }
        }
    }

    // Go
    if snapshot.has("go.mod") {
        stack.push("Go".to_string());
    }

    // Python
    if snapshot.has("pyproject.toml")
        || snapshot.has("requirements.txt")
        || snapshot.has("Pipfile")
        || snapshot.has("setup.py")
    {
        stack.push("Python".to_string());

        let mut names = std::collections::HashSet::new();
        for filename in ["pyproject.toml", "Pipfile"] {
            if let Some(manifest) = snapshot
                .read_to_string(filename)
                .and_then(|s| s.parse::<toml::Value>().ok())
            {
                collect_dependencies(&manifest, &mut names);
            }
        }
        if let Some(requirements) = snapshot.read_to_string("requirements.txt") {
            for line in requirements
                .lines()
                .map(str::trim)
                .filter(|line| !line.starts_with(['#', '-']))
            {
                if let Some(name) = dependency_name(line) {
                    names.insert(name);
                }
            }
        }
        for (name, label) in [
            ("fastapi", "FastAPI"),
            ("django", "Django"),
            ("flask", "Flask"),
            ("torch", "PyTorch"),
            ("tensorflow", "TensorFlow"),
            ("numpy", "NumPy"),
            ("pandas", "Pandas"),
        ] {
            if names.contains(name) {
                stack.push(label.into());
            }
        }
    }

    // Docker
    if snapshot.has("Dockerfile") {
        stack.push("Docker".to_string());
    }
    if snapshot.has("docker-compose.yml") || snapshot.has("docker-compose.yaml") {
        if !stack_contains(&stack, "Docker") {
            stack.push("Docker".to_string());
        }
        stack.push("Compose".to_string());
    }

    // .NET / C#
    if !snapshot.has("Cargo.toml") {
        let has_sln = snapshot
            .entries()
            .iter()
            .any(|entry| entry.is_file && entry.name.ends_with(".sln"));

        let has_csproj = snapshot
            .entries()
            .iter()
            .any(|entry| entry.is_file && entry.name.ends_with(".csproj"));

        if has_sln || has_csproj {
            stack.push(".NET".to_string());
            stack.push("C#".to_string());
        }
    }

    // Java
    if snapshot.has("pom.xml") {
        stack.push("Java".to_string());
        stack.push("Maven".to_string());
    }
    if snapshot.has("build.gradle") || snapshot.has("build.gradle.kts") {
        stack.push("Java".to_string());
        stack.push("Gradle".to_string());
    }

    // Kotlin
    if snapshot.has("build.gradle.kts") {
        if let Some(content) = snapshot.read_to_string("build.gradle.kts") {
            if (content.contains("kotlin") || content.contains("org.jetbrains.kotlin"))
                && !stack_contains(&stack, "Kotlin")
            {
                stack.push("Kotlin".to_string());
            }
        }
    }

    // C/C++
    if snapshot.has("CMakeLists.txt") {
        stack.push("C/C++".to_string());
        stack.push("CMake".to_string());
    }
    if snapshot.has("Makefile")
        && !stack_contains(&stack, "C/C++")
        && (snapshot.has("main.c")
            || snapshot.has("main.cpp")
            || project_path.join("src").join("main.c").exists()
            || project_path.join("src").join("main.cpp").exists())
    {
        stack.push("C/C++".to_string());
    }

    // Ruby
    if snapshot.has("Gemfile") {
        stack.push("Ruby".to_string());
        if snapshot.has("Rakefile") {
            stack.push("Rails".to_string());
        }
    }

    // Swift
    if snapshot.has("Package.swift") {
        stack.push("Swift".to_string());
    }

    // Deno
    if snapshot.has("deno.json") || snapshot.has("deno.jsonc") {
        stack.push("Deno".to_string());
    }

    // Bun
    if snapshot.has("bun.lockb") || snapshot.has("bunfig.toml") {
        stack.push("Bun".to_string());
    }

    // Database / migrations
    let has_migrations = snapshot.has("migrations")
        || project_path.join("db").join("migrations").exists()
        || project_path.join("database").join("migrations").exists()
        || snapshot.has("prisma");

    if has_migrations {
        stack.push("DB".to_string());
    }

    stack
}

fn stack_contains(stack: &[String], needle: &str) -> bool {
    stack.iter().any(|entry| entry == needle)
}

/// Detect the package manager used by the project.
#[allow(dead_code)]
pub fn detect_manager(project_path: &Path) -> Option<String> {
    detect_manager_with_snapshot(&DirSnapshot::read(project_path))
}

pub(crate) fn detect_manager_with_snapshot(snapshot: &DirSnapshot) -> Option<String> {
    if snapshot.has("pnpm-lock.yaml") {
        return Some("pnpm".to_string());
    }
    if snapshot.has("yarn.lock") {
        return Some("yarn".to_string());
    }
    if snapshot.has("package-lock.json") {
        return Some("npm".to_string());
    }
    if snapshot.has("Cargo.toml") {
        return Some("cargo".to_string());
    }
    if snapshot.has("go.mod") {
        return Some("go".to_string());
    }
    if snapshot.has("pubspec.yaml") {
        return Some("pub".to_string());
    }
    if snapshot.has("pyproject.toml") {
        return Some("pip/poetry".to_string());
    }
    if snapshot.has("Pipfile") {
        return Some("pipenv".to_string());
    }
    if snapshot.has("requirements.txt") {
        return Some("pip".to_string());
    }
    if snapshot.has("Gemfile") {
        return Some("bundler".to_string());
    }
    if snapshot.has("pom.xml") {
        return Some("maven".to_string());
    }
    if snapshot.has("build.gradle") || snapshot.has("build.gradle.kts") {
        return Some("gradle".to_string());
    }
    None
}

/// Detect available scripts from package.json.
#[allow(dead_code)]
pub fn detect_scripts(project_path: &Path) -> Vec<String> {
    detect_scripts_with_snapshot(&DirSnapshot::read(project_path))
}

pub(crate) fn detect_scripts_with_snapshot(snapshot: &DirSnapshot) -> Vec<String> {
    let mut scripts = Vec::new();

    if let Some(content) = snapshot.read_to_string("package.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.get("scripts").and_then(|s| s.as_object()) {
                for (key, value) in obj {
                    if !value.is_string() {
                        continue;
                    }
                    scripts.push(key.clone());
                }
            }
        }
    }

    scripts
}

fn dependency_name(text: &str) -> Option<String> {
    let name: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || ['-', '_', '.'].contains(c))
        .collect();
    (!name.is_empty()).then(|| name.to_ascii_lowercase().replace('_', "-"))
}

fn collect_dependencies(value: &toml::Value, names: &mut std::collections::HashSet<String>) {
    if let Some(table) = value.as_table() {
        for (key, value) in table {
            if [
                "dependencies",
                "dev-dependencies",
                "build-dependencies",
                "packages",
                "dev-packages",
            ]
            .contains(&key.as_str())
            {
                match value {
                    toml::Value::Table(deps) => {
                        for (name, detail) in deps {
                            let name = detail
                                .get("package")
                                .and_then(toml::Value::as_str)
                                .unwrap_or(name);
                            if let Some(name) = dependency_name(name) {
                                names.insert(name);
                            }
                        }
                    }
                    toml::Value::Array(deps) => {
                        for dep in deps.iter().filter_map(toml::Value::as_str) {
                            if let Some(name) = dependency_name(dep) {
                                names.insert(name);
                            }
                        }
                    }
                    _ => {}
                }
            } else if [
                "workspace",
                "target",
                "project",
                "tool",
                "poetry",
                "group",
                "optional-dependencies",
                "dependency-groups",
            ]
            .contains(&key.as_str())
                || value
                    .as_table()
                    .is_some_and(|table| table.contains_key("dependencies"))
                || key.starts_with("cfg(")
            {
                collect_dependencies(value, names);
                if ["optional-dependencies", "dependency-groups"].contains(&key.as_str()) {
                    if let Some(groups) = value.as_table() {
                        for deps in groups.values().filter_map(toml::Value::as_array) {
                            for dep in deps.iter().filter_map(toml::Value::as_str) {
                                if let Some(name) = dependency_name(dep) {
                                    names.insert(name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
