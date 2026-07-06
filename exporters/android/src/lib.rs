//! Android exporter (milestone M15): turns an `.aigs` project into a signed
//! APK.
//!
//! Unlike Desktop (M7) and Web (M14), there is no single prebuilt "generic
//! player" artifact to just copy: Android bakes a project's assets into the
//! APK at *build* time (there's no runtime `fetch`, and reading arbitrary
//! files from outside the APK needs user-granted storage permissions we'd
//! rather not require). So exporting a project here means: copy the
//! `android-player` template (`exporters/android-player`) to a scratch
//! build directory, drop the project's data into its bundled assets, give
//! it a project-specific package id so multiple exported games can be
//! installed side by side, and actually run `cargo apk build` — the
//! exporting machine needs the Android NDK/SDK and `cargo-apk` installed,
//! the same way exporting for iOS needs Xcode.

use std::path::{Path, PathBuf};
use std::process::Command;

use aigs_project::{Project, Scene};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("manifest: {0}")]
    Format(#[from] aigs_project::FormatError),
    #[error("io error on {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("output {0} already exists; remove it or choose another --output")]
    OutputExists(String),
    #[error(
        "`cargo apk build` failed (exit {0}); make sure the Android NDK/SDK and cargo-apk \
         are installed and ANDROID_HOME/ANDROID_NDK_HOME are set"
    )]
    Build(std::process::ExitStatus),
    #[error("cargo-apk did not produce an APK at the expected path {0}")]
    ApkNotFound(PathBuf),
}

/// Options for an Android export.
pub struct ExportOptions<'a> {
    /// Checkout of the `android-player` template crate to build from.
    pub player_template: &'a Path,
    /// Directory where the scratch build lives and the final `.apk` is copied.
    pub output: &'a Path,
    /// `cargo apk build --release` (needs a signing keystore configured in
    /// the template's `Cargo.toml`) instead of a debug build (works out of
    /// the box, self-signed with a local debug keystore, not for distribution).
    pub release: bool,
}

/// What an export produced, for reporting.
pub struct ExportReport {
    pub apk: PathBuf,
    pub build_dir: PathBuf,
}

/// Exports the project at `manifest` as a signed Android APK.
pub fn export(manifest: &Path, options: &ExportOptions) -> Result<ExportReport, ExportError> {
    let project = Project::load(manifest)?;
    let root = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();

    for scene_path in &project.scenes {
        Scene::load(&root.join(scene_path))?;
    }

    let slug = slugify(&project.name);
    let build_dir = options.output.join(format!("{slug}-android"));
    if build_dir.exists() {
        return Err(ExportError::OutputExists(build_dir.display().to_string()));
    }

    copy_dir_all(options.player_template, &build_dir)?;
    retarget_package(&build_dir, &slug)?;
    fix_path_dependencies(&build_dir, options.player_template)?;

    // Project data: manifest + scenes + assets, preserving relative paths —
    // the same layout `AndroidAssets` reads at runtime.
    let assets_dir = build_dir.join("assets/data");
    let _ = std::fs::remove_dir_all(&assets_dir); // clear the template's placeholder
    create_dir(&assets_dir)?;
    copy_file(manifest, &assets_dir.join("game.aigs"))?;
    for relative in project
        .scenes
        .iter()
        .chain(project.assets.iter().map(|asset| &asset.path))
    {
        let destination = assets_dir.join(relative);
        if let Some(parent) = destination.parent() {
            create_dir(parent)?;
        }
        copy_file(&root.join(relative), &destination)?;
    }

    let mut command = Command::new("cargo");
    command.arg("apk").arg("build").current_dir(&build_dir);
    if options.release {
        command.arg("--release");
    }
    let status = command.status().map_err(|source| ExportError::Io {
        path: "cargo apk build".to_string(),
        source,
    })?;
    if !status.success() {
        return Err(ExportError::Build(status));
    }

    let profile_dir = if options.release { "release" } else { "debug" };
    let apk = build_dir
        .join("target")
        .join(profile_dir)
        .join("apk")
        .join(format!("{slug}.apk"));
    if !apk.is_file() {
        return Err(ExportError::ApkNotFound(apk));
    }
    Ok(ExportReport { apk, build_dir })
}

/// Gives the copied template a project-specific package id and apk name
/// (`apk_name`/`package` in `[package.metadata.android]`) so exported games
/// don't collide when installed side by side. The template's values are
/// unique, fixed strings specifically so this substitution is unambiguous.
fn retarget_package(build_dir: &Path, slug: &str) -> Result<(), ExportError> {
    let manifest_path = build_dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path).map_err(|source| ExportError::Io {
        path: manifest_path.display().to_string(),
        source,
    })?;
    let package_id = format!("studio.aigamestudio.player.{}", slug.replace('-', "_"));
    let text = text
        .replace(
            "package = \"studio.aigamestudio.player\"",
            &format!("package = \"{package_id}\""),
        )
        .replace(
            "apk_name = \"aigs-player\"",
            &format!("apk_name = \"{slug}\""),
        );
    std::fs::write(&manifest_path, text).map_err(|source| ExportError::Io {
        path: manifest_path.display().to_string(),
        source,
    })
}

