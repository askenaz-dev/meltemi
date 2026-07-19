// SPDX-License-Identifier: Apache-2.0

//! Fleet catalog (catalogo-flota): the bundled registry snapshot crossed
//! with passive local detection, plus the user's custom agents.
//!
//! - The registry is a versioned TOML snapshot embedded at build time (D1);
//!   populating the catalog never touches the network. A local file given
//!   via `MELTEMI_FLEET_REGISTRY` or the `fleet.registry` config key
//!   substitutes it (the test and power-user lever).
//! - Detection resolves each entry's binary on `PATH` plus its declared
//!   candidate paths (D2). On Windows the executable extensions
//!   `.exe`/`.cmd`/`.bat` are probed (npm shims). Detection never executes
//!   a binary nor launches any process; the agent's real version arrives
//!   later through the ACP handshake when it is actually used.
//! - `fleet/list` re-runs detection on every query, so the result reflects
//!   the present (D3).
//! - `agent.id` selects a catalog entry for sessions, below the environment
//!   override and the literal `agent.command` (D4).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use meltemi_proto::{FleetAgent, FleetAgentSource, FleetListParams, FleetListResult, error_codes};

use crate::config::Config;
use crate::rpc::RpcError;
use crate::server::DaemonState;

/// The registry snapshot bundled into the daemon (design D1), curated per
/// release from the public ACP registry plus the internal research.
pub const EMBEDDED_REGISTRY: &str = include_str!("../data/fleet-registry.toml");

/// A parsed, validated fleet registry.
#[derive(Debug, Clone)]
pub struct Registry {
    /// Snapshot version, reported verbatim by `fleet/list`.
    pub version: String,
    /// Entries in registry order.
    pub agents: Vec<RegistryAgent>,
}

/// One registry entry, with the binary already picked for the current OS.
#[derive(Debug, Clone)]
pub struct RegistryAgent {
    /// Stable catalog id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Declared integration level (1-4).
    pub level: u8,
    /// Binary name for this OS, if the entry declares one.
    pub bin: Option<String>,
    /// Well-known install locations probed besides `PATH`.
    pub candidate_paths: Vec<String>,
    /// Arguments that put the binary in ACP mode on stdio.
    pub acp_args: Vec<String>,
    /// Level 2: the ACP adapter binary that bridges this agent to ACP.
    pub adapter: Option<String>,
    /// Level 2: arguments for the adapter.
    pub adapter_args: Vec<String>,
    /// Level 3: the headless binary and its invocation arguments.
    pub headless: Option<String>,
    pub headless_args: Vec<String>,
    /// Level 3: native-control arguments Meltemi always adds (approval mode,
    /// agent sandbox), configured from data in one place (design D2).
    pub native_controls: Vec<String>,
    /// Level 4: the projected instruction file this agent reads (design D1).
    pub l4_target: Option<String>,
    /// Whether the agent supports MCP passthrough (declared in the registry).
    pub mcp: bool,
}

#[derive(Debug, Deserialize)]
struct RawRegistry {
    version: String,
    #[serde(default)]
    agents: Vec<RawRegistryAgent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RawRegistryAgent {
    id: String,
    name: String,
    level: u8,
    #[serde(default)]
    bin: Option<BinSpec>,
    #[serde(default)]
    acp_args: Vec<String>,
    #[serde(default)]
    candidate_paths: Vec<String>,
    #[serde(default)]
    adapter: Option<BinSpec>,
    #[serde(default)]
    adapter_args: Vec<String>,
    #[serde(default)]
    headless: Option<BinSpec>,
    #[serde(default)]
    headless_args: Vec<String>,
    #[serde(default)]
    native_controls: Vec<String>,
    #[serde(default)]
    l4_target: Option<String>,
    #[serde(default)]
    mcp: bool,
}

/// The binary name of an entry: one for every OS, or a per-OS table.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BinSpec {
    Same(String),
    PerOs {
        default: Option<String>,
        windows: Option<String>,
        macos: Option<String>,
        linux: Option<String>,
    },
}

