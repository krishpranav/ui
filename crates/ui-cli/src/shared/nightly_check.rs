use std::fs;
use std::path::Path;

/* ========================================================== */
/*                        📦 TYPES 📦                         */
/* ========================================================== */

/// Result of checking whether the project is set up for nightly Rust.
/// Both conditions must be true for `bind:value` and other Leptos nightly features to compile.
#[derive(Debug, PartialEq)]
pub struct NightlyStatus {
    /// `rust-toolchain.toml` (or `rust-toolchain`) sets `channel = "nightly"`.
    pub toolchain_nightly: bool,
    /// `Cargo.toml` has `leptos` with `features = ["nightly"]`.
    pub leptos_nightly_feature: bool,
}

impl NightlyStatus {
    pub fn is_ok(&self) -> bool {
        self.toolchain_nightly && self.leptos_nightly_feature
    }

    /// Human-readable list of missing items, for display in warnings.
    pub fn missing_items(&self) -> Vec<&'static str> {
        let mut items = Vec::new();
        if !self.toolchain_nightly {
            items.push("rust-toolchain.toml with channel = \"nightly\"");
        }
        if !self.leptos_nightly_feature {
            items.push("leptos with features = [\"nightly\"] in Cargo.toml");
        }
        items
    }
}

/* ========================================================== */
/*                     ✨ FUNCTIONS ✨                        */
/* ========================================================== */

/// Check whether `dir` has both a nightly toolchain file and leptos nightly feature configured.
pub fn check_nightly_setup(dir: &Path) -> NightlyStatus {
    NightlyStatus {
        toolchain_nightly: has_nightly_toolchain(dir),
        leptos_nightly_feature: has_leptos_nightly_feature(dir),
    }
}

/* ========================================================== */
/*                     🔍 HELPERS 🔍                          */
/* ========================================================== */

/// Returns true if `rust-toolchain.toml` or `rust-toolchain` in `dir` specifies nightly.
fn has_nightly_toolchain(dir: &Path) -> bool {
    // Modern format: rust-toolchain.toml with [toolchain] channel = "nightly"
    if let Ok(content) = fs::read_to_string(dir.join("rust-toolchain.toml")) {
        return parse_toolchain_toml_channel(&content).as_deref() == Some("nightly");
    }
    // Legacy format: rust-toolchain file with plain "nightly" text
    if let Ok(content) = fs::read_to_string(dir.join("rust-toolchain")) {
        return content.lines().find(|l| !l.trim().is_empty()).map(|l| l.trim()) == Some("nightly");
    }
    false
}

/// Returns true if `Cargo.toml` in `dir` has `leptos` with `"nightly"` in features.
/// Checks both `[dependencies]` and `[workspace.dependencies]`.
fn has_leptos_nightly_feature(dir: &Path) -> bool {
    use cargo_toml::Manifest;

    let path = dir.join("Cargo.toml");
    let Ok(manifest) = Manifest::from_path(&path) else {
        return false;
    };

    if leptos_features_contain_nightly(&manifest.dependencies) {
        return true;
    }
    if let Some(ws) = &manifest.workspace {
        if leptos_features_contain_nightly(&ws.dependencies) {
            return true;
        }
    }
    false
}

/// Parse `[toolchain].channel` from a `rust-toolchain.toml` content string.
fn parse_toolchain_toml_channel(content: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct ToolchainFile {
        toolchain: ToolchainSection,
    }
    #[derive(serde::Deserialize)]
    struct ToolchainSection {
        channel: Option<String>,
    }

    toml::from_str::<ToolchainFile>(content).ok().and_then(|f| f.toolchain.channel)
}

fn leptos_features_contain_nightly(deps: &cargo_toml::DepsSet) -> bool {
    deps.get("leptos")
        .map(|dep| dep.req_features().iter().any(|f| f == "nightly"))
        .unwrap_or(false)
}

