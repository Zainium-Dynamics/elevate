//! Install and runtime paths for elevate.
//!
//! Read from the `[paths]` table of a small TOML file. The file is the first
//! of these that exists:
//!
//! 1. `$ELEVATE_CONFIG`
//! 2. `/etc/elevate/config.toml`
//! 3. `/overlayer/syshub/etc/elevate/config.toml`
//! 4. `./elevate.toml` (dev checkout)
//!
//! ```toml
//! [paths]
//! prefix = "/overlayer/syshub"   # everything below derives from this
//! # etc_dir = "<prefix>/etc"
//! # bindir  = "<prefix>/bin"
//! # sbindir = "<prefix>/bin"
//! # libdir  = "<prefix>/lib"
//! # elevators_dir  = "<prefix>/etc/elevators"
//! # elevators_file = "<elevators_dir>/elevate.toml"
//! ```
//!
//! Anything not set is derived from the resolved `prefix`, so changing only
//! the prefix moves every path. With no file, the defaults are Zainium's
//! layout under `/overlayer/syshub`.
//!
//! The parser only understands what this needs (a `[paths]` table of
//! `key = "string"` lines), so there is no `serde` / `toml` dependency.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const DEFAULT_PREFIX: &str = "/overlayer/syshub";
const CONFIG_ENV: &str = "ELEVATE_CONFIG";
const CONFIG_NAME: &str = "elevate";

/// Resolved locations.
#[derive(Debug, Clone)]
pub struct PathsConfig {
    pub prefix: String,
    pub bindir: String,
    pub sbindir: String,
    pub libdir: String,
    pub elevators_file: String,
    etc_dir: String,
}

#[derive(Debug, Default)]
struct Raw {
    prefix: Option<String>,
    etc_dir: Option<String>,
    bindir: Option<String>,
    sbindir: Option<String>,
    libdir: Option<String>,
    elevators_dir: Option<String>,
    elevators_file: Option<String>,
}

fn clean(s: String) -> String {
    let t = s.trim_end_matches('/');
    if t.is_empty() && !s.is_empty() {
        "/".to_string()
    } else {
        t.to_string()
    }
}

/// Parse the `[paths]` table out of a TOML document.
fn parse(text: &str) -> Raw {
    let mut raw = Raw::default();
    let mut in_paths = false;
    for line in text.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_paths = line == "[paths]";
            continue;
        }
        if !in_paths {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let Some(value) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else {
            continue;
        };
        let value = Some(value.to_string());
        match key.trim() {
            "prefix" => raw.prefix = value,
            "etc_dir" => raw.etc_dir = value,
            "bindir" => raw.bindir = value,
            "sbindir" => raw.sbindir = value,
            "libdir" => raw.libdir = value,
            "elevators_dir" => raw.elevators_dir = value,
            "elevators_file" => raw.elevators_file = value,
            _ => {}
        }
    }
    raw
}

/// Drop a trailing `# comment`, ignoring `#` inside a quoted string.
fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &line[..i],
            _ => {}
        }
    }
    line
}

impl From<Raw> for PathsConfig {
    fn from(raw: Raw) -> Self {
        let prefix = clean(raw.prefix.unwrap_or_else(|| DEFAULT_PREFIX.to_string()));
        let etc_dir = clean(raw.etc_dir.unwrap_or_else(|| format!("{prefix}/etc")));
        let elevators_dir = clean(
            raw.elevators_dir
                .unwrap_or_else(|| format!("{prefix}/etc/elevators")),
        );
        Self {
            bindir: clean(raw.bindir.unwrap_or_else(|| format!("{prefix}/bin"))),
            sbindir: clean(raw.sbindir.unwrap_or_else(|| format!("{prefix}/bin"))),
            libdir: clean(raw.libdir.unwrap_or_else(|| format!("{prefix}/lib"))),
            elevators_file: raw
                .elevators_file
                .unwrap_or_else(|| format!("{elevators_dir}/elevate.toml")),
            etc_dir,
            prefix,
        }
    }
}

impl Default for PathsConfig {
    fn default() -> Self {
        Raw::default().into()
    }
}

