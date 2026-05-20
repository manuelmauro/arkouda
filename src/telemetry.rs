//! Local invocation telemetry.
//!
//! Each arkouda invocation appends one JSON event to a local log file under
//! the OS state directory. Network access is never used; ADR content never
//! leaves the user's machine. See ADR
//! `telemetry-for-agent-command-invocations` for the full rationale.

use crate::cli::{Cli, Command};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const APP_DIR: &str = "arkouda";
const LOG_FILE: &str = "telemetry.jsonl";
const NOTICE_SENTINEL: &str = ".notice-shown";
const ROTATE_BYTES: u64 = 10 * 1024 * 1024;
const ENV_OPT_OUT: &str = "ARKOUDA_TELEMETRY";

/// Allowlist of env vars that identify a known agent harness. First match
/// wins; the value is the agent's stable short id used in the event.
const AGENT_ENV: &[(&str, &str)] = &[
    ("CLAUDECODE", "claude-code"),
    ("CURSOR_AGENT", "cursor"),
    ("AIDER", "aider"),
];

/// A single CLI invocation event.
#[derive(Debug, Serialize)]
pub struct Event {
    /// UTC ISO-8601 timestamp, second precision.
    pub ts: String,
    /// Arkouda's own version (Cargo `CARGO_PKG_VERSION`).
    pub version: &'static str,
    /// Subcommand name, or `None` for `--help`/`--version`/parse failure.
    pub command: Option<&'static str>,
    /// Argv tail after the program name, with paths and free-text titles
    /// redacted to their kind. Flag names and short slug/enum values are
    /// kept verbatim.
    pub args: Vec<String>,
    /// Process exit code (0 success, 1 failure).
    pub exit_code: i32,
    /// Wall-clock duration of the invocation in milliseconds.
    pub duration_ms: u128,
    /// Detected agent short id, or `None` when no allowlisted env var matched.
    pub agent: Option<&'static str>,
    /// Which env var produced the agent detection (e.g. `env:CLAUDECODE`).
    pub agent_source: Option<String>,
    /// Whether stdout is a TTY at invocation time.
    pub tty: bool,
}

impl Event {
    /// Build an event from a parsed CLI, raw argv, exit code, and elapsed time.
    pub fn capture(cli: &Cli, raw_argv: &[String], exit_code: i32, elapsed: Duration) -> Self {
        let command = cli.command.name();
        let (agent, agent_source) = detect_agent();
        Self {
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            version: env!("CARGO_PKG_VERSION"),
            command: Some(command),
            args: redact_args(raw_argv, Some(command)),
            exit_code,
            duration_ms: elapsed.as_millis(),
            agent,
            agent_source,
            tty: std::io::stdout().is_terminal(),
        }
    }
}

/// Telemetry recorder. Constructed once per invocation; cheap when disabled.
pub struct Telemetry {
    enabled: bool,
    quiet: bool,
    log_dir: Option<PathBuf>,
}

impl Telemetry {
    /// Resolve enablement from env, config, and CLI quiet flag. Never fails;
    /// any error in discovery is treated as "leave telemetry on at defaults"
    /// so a missing config doesn't suppress collection.
    pub fn from_env(quiet: bool) -> Self {
        let enabled = telemetry_enabled();
        let log_dir = if enabled { state_dir() } else { None };
        Self {
            enabled,
            quiet,
            log_dir,
        }
    }

    /// Append `event` as a single JSON line to the telemetry log. Silently
    /// drops the event on any I/O or serialization error so telemetry never
    /// affects the user's command outcome.
    pub fn record(&self, event: &Event) {
        if !self.enabled {
            return;
        }
        let Some(log_dir) = self.log_dir.as_ref() else {
            return;
        };
        let _ = write_event(log_dir, event, self.quiet);
    }
}

