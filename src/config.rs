//! Configuration loaded from `.arkoudarc.toml`.
//!
//! Discovery walks up from the starting directory (typically `cwd`) until it
//! finds a `.arkoudarc.toml` or hits the filesystem root. Relative paths in
//! `dirs` are resolved against the directory containing the config file, so
//! the same config works regardless of which subdirectory arkouda is invoked
//! from.

use crate::error::{ArkoudaError, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const FILENAME: &str = ".arkoudarc.toml";
const DEFAULT_DIR: &str = "docs/adr";

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    dirs: Vec<PathBuf>,
}

/// Effective list of directories to scan for ADRs, given CLI overrides and
/// any `.arkoudarc.toml` discovered up the tree from `start`.
///
/// Precedence: explicit `cli_dir` > `.arkoudarc.toml` `dirs` > default
/// (`docs/adr`).
pub fn effective_dirs(cli_dir: Option<&Path>, start: &Path) -> Result<Vec<PathBuf>> {
    if let Some(dir) = cli_dir {
        return Ok(vec![dir.to_path_buf()]);
    }
    if let Some(dirs) = discover(start)?
        && !dirs.is_empty()
    {
        return Ok(dirs);
    }
    Ok(vec![PathBuf::from(DEFAULT_DIR)])
}

fn discover(start: &Path) -> Result<Option<Vec<PathBuf>>> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            return Ok(Some(parse(&text, ancestor).map_err(|message| {
                ArkoudaError::Config {
                    path: candidate.display().to_string(),
                    message,
                }
            })?));
        }
    }
    Ok(None)
}

fn parse(text: &str, base: &Path) -> std::result::Result<Vec<PathBuf>, String> {
    let parsed: ConfigFile = toml::from_str(text).map_err(|err| err.to_string())?;
    Ok(parsed
        .dirs
        .into_iter()
        .map(|dir| {
            if dir.is_absolute() {
                dir
            } else {
                base.join(dir)
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dirs_relative_to_base() {
        let dirs = parse(
            "dirs = [\"docs/adr\", \"services/foo/adr\"]\n",
            Path::new("/repo"),
        )
        .expect("ok");
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/repo/docs/adr"),
                PathBuf::from("/repo/services/foo/adr"),
            ],
        );
    }

    #[test]
    fn keeps_absolute_paths_as_is() {
        let dirs = parse("dirs = [\"/abs/adr\"]\n", Path::new("/repo")).expect("ok");
        assert_eq!(dirs, vec![PathBuf::from("/abs/adr")]);
    }

    #[test]
    fn empty_dirs_returns_empty_list() {
        let dirs = parse("dirs = []\n", Path::new("/repo")).expect("ok");
        assert!(dirs.is_empty());
    }

    #[test]
    fn missing_dirs_key_returns_empty_list() {
        let dirs = parse("", Path::new("/repo")).expect("ok");
        assert!(dirs.is_empty());
    }

    #[test]
    fn malformed_toml_is_an_error() {
        let result = parse("dirs = not-a-list\n", Path::new("/repo"));
        assert!(result.is_err());
    }
}