impl BinSpec {
    /// The binary name for the current OS, falling back to `default`.
    fn for_current_os(&self) -> Option<String> {
        match self {
            BinSpec::Same(name) => Some(name.clone()),
            BinSpec::PerOs {
                default,
                windows,
                macos,
                linux,
            } => {
                let per_os = if cfg!(windows) {
                    windows
                } else if cfg!(target_os = "macos") {
                    macos
                } else {
                    linux
                };
                per_os.clone().or_else(|| default.clone())
            }
        }
    }
}

/// Parses and validates a registry document: non-empty version, non-empty
/// unique ids, levels within the declared 1-4 scale.
pub fn parse_registry(text: &str) -> Result<Registry, String> {
    let raw: RawRegistry = toml::from_str(text).map_err(|e| e.to_string())?;
    if raw.version.trim().is_empty() {
        return Err("registry version must not be empty".into());
    }
    let mut seen = std::collections::HashSet::new();
    let mut agents = Vec::with_capacity(raw.agents.len());
    for agent in raw.agents {
        if agent.id.trim().is_empty() || agent.name.trim().is_empty() {
            return Err("registry entry with an empty id or name".into());
        }
        if !(1..=4).contains(&agent.level) {
            return Err(format!(
                "registry entry `{}` declares level {} outside 1..=4",
                agent.id, agent.level
            ));
        }
        if !seen.insert(agent.id.clone()) {
            return Err(format!("duplicate registry id `{}`", agent.id));
        }
        agents.push(RegistryAgent {
            id: agent.id,
            name: agent.name,
            level: agent.level,
            bin: agent.bin.and_then(|b| b.for_current_os()),
            candidate_paths: agent.candidate_paths,
            acp_args: agent.acp_args,
            adapter: agent.adapter.and_then(|b| b.for_current_os()),
            adapter_args: agent.adapter_args,
            headless: agent.headless.and_then(|b| b.for_current_os()),
            headless_args: agent.headless_args,
            native_controls: agent.native_controls,
            l4_target: agent.l4_target,
            mcp: agent.mcp,
        });
    }
    Ok(Registry {
        version: raw.version,
        agents,
    })
}

/// Loads the registry: the override file when given and valid, the embedded
/// snapshot otherwise. An invalid override is ignored with a warning (the
/// same posture as an invalid config file); the reported version then
/// honestly names the embedded snapshot actually in use.
pub fn load_registry(override_path: Option<&Path>) -> Registry {
    if let Some(path) = override_path {
        match std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse_registry(&text))
        {
            Ok(registry) => return registry,
            Err(error) => tracing::warn!(
                path = %path.display(),
                error,
                "ignoring invalid fleet registry override; using the embedded snapshot"
            ),
        }
    }
    parse_registry(EMBEDDED_REGISTRY).expect("embedded fleet registry is valid (unit-tested)")
}

/// Executable extensions probed on Windows: the bounded PATHEXT of design D2
/// (installers ship `.exe`, npm shims are `.cmd`, some wrappers `.bat`).
#[cfg(windows)]
const WINDOWS_EXTS: &[&str] = &["exe", "cmd", "bat"];

/// Resolves a catalog binary: `PATH` lookup by name, then the entry's
/// candidate paths (`~/` expands to the home directory). Returns the
/// absolute path when present. Pure filesystem probing — no subprocess is
/// ever launched (design D2).
pub fn resolve_binary(
    bin: Option<&str>,
    candidates: &[String],
    path_var: &OsStr,
) -> Option<PathBuf> {
    if let Some(name) = bin {
        // A name carrying a path separator is a direct location.
        if name.contains('/') || name.contains('\\') {
            if let Some(hit) = probe(&expand_home(name)) {
                return Some(absolute(hit));
            }
        } else {
            for dir in std::env::split_paths(path_var) {
                if dir.as_os_str().is_empty() {
                    continue;
                }
                if let Some(hit) = probe(&dir.join(name)) {
                    return Some(absolute(hit));
                }
            }
        }
    }
    for candidate in candidates {
        if let Some(hit) = probe(&expand_home(candidate)) {
            return Some(absolute(hit));
        }
    }
    None
}