impl PathsConfig {
    fn candidates() -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(explicit) = std::env::var(CONFIG_ENV) {
            out.push(PathBuf::from(explicit));
        }
        out.push(PathBuf::from(format!("/etc/{CONFIG_NAME}/config.toml")));
        out.push(PathBuf::from(format!(
            "{DEFAULT_PREFIX}/etc/{CONFIG_NAME}/config.toml"
        )));
        out.push(PathBuf::from(format!("{CONFIG_NAME}.toml")));
        out
    }

    fn load_path(path: impl AsRef<Path>) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        Some(parse(&text).into())
    }

    fn load() -> Self {
        Self::candidates()
            .iter()
            .find_map(Self::load_path)
            .unwrap_or_default()
    }

    /// Process-wide config, loaded on first use.
    pub fn get() -> &'static Self {
        static CACHE: OnceLock<PathsConfig> = OnceLock::new();
        CACHE.get_or_init(Self::load)
    }

    pub fn shells_file(&self) -> String {
        format!("{}/shells", self.etc_dir)
    }
    /// Zoneinfo directories to try, in order. The standard location is the
    /// last fallback for hosts without a Zainium tree.
    pub fn zoneinfo_dirs(&self) -> [String; 3] {
        [
            format!("{}/zoneinfo", self.etc_dir),
            format!("{}/lib/zoneinfo", self.prefix),
            String::from("/usr/share/zoneinfo"),
        ]
    }
    /// The directory holding `prefix` (`/overlayer`). Protected only as itself: userland
    /// (`zexlib`) and `agents` live under it and stay root's to manage.
    pub fn overlay_root(&self) -> String {
        let prefix = self.prefix.trim_end_matches('/');
        match prefix.rfind('/') {
            Some(0) | None => prefix.to_string(),
            Some(pos) => prefix[..pos].to_string(),
        }
    }

    /// Immutable core layers that destructive commands must not touch, with
    /// everything under them: `prefix` itself, the sibling `zaisys` layer, and
    /// the staged base upgrade waiting for the next boot.
    pub fn protected_layers(&self) -> Vec<String> {
        let overlay_root = self.overlay_root();
        vec![
            self.prefix.trim_end_matches('/').to_string(),
            format!("{overlay_root}/zaisys"),
            format!("{overlay_root}/.syshub-pending"),
        ]
    }
}

/// Shortcut for `PathsConfig::get()`.
pub fn get() -> &'static PathsConfig {
    PathsConfig::get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_zainium_layout() {
        let c = PathsConfig::default();
        assert_eq!(c.prefix, DEFAULT_PREFIX);
        assert_eq!(c.bindir, format!("{DEFAULT_PREFIX}/bin"));
        assert_eq!(c.shells_file(), format!("{DEFAULT_PREFIX}/etc/shells"));
        assert_eq!(
            c.elevators_file,
            format!("{DEFAULT_PREFIX}/etc/elevators/elevate.toml")
        );
    }

    #[test]
    fn prefix_cascades() {
        let c: PathsConfig = parse("[paths]\nprefix = \"/opt/x/\"\n").into();
        assert_eq!(c.prefix, "/opt/x");
        assert_eq!(c.libdir, "/opt/x/lib");
        assert_eq!(c.shells_file(), "/opt/x/etc/shells");
        assert_eq!(c.elevators_file, "/opt/x/etc/elevators/elevate.toml");
    }

    #[test]
    fn explicit_keys_win_and_other_tables_are_ignored() {
        let text = "[project]\nprefix = \"/nope\"\n\n[paths]\nprefix = \"/opt/x\" # c\nlibdir = \"/lib64\"\n";
        let c: PathsConfig = parse(text).into();
        assert_eq!(c.prefix, "/opt/x");
        assert_eq!(c.libdir, "/lib64");
        assert_eq!(c.bindir, "/opt/x/bin");
    }

    #[test]
    fn hash_inside_quotes_is_kept() {
        assert_eq!(strip_comment("a = \"b#c\" # d").trim(), "a = \"b#c\"");
    }

    #[test]
    fn protected_layers_follow_prefix() {
        let c: PathsConfig = parse("[paths]\nprefix = \"/overlayer/syshub\"\n").into();
        assert_eq!(c.overlay_root(), "/overlayer");
        assert_eq!(
            c.protected_layers(),
            vec!["/overlayer/syshub", "/overlayer/zaisys", "/overlayer/.syshub-pending"]
        );
    }
}