fn write_event(log_dir: &Path, event: &Event, quiet: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(log_dir)?;
    let log_path = log_dir.join(LOG_FILE);
    rotate_if_needed(&log_path)?;
    let mut serialized = serde_json::to_vec(event).map_err(std::io::Error::other)?;
    serialized.push(b'\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    file.write_all(&serialized)?;

    maybe_print_notice(log_dir, quiet);
    Ok(())
}

fn rotate_if_needed(log_path: &Path) -> std::io::Result<()> {
    let Ok(metadata) = std::fs::metadata(log_path) else {
        return Ok(());
    };
    if metadata.len() < ROTATE_BYTES {
        return Ok(());
    }
    let rotated = log_path.with_extension("jsonl.1");
    std::fs::rename(log_path, rotated)?;
    File::create(log_path)?;
    Ok(())
}

fn maybe_print_notice(log_dir: &Path, quiet: bool) {
    let sentinel = log_dir.join(NOTICE_SENTINEL);
    if sentinel.exists() || quiet {
        return;
    }
    eprintln!(
        "arkouda: local telemetry enabled (events at {}). \
         Disable with ARKOUDA_TELEMETRY=0 or `telemetry = false` in .arkoudarc.toml. \
         See docs/adr/telemetry-for-agent-command-invocations.md.",
        log_dir.join(LOG_FILE).display()
    );
    let _ = File::create(sentinel);
}

/// Effective telemetry-enabled state from env then config. Defaults to `true`.
fn telemetry_enabled() -> bool {
    if let Some(decision) = env_opt_out() {
        return decision;
    }
    config_telemetry().unwrap_or(true)
}

fn env_opt_out() -> Option<bool> {
    let raw = std::env::var(ENV_OPT_OUT).ok()?;
    parse_bool(&raw)
}

/// Parse the env-var override. Returns `Some(true)`/`Some(false)` for
/// recognised values, `None` for empty or unrecognised input (so we fall
/// through to the config).
pub(crate) fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "0" | "false" | "off" | "no" => Some(false),
        "1" | "true" | "on" | "yes" => Some(true),
        _ => None,
    }
}

fn config_telemetry() -> Option<bool> {
    let cwd = std::env::current_dir().ok()?;
    crate::config::telemetry_from_config(&cwd).ok().flatten()
}

fn detect_agent() -> (Option<&'static str>, Option<String>) {
    for (var, id) in AGENT_ENV {
        if std::env::var(var).is_ok() {
            return (Some(*id), Some(format!("env:{var}")));
        }
    }
    (None, None)
}

/// Resolve the directory under which to write `telemetry.jsonl`. Returns
/// `None` if no plausible location can be derived (in which case telemetry
/// is silently dropped).
fn state_dir() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("ARKOUDA_STATE_DIR")
        && !custom.is_empty()
    {
        return Some(PathBuf::from(custom).join(APP_DIR));
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_DIR),
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(xdg) = std::env::var("XDG_STATE_HOME")
            && !xdg.is_empty()
        {
            return Some(PathBuf::from(xdg).join(APP_DIR));
        }
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".local")
                .join("state")
                .join(APP_DIR),
        )
    }
}

/// Redact argv tokens so values that look like paths or free-text titles
/// become opaque markers. Flag names and short slug/enum values pass
/// through unchanged.
pub(crate) fn redact_args(raw_args: &[String], command_name: Option<&str>) -> Vec<String> {
    let mut out = Vec::with_capacity(raw_args.len());
    let mut subcmd_skipped = command_name.is_none();
    for token in raw_args {
        if !subcmd_skipped && Some(token.as_str()) == command_name {
            subcmd_skipped = true;
            continue;
        }
        out.push(redact_token(token));
    }
    out
}

pub(crate) fn redact_token(token: &str) -> String {
    if token.starts_with('-') {
        return token.to_owned();
    }
    if token.chars().any(char::is_whitespace) {
        return "<title>".to_owned();
    }
    if token.contains('/') || token.contains('\\') || token.ends_with(".md") {
        return "<path>".to_owned();
    }
    token.to_owned()
}

