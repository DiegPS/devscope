use std::path::Path;

/// Detect the tech stack of a project by examining its files.
pub fn detect_stack(project_path: &Path) -> Vec<String> {
    let mut stack = Vec::new();

    // Flutter/Dart
    if project_path.join("pubspec.yaml").exists() {
        stack.push("Flutter/Dart".to_string());

        if project_path.join("windows").exists() {
            stack.push("Windows".to_string());
        }
        if project_path.join("android").exists() {
            stack.push("Android".to_string());
        }
        if project_path.join("ios").exists() {
            stack.push("iOS".to_string());
        }
        if project_path.join("web").exists() {
            stack.push("Web".to_string());
        }
        if project_path.join("linux").exists() {
            stack.push("Linux".to_string());
        }
        if project_path.join("macos").exists() {
            stack.push("macOS".to_string());
        }
    }

    // Node.js ecosystem
    if project_path.join("package.json").exists() {
        stack.push("Node".to_string());

        if let Ok(content) = std::fs::read_to_string(project_path.join("package.json")) {
            let lower = content.to_lowercase();

            if lower.contains("\"react\"") || lower.contains("\"react-dom\"") {
                stack.push("React".to_string());
            }
            if lower.contains("\"vue\"") || lower.contains("\"@vue/") {
                stack.push("Vue".to_string());
            }
            if lower.contains("\"svelte\"") || lower.contains("\"@sveltejs/") {
                stack.push("Svelte".to_string());
            }
            if lower.contains("\"next\"") || lower.contains("\"next/") {
                stack.push("Next.js".to_string());
            }
            if lower.contains("\"vite\"") || lower.contains("\"@vitejs/") {
                stack.push("Vite".to_string());
            }
            if lower.contains("\"tailwindcss\"") {
                stack.push("Tailwind".to_string());
            }
            if lower.contains("\"electron\"") {
                stack.push("Electron".to_string());
            }
            if lower.contains("\"tauri\"") || lower.contains("\"@tauri-apps/") {
                stack.push("Tauri".to_string());
            }
            if lower.contains("\"express\"") {
                stack.push("Express".to_string());
            }
            if lower.contains("\"fastify\"") {
                stack.push("Fastify".to_string());
            }
            if lower.contains("\"nuxt\"") {
                stack.push("Nuxt".to_string());
            }
            if lower.contains("\"angular\"") || lower.contains("\"@angular/") {
                stack.push("Angular".to_string());
            }
            if lower.contains("\"typescript\"") {
                stack.push("TypeScript".to_string());
            }
        }
    }

    // pnpm / yarn / npm lock files
    if project_path.join("pnpm-lock.yaml").exists() {
        stack.push("pnpm".to_string());
    } else if project_path.join("yarn.lock").exists() {
        stack.push("yarn".to_string());
    } else if project_path.join("package-lock.json").exists() {
        stack.push("npm".to_string());
    }

    // Rust
    if project_path.join("Cargo.toml").exists() {
        stack.push("Rust".to_string());

        if let Ok(content) = std::fs::read_to_string(project_path.join("Cargo.toml")) {
            let lower = content.to_lowercase();
            if lower.contains("ratatui") {
                stack.push("Ratatui".to_string());
            }
            if lower.contains("tauri") {
                stack.push("Tauri".to_string());
            }
            if lower.contains("axum") {
                stack.push("Axum".to_string());
            }
            if lower.contains("actix-web") || lower.contains("actix_web") {
                stack.push("Actix".to_string());
            }
            if lower.contains("bevy") {
                stack.push("Bevy".to_string());
            }
            if lower.contains("tokio") {
                stack.push("Tokio".to_string());
            }
            if lower.contains("serde") {
                stack.push("Serde".to_string());
            }
        }
    }

    // Go
    if project_path.join("go.mod").exists() {
        stack.push("Go".to_string());
    }

    // Python
    if project_path.join("pyproject.toml").exists()
        || project_path.join("requirements.txt").exists()
        || project_path.join("Pipfile").exists()
        || project_path.join("setup.py").exists()
    {
        stack.push("Python".to_string());

        let check_file = |filename: &str| -> Option<String> {
            if let Ok(content) = std::fs::read_to_string(project_path.join(filename)) {
                let lower = content.to_lowercase();
                if lower.contains("fastapi") {
                    return Some("FastAPI".to_string());
                }
                if lower.contains("django") {
                    return Some("Django".to_string());
                }
                if lower.contains("flask") {
                    return Some("Flask".to_string());
                }
                if lower.contains("torch") || lower.contains("pytorch") {
                    return Some("PyTorch".to_string());
                }
                if lower.contains("tensorflow") {
                    return Some("TensorFlow".to_string());
                }
                if lower.contains("numpy") {
                    return Some("NumPy".to_string());
                }
                if lower.contains("pandas") {
                    return Some("Pandas".to_string());
                }
            }
            None
        };

        for f in &["pyproject.toml", "requirements.txt", "Pipfile"] {
            if let Some(framework) = check_file(f) {
                if !stack.contains(&framework) {
                    stack.push(framework);
                }
            }
        }
    }

    // Docker
    if project_path.join("Dockerfile").exists() {
        stack.push("Docker".to_string());
    }
    if project_path.join("docker-compose.yml").exists()
        || project_path.join("docker-compose.yaml").exists()
    {
        if !stack.contains(&"Docker".to_string()) {
            stack.push("Docker".to_string());
        }
        stack.push("Compose".to_string());
    }

    // .NET / C#
    if !project_path.join("Cargo.toml").exists() {
        let has_sln = std::fs::read_dir(project_path)
            .ok()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "sln")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        let has_csproj = std::fs::read_dir(project_path)
            .ok()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "csproj")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if has_sln || has_csproj {
            stack.push(".NET".to_string());
            stack.push("C#".to_string());
        }
    }

    // Java
    if project_path.join("pom.xml").exists() {
        stack.push("Java".to_string());
        stack.push("Maven".to_string());
    }
    if project_path.join("build.gradle").exists() || project_path.join("build.gradle.kts").exists()
    {
        stack.push("Java".to_string());
        stack.push("Gradle".to_string());
    }

    // Kotlin
    if project_path.join("build.gradle.kts").exists() {
        if let Ok(content) = std::fs::read_to_string(project_path.join("build.gradle.kts")) {
            if (content.contains("kotlin") || content.contains("org.jetbrains.kotlin"))
                && !stack.contains(&"Kotlin".to_string())
            {
                stack.push("Kotlin".to_string());
            }
        }
    }

    // C/C++
    if project_path.join("CMakeLists.txt").exists() {
        stack.push("C/C++".to_string());
        stack.push("CMake".to_string());
    }
    if project_path.join("Makefile").exists()
        && !stack.contains(&"C/C++".to_string())
        && (project_path.join("main.c").exists()
            || project_path.join("main.cpp").exists()
            || project_path.join("src").join("main.c").exists()
            || project_path.join("src").join("main.cpp").exists())
    {
        stack.push("C/C++".to_string());
    }

    // Ruby
    if project_path.join("Gemfile").exists() {
        stack.push("Ruby".to_string());
        if project_path.join("Rakefile").exists() {
            stack.push("Rails".to_string());
        }
    }

    // Swift
    if project_path.join("Package.swift").exists() {
        stack.push("Swift".to_string());
    }

    // Deno
    if project_path.join("deno.json").exists() || project_path.join("deno.jsonc").exists() {
        stack.push("Deno".to_string());
    }

    // Bun
    if project_path.join("bun.lockb").exists() || project_path.join("bunfig.toml").exists() {
        stack.push("Bun".to_string());
    }

    // Database / migrations
    let has_migrations = project_path.join("migrations").exists()
        || project_path.join("db").join("migrations").exists()
        || project_path.join("database").join("migrations").exists()
        || project_path.join("prisma").exists();

    if has_migrations {
        stack.push("DB".to_string());
    }

    stack
}