fn absolute(path: PathBuf) -> PathBuf {
    std::path::absolute(&path).unwrap_or(path)
}

/// Whether `path` names an executable file, probing the bounded Windows
/// extensions when it carries none. Returns the concrete file found.
#[cfg(windows)]
fn probe(path: &Path) -> Option<PathBuf> {
    let has_known_ext = path
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| WINDOWS_EXTS.iter().any(|k| ext.eq_ignore_ascii_case(k)));
    if has_known_ext {
        return is_file(path).then(|| path.to_path_buf());
    }
    let name = path.file_name()?.to_str()?.to_string();
    for ext in WINDOWS_EXTS {
        let with_ext = path.with_file_name(format!("{name}.{ext}"));
        if is_file(&with_ext) {
            return Some(with_ext);
        }
    }
    None
}

/// Whether `path` names an executable file (regular file with any execute
/// bit set).
#[cfg(unix)]
fn probe(path: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path).ok()?;
    (meta.is_file() && meta.permissions().mode() & 0o111 != 0).then(|| path.to_path_buf())
}

#[cfg(windows)]
fn is_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.is_file())
        .unwrap_or(false)
}

/// Expands a leading `~/` (or `~\`) to the user's home directory.
fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"))
        && let Some(base) = directories::BaseDirs::new()
    {
        return base.home_dir().join(rest);
    }
    PathBuf::from(path)
}

/// One catalog entry before detection.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    /// Stable catalog id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Where the entry comes from.
    pub source: FleetAgentSource,
    /// Declared integration level (1-4).
    pub level: u8,
    /// Binary name (or direct path) to detect.
    pub bin: Option<String>,
    /// Locations probed besides `PATH`.
    pub candidate_paths: Vec<String>,
    /// Arguments appended to the detected binary to speak ACP.
    pub acp_args: Vec<String>,
    /// Level 2: the ACP adapter binary and its arguments.
    pub adapter: Option<String>,
    pub adapter_args: Vec<String>,
    /// Level 3: the headless binary, its arguments, and the native controls.
    pub headless: Option<String>,
    pub headless_args: Vec<String>,
    pub native_controls: Vec<String>,
    /// Level 4: the projected instruction file this agent reads.
    pub l4_target: Option<String>,
    /// Whether the agent supports MCP passthrough.
    pub mcp: bool,
}

impl Default for CatalogEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            source: FleetAgentSource::Custom,
            level: 1,
            bin: None,
            candidate_paths: Vec::new(),
            acp_args: Vec::new(),
            adapter: None,
            adapter_args: Vec::new(),
            headless: None,
            headless_args: Vec::new(),
            native_controls: Vec::new(),
            l4_target: None,
            mcp: false,
        }
    }
}

/// The full catalog: registry entries plus the config's custom agents.
#[derive(Debug, Clone)]
pub struct Catalog {
    /// Version of the registry the catalog was built from.
    pub registry_version: String,
    /// Registry entries in order, custom entries appended.
    pub entries: Vec<CatalogEntry>,
}