/* ========================================================== */
/*                        🧪 TESTS 🧪                         */
/* ========================================================== */

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write(dir: &TempDir, name: &str, content: &str) {
        fs::write(dir.path().join(name), content).unwrap();
    }

    // ── toolchain ──────────────────────────────────────────────────────────

    #[test]
    fn toolchain_toml_nightly_is_detected() {
        let dir = TempDir::new().unwrap();
        write(&dir, "rust-toolchain.toml", "[toolchain]\nchannel = \"nightly\"\n");
        assert!(has_nightly_toolchain(dir.path()));
    }

    #[test]
    fn toolchain_toml_stable_returns_false() {
        let dir = TempDir::new().unwrap();
        write(&dir, "rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n");
        assert!(!has_nightly_toolchain(dir.path()));
    }

    #[test]
    fn toolchain_toml_with_targets_nightly_is_detected() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"nightly\"\ntargets = [\"wasm32-unknown-unknown\"]\n",
        );
        assert!(has_nightly_toolchain(dir.path()));
    }

    #[test]
    fn legacy_toolchain_file_nightly_is_detected() {
        let dir = TempDir::new().unwrap();
        write(&dir, "rust-toolchain", "nightly");
        assert!(has_nightly_toolchain(dir.path()));
    }

    #[test]
    fn legacy_toolchain_file_stable_returns_false() {
        let dir = TempDir::new().unwrap();
        write(&dir, "rust-toolchain", "stable");
        assert!(!has_nightly_toolchain(dir.path()));
    }

    #[test]
    fn missing_toolchain_file_returns_false() {
        let dir = TempDir::new().unwrap();
        assert!(!has_nightly_toolchain(dir.path()));
    }

    #[test]
    fn toml_toolchain_takes_priority_over_legacy() {
        let dir = TempDir::new().unwrap();
        // rust-toolchain.toml is stable, rust-toolchain is nightly → stable wins (toml checked first)
        write(&dir, "rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n");
        write(&dir, "rust-toolchain", "nightly");
        assert!(!has_nightly_toolchain(dir.path()));
    }

    // ── leptos features ────────────────────────────────────────────────────

    #[test]
    fn leptos_nightly_feature_in_dependencies_is_detected() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nleptos = { version = \"0.8\", features = [\"nightly\"] }\n",
        );
        assert!(has_leptos_nightly_feature(dir.path()));
    }

    #[test]
    fn leptos_without_nightly_feature_returns_false() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nleptos = { version = \"0.8\", features = [\"csr\"] }\n",
        );
        assert!(!has_leptos_nightly_feature(dir.path()));
    }

    #[test]
    fn leptos_nightly_feature_in_workspace_dependencies_is_detected() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "Cargo.toml",
            "[workspace]\nmembers = []\n\n[workspace.dependencies]\nleptos = { version = \"0.8\", features = [\"nightly\"] }\n",
        );
        assert!(has_leptos_nightly_feature(dir.path()));
    }

    #[test]
    fn leptos_nightly_among_multiple_features_is_detected() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nleptos = { version = \"0.8\", features = [\"csr\", \"nightly\", \"experimental-islands\"] }\n",
        );
        assert!(has_leptos_nightly_feature(dir.path()));
    }

    #[test]
    fn missing_cargo_toml_returns_false() {
        let dir = TempDir::new().unwrap();
        assert!(!has_leptos_nightly_feature(dir.path()));
    }

    #[test]
    fn cargo_toml_without_leptos_returns_false() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
        );
        assert!(!has_leptos_nightly_feature(dir.path()));
    }

    // ── NightlyStatus ──────────────────────────────────────────────────────

    #[test]
    fn status_is_ok_when_both_are_set() {
        let status = NightlyStatus { toolchain_nightly: true, leptos_nightly_feature: true };
        assert!(status.is_ok());
        assert!(status.missing_items().is_empty());
    }

    #[test]
    fn status_not_ok_when_toolchain_missing() {
        let status = NightlyStatus { toolchain_nightly: false, leptos_nightly_feature: true };
        assert!(!status.is_ok());
        assert_eq!(status.missing_items(), vec!["rust-toolchain.toml with channel = \"nightly\""]);
    }

    #[test]
    fn status_not_ok_when_leptos_feature_missing() {
        let status = NightlyStatus { toolchain_nightly: true, leptos_nightly_feature: false };
        assert!(!status.is_ok());
        assert_eq!(
            status.missing_items(),
            vec!["leptos with features = [\"nightly\"] in Cargo.toml"]
        );
    }

    #[test]
    fn status_lists_both_missing_items() {
        let status = NightlyStatus { toolchain_nightly: false, leptos_nightly_feature: false };
        assert!(!status.is_ok());
        assert_eq!(status.missing_items().len(), 2);
    }

    #[test]
    fn check_nightly_setup_full_project() {
        let dir = TempDir::new().unwrap();
        write(&dir, "rust-toolchain.toml", "[toolchain]\nchannel = \"nightly\"\n");
        write(
            &dir,
            "Cargo.toml",
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nleptos = { version = \"0.8\", features = [\"nightly\"] }\n",
        );

        let status = check_nightly_setup(dir.path());
        assert!(status.is_ok());
    }
}