impl Command {
    /// Stable short id for the subcommand, used in telemetry events.
    pub fn name(&self) -> &'static str {
        match self {
            Self::List(_) => "list",
            Self::Decision(_) => "decision",
            Self::Check => "check",
            Self::New(_) => "new",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_paths_and_titles_keeps_flags_and_slugs() {
        let argv = vec![
            "decision".to_string(),
            "use-postgres".to_string(),
            "--section".to_string(),
            "consequences".to_string(),
        ];
        let args = redact_args(&argv, Some("decision"));
        assert_eq!(
            args,
            vec!["use-postgres", "--section", "consequences"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn redacts_title_with_whitespace() {
        assert_eq!(redact_token("Use Postgres"), "<title>");
    }

    #[test]
    fn redacts_path_like_value() {
        assert_eq!(redact_token("/tmp/adr"), "<path>");
        assert_eq!(redact_token("docs/adr/use-postgres.md"), "<path>");
        assert_eq!(redact_token("relative/path"), "<path>");
    }

    #[test]
    fn keeps_short_slugs_and_enum_values() {
        assert_eq!(redact_token("use-postgres"), "use-postgres");
        assert_eq!(redact_token("accepted"), "accepted");
        assert_eq!(redact_token("id"), "id");
    }

    #[test]
    fn keeps_flag_tokens_even_with_path_chars() {
        // Flag-style tokens never get redacted, even if they syntactically
        // look pathy (e.g. `--dir=docs/adr`).
        assert_eq!(redact_token("--section"), "--section");
        assert_eq!(redact_token("--dir=docs/adr"), "--dir=docs/adr");
        assert_eq!(redact_token("-l"), "-l");
    }

    #[test]
    fn skips_first_subcommand_occurrence_only() {
        // If a value coincidentally matches the subcommand name, only the
        // first occurrence is stripped (the parsed subcommand token itself).
        let argv = vec!["list".to_string(), "--sort".to_string(), "id".to_string()];
        assert_eq!(redact_args(&argv, Some("list")), vec!["--sort", "id"]);
    }

    #[test]
    fn parse_bool_recognises_common_forms() {
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("OFF"), Some(false));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("On"), Some(true));
        assert_eq!(parse_bool("yes"), Some(true));
        assert_eq!(parse_bool(""), None);
        assert_eq!(parse_bool("maybe"), None);
    }

    #[test]
    fn event_serializes_to_expected_shape() {
        let event = Event {
            ts: "2026-05-20T14:32:11Z".to_owned(),
            version: "0.0.0",
            command: Some("decision"),
            args: vec!["use-postgres".to_owned(), "--section".to_owned()],
            exit_code: 0,
            duration_ms: 42,
            agent: Some("claude-code"),
            agent_source: Some("env:CLAUDECODE".to_owned()),
            tty: false,
        };
        let json: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&event).unwrap()).unwrap();
        assert_eq!(json["command"], "decision");
        assert_eq!(json["exit_code"], 0);
        assert_eq!(json["duration_ms"], 42);
        assert_eq!(json["agent"], "claude-code");
        assert_eq!(json["agent_source"], "env:CLAUDECODE");
        assert_eq!(json["tty"], false);
        assert_eq!(json["args"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn rotation_renames_when_log_is_oversized() {
        let dir = tempdir();
        let log = dir.join(LOG_FILE);
        std::fs::write(&log, vec![0u8; (ROTATE_BYTES + 1) as usize]).unwrap();
        rotate_if_needed(&log).unwrap();
        assert!(dir.join("telemetry.jsonl.1").exists());
        assert!(log.exists());
        assert_eq!(std::fs::metadata(&log).unwrap().len(), 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rotation_is_noop_when_under_limit() {
        let dir = tempdir();
        let log = dir.join(LOG_FILE);
        std::fs::write(&log, b"small\n").unwrap();
        rotate_if_needed(&log).unwrap();
        assert!(!dir.join("telemetry.jsonl.1").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_event_appends_jsonl_and_drops_sentinel() {
        let dir = tempdir();
        let event = Event {
            ts: "2026-05-20T14:32:11Z".to_owned(),
            version: "0.0.0",
            command: Some("list"),
            args: vec!["--sort".to_owned(), "id".to_owned()],
            exit_code: 0,
            duration_ms: 12,
            agent: None,
            agent_source: None,
            tty: false,
        };

        // Quiet first call: writes event, suppresses notice and sentinel.
        write_event(&dir, &event, true).unwrap();
        assert!(!dir.join(NOTICE_SENTINEL).exists());
        // Non-quiet second call: writes event and drops sentinel.
        write_event(&dir, &event, false).unwrap();
        assert!(dir.join(NOTICE_SENTINEL).exists());

        let body = std::fs::read_to_string(dir.join(LOG_FILE)).unwrap();
        let lines: Vec<_> = body.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(parsed["command"], "list");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "arkouda-telemetry-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }
}