/// Builds the catalog for a resolved config: the (override-aware) registry
/// in order, then the custom agents. A custom id colliding with an existing
/// entry replaces it — the user knows their machine better than a snapshot.
pub fn build_catalog(config: &Config) -> Catalog {
    let registry = load_registry(config.fleet_registry.as_deref());
    let mut entries: Vec<CatalogEntry> = registry
        .agents
        .into_iter()
        .map(|agent| CatalogEntry {
            id: agent.id,
            name: agent.name,
            source: FleetAgentSource::Registry,
            level: agent.level,
            bin: agent.bin,
            candidate_paths: agent.candidate_paths,
            acp_args: agent.acp_args,
            adapter: agent.adapter,
            adapter_args: agent.adapter_args,
            headless: agent.headless,
            headless_args: agent.headless_args,
            native_controls: agent.native_controls,
            l4_target: agent.l4_target,
            mcp: agent.mcp,
        })
        .collect();
    for custom in &config.fleet_custom {
        let entry = CatalogEntry {
            id: custom.id.clone(),
            name: custom.name.clone(),
            source: FleetAgentSource::Custom,
            // A custom entry is declared to speak ACP — that is what the
            // daemon pilots; verification is the conformance suite's job.
            level: 1,
            bin: custom.command.first().cloned(),
            acp_args: custom.command.iter().skip(1).cloned().collect(),
            ..CatalogEntry::default()
        };
        match entries.iter_mut().find(|e| e.id == entry.id) {
            Some(existing) => *existing = entry,
            None => entries.push(entry),
        }
    }
    Catalog {
        registry_version: registry.version,
        entries,
    }
}

/// Detects one entry: the absolute path of its binary when present.
pub fn detect(entry: &CatalogEntry, path_var: &OsStr) -> Option<PathBuf> {
    resolve_binary(entry.bin.as_deref(), &entry.candidate_paths, path_var)
}

/// Materializes the `fleet/list` result: fresh detection on every call (D3),
/// marking the entry `configured_id` selects, if any.
pub fn list(config: &Config, configured_id: Option<&str>, path_var: &OsStr) -> FleetListResult {
    let catalog = build_catalog(config);
    let mut agents: Vec<FleetAgent> = catalog
        .entries
        .iter()
        .map(|entry| {
            let binary = detect(entry, path_var);
            FleetAgent {
                id: entry.id.clone(),
                display_name: entry.name.clone(),
                source: entry.source,
                integration_level: entry.level,
                // Enriched from persisted conformance in `handle_fleet_list`.
                verified_level: None,
                verified_at: None,
                mcp_support: entry.mcp,
                detected: binary.is_some(),
                binary_path: binary.map(|p| p.display().to_string()),
                configured: configured_id == Some(entry.id.as_str()),
                underlying_agent: None,
            }
        })
        .collect();

    // Launch profiles: a catalog agent under a selected auth context. The row
    // detects the underlying binary and names the agent it launches (flota-
    // multiproveedor D4). Never the configured selection (profiles are chosen
    // per session, not project-wide).
    for profile in &config.fleet_profiles {
        let underlying = catalog.entries.iter().find(|e| e.id == profile.agent);
        let binary = underlying.and_then(|e| detect(e, path_var));
        agents.push(FleetAgent {
            id: profile.name.clone(),
            display_name: profile.name.clone(),
            source: FleetAgentSource::Profile,
            integration_level: underlying.map_or(1, |e| e.level),
            verified_level: None,
            verified_at: None,
            mcp_support: underlying.is_some_and(|e| e.mcp),
            detected: binary.is_some(),
            binary_path: binary.map(|p| p.display().to_string()),
            configured: false,
            underlying_agent: Some(profile.agent.clone()),
        });
    }

    FleetListResult {
        registry_version: catalog.registry_version,
        agents,
    }
}