/// Detect the package manager used by the project.
pub fn detect_manager(project_path: &Path) -> Option<String> {
    if project_path.join("pnpm-lock.yaml").exists() {
        return Some("pnpm".to_string());
    }
    if project_path.join("yarn.lock").exists() {
        return Some("yarn".to_string());
    }
    if project_path.join("package-lock.json").exists() {
        return Some("npm".to_string());
    }
    if project_path.join("Cargo.toml").exists() {
        return Some("cargo".to_string());
    }
    if project_path.join("go.mod").exists() {
        return Some("go".to_string());
    }
    if project_path.join("pubspec.yaml").exists() {
        return Some("pub".to_string());
    }
    if project_path.join("pyproject.toml").exists() {
        return Some("pip/poetry".to_string());
    }
    if project_path.join("Pipfile").exists() {
        return Some("pipenv".to_string());
    }
    if project_path.join("requirements.txt").exists() {
        return Some("pip".to_string());
    }
    if project_path.join("Gemfile").exists() {
        return Some("bundler".to_string());
    }
    if project_path.join("pom.xml").exists() {
        return Some("maven".to_string());
    }
    if project_path.join("build.gradle").exists() || project_path.join("build.gradle.kts").exists()
    {
        return Some("gradle".to_string());
    }
    None
}

/// Detect available scripts from package.json.
pub fn detect_scripts(project_path: &Path) -> Vec<String> {
    let mut scripts = Vec::new();

    if let Ok(content) = std::fs::read_to_string(project_path.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = json.get("scripts").and_then(|s| s.as_object()) {
                for key in obj.keys() {
                    scripts.push(key.clone());
                }
            }
        }
    }

    scripts
}