/// The template's `Cargo.toml` depends on `aigs-project`/`aigs-runtime` by
/// a path *relative to itself* (`../../runtime/crates/...`); once copied to
/// a scratch build directory elsewhere, that relative path points nowhere.
/// Rewrites it to the equivalent absolute path, resolved against the
/// original (uncopied) template location.
fn fix_path_dependencies(build_dir: &Path, original_template: &Path) -> Result<(), ExportError> {
    let manifest_path = build_dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path).map_err(|source| ExportError::Io {
        path: manifest_path.display().to_string(),
        source,
    })?;
    let mut fixed = text;
    for crate_name in ["aigs-project", "aigs-runtime"] {
        let relative = format!("../../runtime/crates/{crate_name}");
        let absolute = original_template
            .join(&relative)
            .canonicalize()
            .map_err(|source| ExportError::Io {
                path: format!(
                    "{} (does the player template ship next to a full runtime/crates checkout?)",
                    original_template.join(&relative).display()
                ),
                source,
            })?;
        fixed = fixed.replace(
            &format!("path = \"{relative}\""),
            &format!("path = {absolute:?}"),
        );
    }
    std::fs::write(&manifest_path, fixed).map_err(|source| ExportError::Io {
        path: manifest_path.display().to_string(),
        source,
    })
}

/// Folder/package name derived from the game name (same rule as the other
/// exporters, kept independent per-crate rather than shared).
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    let mut collapsed = String::with_capacity(slug.len());
    let mut previous_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !previous_dash {
                collapsed.push('-');
            }
            previous_dash = true;
        } else {
            collapsed.push(c);
            previous_dash = false;
        }
    }
    if collapsed.is_empty() {
        "game".to_string()
    } else {
        collapsed
    }
}

fn create_dir(path: &Path) -> Result<(), ExportError> {
    std::fs::create_dir_all(path).map_err(|source| ExportError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn copy_file(from: &Path, to: &Path) -> Result<(), ExportError> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|source| ExportError::Io {
            path: from.display().to_string(),
            source,
        })
}

fn copy_dir_all(from: &Path, to: &Path) -> Result<(), ExportError> {
    create_dir(to)?;
    let entries = std::fs::read_dir(from).map_err(|source| ExportError::Io {
        path: from.display().to_string(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ExportError::Io {
            path: from.display().to_string(),
            source,
        })?;
        let path = entry.path();
        // The template's own build output must never be copied along.
        if path.file_name().is_some_and(|name| name == "target") {
            continue;
        }
        let destination = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &destination)?;
        } else {
            copy_file(&path, &destination)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_handles_names() {
        assert_eq!(slugify("Robot Rescue"), "robot-rescue");
        assert_eq!(slugify("¡Mi Juego 2!"), "mi-juego-2");
        assert_eq!(slugify("***"), "game");
    }

    #[test]
    fn copy_dir_all_skips_target_and_preserves_structure() {
        let temp = tempfile::tempdir().unwrap();
        let from = temp.path().join("template");
        std::fs::create_dir_all(from.join("src")).unwrap();
        std::fs::create_dir_all(from.join("target/debug")).unwrap();
        std::fs::write(from.join("Cargo.toml"), b"[package]").unwrap();
        std::fs::write(from.join("src/lib.rs"), b"fn f() {}").unwrap();
        std::fs::write(from.join("target/debug/junk"), b"stale build output").unwrap();

        let to = temp.path().join("copy");
        copy_dir_all(&from, &to).unwrap();

        assert!(to.join("Cargo.toml").is_file());
        assert!(to.join("src/lib.rs").is_file());
        assert!(
            !to.join("target").exists(),
            "must not copy the template's own build output"
        );
    }

    #[test]
    fn retarget_package_rewrites_the_template_ids() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "package = \"studio.aigamestudio.player\"\napk_name = \"aigs-player\"\n",
        )
        .unwrap();

        retarget_package(temp.path(), "robot-rescue").unwrap();

        let text = std::fs::read_to_string(temp.path().join("Cargo.toml")).unwrap();
        assert!(text.contains("package = \"studio.aigamestudio.player.robot_rescue\""));
        assert!(text.contains("apk_name = \"robot-rescue\""));
    }

    #[test]
    fn fix_path_dependencies_rewrites_relative_paths_to_absolute() {
        let temp = tempfile::tempdir().unwrap();
        // Mirrors the real repo layout (exporters/android-player is two
        // levels under the repo root, siblings with runtime/crates/...) so
        // `../../runtime/crates/...` from the template actually resolves.
        let original_template = temp.path().join("repo/exporters/android-player");
        std::fs::create_dir_all(&original_template).unwrap();
        std::fs::create_dir_all(temp.path().join("repo/runtime/crates/aigs-project")).unwrap();
        std::fs::create_dir_all(temp.path().join("repo/runtime/crates/aigs-runtime")).unwrap();
        let build_dir = temp.path().join("copy");
        std::fs::create_dir_all(&build_dir).unwrap();
        std::fs::write(
            build_dir.join("Cargo.toml"),
            "aigs-project = { path = \"../../runtime/crates/aigs-project\" }\n\
             aigs-runtime = { path = \"../../runtime/crates/aigs-runtime\" }\n",
        )
        .unwrap();

        fix_path_dependencies(&build_dir, &original_template).unwrap();

        // Windows' canonicalize() yields backslash-separated (and escaped,
        // once written into a quoted TOML string) paths, so this checks for
        // the crate names rather than assuming a separator style.
        let text = std::fs::read_to_string(build_dir.join("Cargo.toml")).unwrap();
        assert!(
            !text.contains("../../runtime"),
            "no relative path left: {text}"
        );
        assert!(text.contains("aigs-project"));
        assert!(text.contains("aigs-runtime"));
        assert!(
            text.contains("runtime"),
            "path must still reference runtime/crates: {text}"
        );
    }
}