/// Handles the `fleet/list` request: catalog plus fresh detection, marking
/// the configured agent when the request names a project root.
pub fn handle_fleet_list(params: Value, state: &Arc<DaemonState>) -> Result<Value, RpcError> {
    let params: FleetListParams = if params.is_null() {
        FleetListParams::default()
    } else {
        serde_json::from_value(params)
            .map_err(|e| RpcError::invalid_params(format!("fleet/list: {e}")))?
    };
    let project_root = params.project_root.map(PathBuf::from);
    let config = Config::load(&state.config_dir, project_root.as_deref());
    // `configured` is only meaningful relative to a given project (D3).
    let configured_id = if project_root.is_some() {
        config.agent_id.clone()
    } else {
        None
    };
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let mut result = list(&config, configured_id.as_deref(), &path_var);
    // Enrich with the verified level from the last persisted conformance run
    // (design D4): declared vs verified is visible in the surfaces.
    let verified = crate::conformance::latest_by_agent(&state.data_dir);
    for agent in &mut result.agents {
        if let Some(run) = verified.get(&agent.id) {
            agent.verified_level = Some(run.verified_level);
            agent.verified_at = Some(run.run_at.clone());
        }
    }
    Ok(serde_json::to_value(result).expect("FleetListResult serializes"))
}

/// Resolves the agent argv for a new session, honoring the precedence
/// env override > literal `command` > catalog `id` (design D4). The first
/// two are already folded into `config.agent_command` by [`Config::load`].
/// Resolution only detects — nothing is launched here; failures are 2000
/// (nothing configured) or 2001 (id unknown or binary not detected).
pub fn resolve_agent_command(config: &Config, path_var: &OsStr) -> Result<Vec<String>, RpcError> {
    if let Some(argv) = &config.agent_command {
        return Ok(argv.clone());
    }
    let Some(id) = &config.agent_id else {
        return Err(RpcError::application(
            error_codes::AGENT_COMMAND_NOT_CONFIGURED,
            "no agent configured",
            "agent_command_not_configured",
            "neither `agent.command` nor `agent.id` is configured",
            Some(
                "Set `agent.id` (see `meltemi fleet`) or `agent.command` in \
                 .meltemi/config.toml or the user config."
                    .into(),
            ),
        ));
    };
    let catalog = build_catalog(config);
    let Some(entry) = catalog.entries.iter().find(|e| e.id == *id) else {
        return Err(not_detected(format!(
            "agent id `{id}` is not in the fleet catalog (registry {})",
            catalog.registry_version
        )));
    };
    match detect(entry, path_var) {
        Some(binary) => {
            let mut argv = vec![binary.display().to_string()];
            argv.extend(entry.acp_args.iter().cloned());
            Ok(argv)
        }
        None => Err(not_detected(format!(
            "the binary of agent `{id}` ({}) was not detected on this system",
            entry.name
        ))),
    }
}

