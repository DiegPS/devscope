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

        if let Some(content) = snapshot.read_to_string("package.json") {
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

        if let Some(content) = snapshot.read_to_string("Cargo.toml") {
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

        let check_file = |filename: &str| -> Option<String> {
            if let Some(content) = snapshot.read_to_string(filename) {
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
                for key in obj.keys() {
                    scripts.push(key.clone());
                }
            }
        }
    }

    scripts
}