fn not_detected(detail: String) -> RpcError {
    RpcError::application(
        error_codes::AGENT_NOT_DETECTED,
        "agent not detected",
        "agent_not_detected",
        detail,
        Some(
            "Run `meltemi fleet` to see the detected agents, install the agent's \
             official CLI, or set `agent.command` explicitly."
                .into(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CustomAgent;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("meltemi-fleet-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Creates a fake executable following the platform's conventions and
    /// returns its concrete path; detection resolves the extension-less name.
    fn fake_binary(dir: &Path, name: &str) -> PathBuf {
        #[cfg(windows)]
        {
            let path = dir.join(format!("{name}.cmd"));
            std::fs::write(&path, "@echo off\r\n").unwrap();
            path
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join(name);
            std::fs::write(&path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }
    }

    #[test]
    fn embedded_snapshot_parses_and_is_valid() {
        let registry = parse_registry(EMBEDDED_REGISTRY).expect("embedded registry parses");
        assert!(!registry.version.is_empty());
        assert!(!registry.agents.is_empty());
        for agent in &registry.agents {
            assert!(
                (1..=4).contains(&agent.level),
                "`{}` level out of range",
                agent.id
            );
            assert!(
                agent.bin.is_some(),
                "`{}` declares no binary for this OS",
                agent.id
            );
        }
    }

    #[test]
    fn override_registry_reports_its_own_version() {
        // Scenario: Registro sustituido para pruebas.
        let dir = temp_dir("override");
        let path = dir.join("registry.toml");
        std::fs::write(
            &path,
            "version = \"fixture-9\"\n\n[[agents]]\nid = \"one\"\nname = \"One\"\nlevel = 1\nbin = \"one\"\n",
        )
        .unwrap();
        let registry = load_registry(Some(&path));
        assert_eq!(registry.version, "fixture-9");
        assert_eq!(registry.agents.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_override_falls_back_to_the_embedded_snapshot() {
        let dir = temp_dir("bad-override");
        let path = dir.join("registry.toml");
        std::fs::write(
            &path,
            "version = \"x\"\n[[agents]]\nid = \"a\"\nname = \"A\"\nlevel = 9\nbin = \"a\"\n",
        )
        .unwrap();
        let embedded = parse_registry(EMBEDDED_REGISTRY).unwrap();
        assert_eq!(load_registry(Some(&path)).version, embedded.version);
        // A missing file falls back too.
        assert_eq!(
            load_registry(Some(&dir.join("absent.toml"))).version,
            embedded.version
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn validation_rejects_bad_registries() {
        assert!(parse_registry("agents = []").is_err(), "missing version");
        assert!(parse_registry("version = \"\"").is_err(), "empty version");
        assert!(
            parse_registry("version=\"v\"\n[[agents]]\nid=\"a\"\nname=\"A\"\nlevel=5\n").is_err(),
            "level out of range"
        );
        assert!(
            parse_registry(
                "version=\"v\"\n[[agents]]\nid=\"a\"\nname=\"A\"\nlevel=1\n\n\
                 [[agents]]\nid=\"a\"\nname=\"B\"\nlevel=1\n"
            )
            .is_err(),
            "duplicate id"
        );
    }

    #[test]
    fn per_os_bin_tables_resolve_for_the_current_os() {
        let text = "version=\"v\"\n[[agents]]\nid=\"a\"\nname=\"A\"\nlevel=1\n\
                    bin={ default = \"gen\", windows = \"win\" }\n";
        let registry = parse_registry(text).unwrap();
        let expected = if cfg!(windows) { "win" } else { "gen" };
        assert_eq!(registry.agents[0].bin.as_deref(), Some(expected));
    }

    #[test]
    fn detection_resolves_present_binaries_in_a_path_fixture() {
        // Scenarios: Agente presente / Agente ausente sin error.
        let dir = temp_dir("detect");
        fake_binary(&dir, "present-agent");
        let path_var = std::env::join_paths([dir.clone()]).unwrap();

        let hit =
            resolve_binary(Some("present-agent"), &[], &path_var).expect("present binary found");
        assert!(hit.is_absolute(), "detection reports an absolute path");
        assert!(hit.starts_with(&dir));

        assert_eq!(resolve_binary(Some("absent-agent"), &[], &path_var), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(windows)]
    #[test]
    fn windows_detection_probes_only_the_bounded_extensions() {
        let dir = temp_dir("pathext");
        std::fs::write(dir.join("shim.cmd"), "@echo off\r\n").unwrap();
        std::fs::write(dir.join("script.ps1"), "").unwrap();
        let path_var = std::env::join_paths([dir.clone()]).unwrap();

        let hit = resolve_binary(Some("shim"), &[], &path_var).expect(".cmd shim detected");
        assert!(hit.extension().unwrap().eq_ignore_ascii_case("cmd"));
        // .ps1 lies outside the bounded set: not detectable.
        assert_eq!(resolve_binary(Some("script"), &[], &path_var), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn unix_detection_requires_the_executable_bit() {
        let dir = temp_dir("execbit");
        std::fs::write(dir.join("plain"), "not executable").unwrap();
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        assert_eq!(resolve_binary(Some("plain"), &[], &path_var), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn candidate_paths_detect_binaries_outside_path() {
        let dir = temp_dir("candidates");
        let bin = fake_binary(&dir, "tucked-away");
        // The candidate is declared extension-less; Windows probing adds it.
        let candidate = dir.join("tucked-away").display().to_string();
        let empty_path = std::ffi::OsString::new();
        let hit = resolve_binary(
            Some("tucked-away"),
            std::slice::from_ref(&candidate),
            &empty_path,
        )
        .expect("candidate detected");
        assert_eq!(hit, bin);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn custom_agents_join_the_catalog_and_detection() {
        // Scenario: Agente custom listado.
        let dir = temp_dir("custom");
        fake_binary(&dir, "my-agent");
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let config = Config {
            fleet_custom: vec![CustomAgent {
                id: "mine".into(),
                name: "My Agent".into(),
                command: vec!["my-agent".into(), "--acp".into()],
            }],
            ..Config::default()
        };
        let result = list(&config, None, &path_var);
        let mine = result
            .agents
            .iter()
            .find(|a| a.id == "mine")
            .expect("custom agent listed");
        assert_eq!(mine.source, FleetAgentSource::Custom);
        assert!(mine.detected);
        assert!(mine.binary_path.is_some());
        // Registry entries are still present alongside the custom one.
        assert!(result.agents.len() > 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn selection_precedence_command_beats_id() {
        // Scenario: Compatibilidad del comando literal (con o sin id).
        let config = Config {
            agent_command: Some(vec!["literal-agent".into(), "--x".into()]),
            agent_id: Some("gemini-cli".into()),
            ..Config::default()
        };
        let argv =
            resolve_agent_command(&config, OsStr::new("")).expect("the literal command wins");
        assert_eq!(argv, vec!["literal-agent".to_string(), "--x".to_string()]);
    }

    #[test]
    fn selection_by_detected_id_yields_binary_plus_acp_args() {
        // Scenario: Sesión por id detectado (resolution half).
        let dir = temp_dir("select");
        let bin = fake_binary(&dir, "reg-agent");
        let registry = dir.join("registry.toml");
        std::fs::write(
            &registry,
            "version=\"fixture-1\"\n[[agents]]\nid=\"reg\"\nname=\"Reg\"\nlevel=1\n\
             bin=\"reg-agent\"\nacp-args=[\"--acp\"]\n",
        )
        .unwrap();
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let config = Config {
            agent_id: Some("reg".into()),
            fleet_registry: Some(registry),
            ..Config::default()
        };
        let argv = resolve_agent_command(&config, &path_var).expect("detected id resolves");
        assert_eq!(argv, vec![bin.display().to_string(), "--acp".to_string()]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_or_undetected_id_is_error_2001() {
        // Scenario: Id no detectado.
        let dir = temp_dir("no-detect");
        let registry = dir.join("registry.toml");
        std::fs::write(
            &registry,
            "version=\"fixture-1\"\n[[agents]]\nid=\"ghost\"\nname=\"Ghost\"\nlevel=1\n\
             bin=\"ghost-agent-nope\"\n",
        )
        .unwrap();
        let base = Config {
            fleet_registry: Some(registry),
            ..Config::default()
        };

        let unknown = resolve_agent_command(
            &Config {
                agent_id: Some("nope".into()),
                ..base.clone()
            },
            OsStr::new(""),
        );
        assert_eq!(
            unknown.unwrap_err().code,
            error_codes::AGENT_NOT_DETECTED,
            "an id outside the catalog is 2001"
        );

        let undetected = resolve_agent_command(
            &Config {
                agent_id: Some("ghost".into()),
                ..base
            },
            OsStr::new(""),
        );
        assert_eq!(
            undetected.unwrap_err().code,
            error_codes::AGENT_NOT_DETECTED,
            "an undetected binary is 2001"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nothing_configured_is_error_2000() {
        let err = resolve_agent_command(&Config::default(), OsStr::new("")).unwrap_err();
        assert_eq!(err.code, error_codes::AGENT_COMMAND_NOT_CONFIGURED);
    }
}
