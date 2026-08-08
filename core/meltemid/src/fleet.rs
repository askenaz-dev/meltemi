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

use meltemi_proto::{
    FleetAgent, FleetAgentSource, FleetInstallState, FleetLayer, FleetLayerKind, FleetLayerSource,
    FleetLegalStatus, FleetListParams, FleetListResult, error_codes,
};

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
    /// The provider's official CLI, when the entry is piloted through an
    /// adapter (flota-deteccion-guia D1).
    pub cli_bin: Option<String>,
    pub cli_candidate_paths: Vec<String>,
    pub cli_install: Option<String>,
    pub adapter_install: Option<String>,
    /// Whether this entry's PILOT layer travels in Meltemi's own installers
    /// (adaptadores-propios-acp design D8). Generic: nothing keys on an id, and
    /// the provider's own CLI layer is never bundled — §2 forbids shipping
    /// somebody else's agent, so only a binary Meltemi builds can be.
    pub bundled: bool,
    pub legal_status: Option<String>,
    pub legal_note: Option<String>,
    /// The env variable that redirects the official binary's auth context,
    /// when the snapshot declares it (vincular-suscripciones D3).
    pub auth_context_var: Option<String>,
    /// The provider's documented login gesture, as snapshot data.
    pub login_hint: Option<String>,
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
    /// Two-layer detection (flota-deteccion-guia D1): the provider's own
    /// official CLI, probed beside the pilot point named by `bin`.
    #[serde(default)]
    cli_bin: Option<BinSpec>,
    #[serde(default)]
    cli_candidate_paths: Vec<String>,
    #[serde(default)]
    cli_install: Option<String>,
    #[serde(default)]
    adapter_install: Option<String>,
    /// The env variable that redirects the official binary's authentication
    /// context, and the provider's documented login gesture — subscription
    /// data, never provider logic (vincular-suscripciones D3).
    #[serde(default)]
    auth_context_var: Option<String>,
    #[serde(default)]
    login_hint: Option<String>,
    /// The entry's pilot layer travels in Meltemi's own installers (design D8).
    #[serde(default)]
    bundled: bool,
    #[serde(default)]
    legal_status: Option<String>,
    #[serde(default)]
    legal_note: Option<String>,
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
        // An entry piloted through an adapter must declare the provider's own
        // official CLI, or the fleet cannot say which layer is missing
        // (flota-deteccion-guia design D1).
        if agent.adapter.is_some() && agent.cli_bin.is_none() {
            return Err(format!(
                "registry entry `{}` declares an adapter without its `cli-bin` layer",
                agent.id
            ));
        }
        // A bundled layer travels in Meltemi's installers; a third-party
        // install command for it would be a contradiction the surfaces would
        // then have to choose between. Refuse the contradiction at the source —
        // in whichever field declares the command for THIS entry's pilot layer.
        // An entry with an adapter over the provider's CLI declares it in
        // `adapter-install`; an entry whose bundled binary is its only layer
        // declares it in `cli-install`, and that second shape is the one an
        // engine of our own will have.
        let pilot_install = if agent.cli_bin.is_some() {
            agent.adapter_install.as_ref()
        } else {
            agent.cli_install.as_ref()
        };
        if agent.bundled && pilot_install.is_some() {
            return Err(format!(
                "registry entry `{}` declares its pilot layer bundled and also a \
                 third-party install command for it",
                agent.id
            ));
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
            cli_bin: agent.cli_bin.and_then(|b| b.for_current_os()),
            cli_candidate_paths: agent.cli_candidate_paths,
            cli_install: agent.cli_install,
            adapter_install: agent.adapter_install,
            auth_context_var: agent.auth_context_var,
            login_hint: agent.login_hint,
            bundled: agent.bundled,
            legal_status: agent.legal_status,
            legal_note: agent.legal_note,
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
/// These are the only ones a launch may target.
#[cfg(windows)]
const WINDOWS_EXTS: &[&str] = &["exe", "cmd", "bat"];

/// Extensions that prove an installation without being launchable: the
/// PowerShell shims npm/nvm also drop (flota-deteccion-guia design D4).
/// `CreateProcess` cannot run a `.ps1`, so a hit here is reported as evidence
/// and never handed to a launch.
#[cfg(windows)]
const WINDOWS_EVIDENCE_EXTS: &[&str] = &["ps1"];

/// The directory the running executable lives in — where Meltemi's own
/// bundled binaries sit beside the daemon, because the installers put them
/// there together (adaptadores-propios-acp design D8). `None` when the platform
/// declines to answer, which reads as "not found" rather than as a guess.
///
/// It is a *probe location*, never a claim: a binary found here is reported
/// with its absolute path and its provenance like any other find.
#[must_use]
pub fn bundled_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.parent().map(Path::to_path_buf)
}

/// Resolves a catalog binary and says WHERE it came from. Precedence: the
/// user's `PATH` by name, the entry's declared candidate paths (`~/` expands to
/// the home directory), and finally — only when `bundled_dir` is given, which
/// the caller does only for a layer the registry declares bundled — the
/// directory beside the running daemon (design D8).
///
/// The order is the philosophy already in force everywhere else: what the user
/// installed outranks what Meltemi shipped. It is not a silent degradation
/// either way, because the find carries its provenance.
///
/// Pure filesystem probing — no subprocess is ever launched (design D2).
pub fn resolve_located(
    bin: Option<&str>,
    candidates: &[String],
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Option<(PathBuf, FleetLayerSource)> {
    if let Some(name) = bin {
        // A name carrying a path separator is a direct location.
        if name.contains('/') || name.contains('\\') {
            if let Some(hit) = probe(&expand_home(name)) {
                return Some((absolute(hit), FleetLayerSource::Path));
            }
        } else {
            for dir in std::env::split_paths(path_var) {
                if dir.as_os_str().is_empty() {
                    continue;
                }
                if let Some(hit) = probe(&dir.join(name)) {
                    return Some((absolute(hit), FleetLayerSource::Path));
                }
            }
        }
    }
    for candidate in candidates {
        if let Some(hit) = probe(&expand_home(candidate)) {
            return Some((absolute(hit), FleetLayerSource::CandidatePath));
        }
    }
    if let (Some(dir), Some(name)) = (bundled_dir, bin)
        && !name.contains('/')
        && !name.contains('\\')
        && let Some(hit) = probe(&dir.join(name))
    {
        return Some((absolute(hit), FleetLayerSource::Bundled));
    }
    None
}

/// [`resolve_located`] without the provenance, for the callers that only need
/// to know what a launch would execute.
pub fn resolve_binary(
    bin: Option<&str>,
    candidates: &[String],
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Option<PathBuf> {
    resolve_located(bin, candidates, path_var, bundled_dir).map(|(path, _)| path)
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

/// Probes the evidence-only extensions: a hit means "installed but not
/// launchable" (design D4). Windows only; every other platform has no such
/// split, so the function reports nothing.
#[cfg(windows)]
fn probe_evidence(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?.to_string();
    for ext in WINDOWS_EVIDENCE_EXTS {
        let with_ext = path.with_file_name(format!("{name}.{ext}"));
        if is_file(&with_ext) {
            return Some(with_ext);
        }
    }
    None
}

#[cfg(unix)]
fn probe_evidence(_path: &Path) -> Option<PathBuf> {
    None
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
    /// The provider's official CLI layer (flota-deteccion-guia D1).
    pub cli_bin: Option<String>,
    pub cli_candidate_paths: Vec<String>,
    pub cli_install: Option<String>,
    pub adapter_install: Option<String>,
    /// Whether the entry's pilot layer travels in Meltemi's own installers.
    pub bundled: bool,
    pub legal_status: Option<String>,
    pub legal_note: Option<String>,
    /// The env variable that redirects the official binary's auth context,
    /// when the snapshot declares it (vincular-suscripciones D3).
    pub auth_context_var: Option<String>,
    /// The provider's documented login gesture, as snapshot data.
    pub login_hint: Option<String>,
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
            cli_bin: None,
            cli_candidate_paths: Vec::new(),
            cli_install: None,
            adapter_install: None,
            auth_context_var: None,
            login_hint: None,
            bundled: false,
            legal_status: None,
            legal_note: None,
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
            cli_bin: agent.cli_bin,
            cli_candidate_paths: agent.cli_candidate_paths,
            cli_install: agent.cli_install,
            adapter_install: agent.adapter_install,
            auth_context_var: agent.auth_context_var,
            login_hint: agent.login_hint,
            bundled: agent.bundled,
            legal_status: agent.legal_status,
            legal_note: agent.legal_note,
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

/// Detects one entry: the absolute path of its binary when present. This is
/// the pilot point — what a launch would execute — and therefore what
/// `detected` reports (flota-deteccion-guia design D2). `bundled_dir` is the
/// directory beside the daemon, probed only when the entry declares its pilot
/// layer bundled (adaptadores-propios-acp design D8).
pub fn detect(
    entry: &CatalogEntry,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Option<PathBuf> {
    resolve_binary(
        entry.bin.as_deref(),
        &entry.candidate_paths,
        path_var,
        entry.bundled.then_some(bundled_dir).flatten(),
    )
}

/// Locates a layer as evidence of an installation: the launchable find first
/// (with its provenance), then the evidence-only shims. The `bool` says whether
/// the hit is launchable.
fn resolve_layer(
    bin: Option<&str>,
    candidates: &[String],
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Option<(PathBuf, bool, Option<FleetLayerSource>)> {
    if let Some((path, source)) = resolve_located(bin, candidates, path_var, bundled_dir) {
        return Some((path, true, Some(source)));
    }
    if let Some(name) = bin {
        if name.contains('/') || name.contains('\\') {
            if let Some(hit) = probe_evidence(&expand_home(name)) {
                return Some((absolute(hit), false, Some(FleetLayerSource::Path)));
            }
        } else {
            for dir in std::env::split_paths(path_var) {
                if dir.as_os_str().is_empty() {
                    continue;
                }
                if let Some(hit) = probe_evidence(&dir.join(name)) {
                    return Some((absolute(hit), false, Some(FleetLayerSource::Path)));
                }
            }
        }
    }
    for candidate in candidates {
        if let Some(hit) = probe_evidence(&expand_home(candidate)) {
            return Some((absolute(hit), false, Some(FleetLayerSource::CandidatePath)));
        }
    }
    if let (Some(dir), Some(name)) = (bundled_dir, bin)
        && !name.contains('/')
        && !name.contains('\\')
        && let Some(hit) = probe_evidence(&dir.join(name))
    {
        return Some((absolute(hit), false, Some(FleetLayerSource::Bundled)));
    }
    None
}

/// The layers of an entry, each detected on its own (design D1/D3): a level-2
/// entry has the official provider CLI plus the ACP adapter it is piloted
/// through; every other entry has the single `cli` layer its `bin` names.
///
/// The bundled probe reaches only the entry's PILOT layer, and only when the
/// registry declares it bundled: the provider's own CLI never travels with
/// Meltemi (§2), so probing beside the daemon for it would be probing for
/// something that cannot be there.
pub fn detect_layers(
    entry: &CatalogEntry,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Vec<FleetLayer> {
    let mut layers = Vec::new();

    if let Some(cli_bin) = &entry.cli_bin {
        let found = resolve_layer(Some(cli_bin), &entry.cli_candidate_paths, path_var, None);
        layers.push(FleetLayer {
            kind: FleetLayerKind::Cli,
            bin: cli_bin.clone(),
            detected: found.is_some(),
            binary_path: found
                .as_ref()
                .map(|(path, _, _)| path.display().to_string()),
            evidence_only: found.as_ref().is_some_and(|(_, launchable, _)| !launchable),
            install: entry.cli_install.clone(),
            bundled: false,
            source: found.as_ref().and_then(|(_, _, source)| *source),
        });
    }

    if let Some(bin) = &entry.bin {
        let found = resolve_layer(
            Some(bin),
            &entry.candidate_paths,
            path_var,
            entry.bundled.then_some(bundled_dir).flatten(),
        );
        // The pilot point is the adapter for a two-layer entry, the CLI itself
        // otherwise.
        let kind = if entry.cli_bin.is_some() {
            FleetLayerKind::Adapter
        } else {
            FleetLayerKind::Cli
        };
        layers.push(FleetLayer {
            kind,
            bin: bin.clone(),
            detected: found.is_some(),
            binary_path: found
                .as_ref()
                .map(|(path, _, _)| path.display().to_string()),
            evidence_only: found.as_ref().is_some_and(|(_, launchable, _)| !launchable),
            install: pilot_install(entry, kind),
            bundled: entry.bundled,
            source: found.as_ref().and_then(|(_, _, source)| *source),
        });
    }

    layers
}

/// The install command of an entry's PILOT layer — `None` when there is none
/// to give.
///
/// Which field declares it depends on the shape of the entry: `adapter-install`
/// when the pilot is an adapter over the provider's own CLI, `cli-install` when
/// the pilot binary IS the entry's only layer.
///
/// A bundled layer has none, whichever field was written: it travels in
/// Meltemi's installers, and the remedy for it is reinstalling Meltemi. The
/// rule is enforced here, on the layer, and not only where the sentence is
/// composed, because the remedy is read off the layer twice — as prose and as
/// a command a surface offers to copy — and an invariant kept in one of the two
/// is not an invariant (adaptadores-propios-acp design D8).
fn pilot_install(entry: &CatalogEntry, kind: FleetLayerKind) -> Option<String> {
    if entry.bundled {
        return None;
    }
    if kind == FleetLayerKind::Adapter {
        entry.adapter_install.clone()
    } else {
        entry.cli_install.clone()
    }
}

/// The layer whose absence an incomplete state is about — the one every
/// remedy, in every surface, is written for. Public so a launch refusal asks
/// the catalog's own question instead of re-deriving the answer and drifting
/// from it (flota-deteccion-guia design D5).
#[must_use]
pub fn missing_layer(layers: &[FleetLayer], launchable: bool) -> Option<&FleetLayer> {
    let two_layer = layers.len() > 1;
    let pilot = layers
        .iter()
        .find(|layer| layer.kind == FleetLayerKind::Adapter)
        .or_else(|| {
            layers
                .iter()
                .find(|layer| layer.kind == FleetLayerKind::Cli)
        });
    let cli = if two_layer {
        layers
            .iter()
            .find(|layer| layer.kind == FleetLayerKind::Cli)
    } else {
        None
    };
    match install_state(layers, launchable) {
        FleetInstallState::Ready => None,
        FleetInstallState::AdapterMissing => layers
            .iter()
            .find(|layer| layer.kind == FleetLayerKind::Adapter),
        FleetInstallState::CliMissing => cli,
        _ => layers
            .iter()
            .find(|layer| !layer.detected || layer.evidence_only)
            .or(pilot),
    }
}

/// The composed install state of an entry across its layers (design D2).
/// `launchable` is the pilot-point detection that `detected` reports.
#[must_use]
pub fn install_state(layers: &[FleetLayer], launchable: bool) -> FleetInstallState {
    let two_layer = layers.len() > 1;
    let pilot = layers
        .iter()
        .find(|layer| layer.kind == FleetLayerKind::Adapter)
        .or_else(|| {
            layers
                .iter()
                .find(|layer| layer.kind == FleetLayerKind::Cli)
        });
    let cli = if two_layer {
        layers
            .iter()
            .find(|layer| layer.kind == FleetLayerKind::Cli)
    } else {
        None
    };

    let pilot_found = pilot.is_some_and(|layer| layer.detected);
    let cli_found = cli.is_none_or(|layer| layer.detected);
    let evidence_only = layers.iter().any(|layer| layer.evidence_only);

    if launchable && cli_found {
        FleetInstallState::Ready
    } else if launchable && !cli_found {
        FleetInstallState::CliMissing
    } else if !pilot_found && cli.is_some_and(|layer| layer.detected) {
        FleetInstallState::AdapterMissing
    } else if evidence_only || pilot_found {
        // Something is installed, but no launchable target exists.
        FleetInstallState::NotLaunchable
    } else {
        FleetInstallState::NotDetected
    }
}

/// The one-sentence remedy for a layer that is missing or unlaunchable: what it
/// is, which binary it names, and what the user does about it. A layer that
/// travels in Meltemi's own installers is sent to reinstall or repair Meltemi
/// and is never handed somebody else's install command, because there is none
/// to hand (adaptadores-propios-acp design D8).
fn remedy_for(layer: &FleetLayer) -> String {
    let what = match layer.kind {
        FleetLayerKind::Cli => "the official provider CLI",
        FleetLayerKind::Adapter => "the ACP adapter",
    };
    let bin = &layer.bin;
    let trouble = if layer.evidence_only {
        "is only a script shim, which cannot be launched"
    } else if layer.bundled {
        "travels in Meltemi's own installers and is not beside the daemon"
    } else {
        "was not found on this system"
    };
    match offered_command(layer) {
        Some(command) => format!("{what} (`{bin}`) {trouble}: {command}"),
        None if layer.bundled => {
            format!("{what} (`{bin}`) {trouble}: reinstall or repair your Meltemi installation")
        }
        None => format!("{what} (`{bin}`) {trouble}"),
    }
}

/// The install command a surface may be offered for a layer — `None` for a
/// layer that travels with Meltemi, whatever its `install` field holds.
///
/// The invariant belongs to the layer and `pilot_install` is what keeps it
/// there, because `install` is a field of the contract and a surface may read
/// it straight off `fleet/list`. This is the same rule at the other end, read
/// once by both things composed from a layer — the remedy's sentence and the
/// command offered beside it — so that the two can never disagree about a
/// layer somebody built by hand.
fn offered_command(layer: &FleetLayer) -> Option<&str> {
    if layer.bundled {
        return None;
    }
    layer.install.as_deref()
}

/// Composes the honest install state of an entry from its layers, plus the
/// remedy for the layer that is missing (design D2/D5). `launchable` is the
/// pilot-point detection that `detected` reports.
pub fn compose_state(
    layers: &[FleetLayer],
    launchable: bool,
) -> (FleetInstallState, Option<String>, Option<String>) {
    let state = install_state(layers, launchable);
    let missing = missing_layer(layers, launchable);
    let remedy = missing.map(remedy_for);
    let remedy_command = missing.and_then(offered_command).map(str::to_string);
    (state, remedy, remedy_command)
}

/// Parses the registry's declared legal status.
fn legal_status_of(entry: &CatalogEntry) -> Option<FleetLegalStatus> {
    match entry.legal_status.as_deref() {
        Some("sanctioned") => Some(FleetLegalStatus::Sanctioned),
        Some("tolerated") => Some(FleetLegalStatus::Tolerated),
        Some("grey") => Some(FleetLegalStatus::Grey),
        _ => None,
    }
}

/// Materializes the `fleet/list` result: fresh detection on every call (D3),
/// marking the entry `configured_id` selects, if any.
pub fn list(
    config: &Config,
    configured_id: Option<&str>,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> FleetListResult {
    let catalog = build_catalog(config);
    let mut agents: Vec<FleetAgent> = catalog
        .entries
        .iter()
        .map(|entry| {
            let binary = detect(entry, path_var, bundled_dir);
            let layers = detect_layers(entry, path_var, bundled_dir);
            let (install_state, remedy, remedy_command) = compose_state(&layers, binary.is_some());
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
                layers,
                install_state: Some(install_state),
                remedy,
                remedy_command,
                legal_status: legal_status_of(entry),
                legal_note: entry.legal_note.clone(),
                auth_context_var: entry.auth_context_var.clone(),
            }
        })
        .collect();

    // Launch profiles: a catalog agent under a selected auth context. The row
    // detects the underlying binary and names the agent it launches (flota-
    // multiproveedor D4). Never the configured selection (profiles are chosen
    // per session, not project-wide).
    for profile in &config.fleet_profiles {
        let underlying = catalog.entries.iter().find(|e| e.id == profile.agent);
        let binary = underlying.and_then(|e| detect(e, path_var, bundled_dir));
        let layers = underlying
            .map(|e| detect_layers(e, path_var, bundled_dir))
            .unwrap_or_default();
        let (install_state, remedy, remedy_command) = compose_state(&layers, binary.is_some());
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
            layers,
            install_state: Some(install_state),
            remedy,
            remedy_command,
            legal_status: underlying.and_then(legal_status_of),
            legal_note: underlying.and_then(|e| e.legal_note.clone()),
            // A profile row is not itself linkable: it IS a link (or a manual
            // twin of one), so it never advertises the variable.
            auth_context_var: None,
        });
    }

    FleetListResult {
        registry_version: catalog.registry_version,
        agents,
    }
}

/// The agents a refusal to resolve one can offer instead: the catalog entries
/// **detected on this system**, each with the fleet's own vocabulary
/// (lanzador-conversacional D7).
///
/// Detected only, and that is what makes an empty list meaningful: it says the
/// fleet was consulted on this machine and nothing was found, which is a
/// different answer from "this error has nothing to do with resolving an agent"
/// and must not read the same. The catalog itself is never empty — the registry
/// is embedded — so listing every entry would make the empty case unreachable
/// and turn the offer into a shopping list. What is NOT installed belongs in the
/// Fleet view, which shows it with its install remedy; what a refusal offers is
/// what the user could pick right now.
///
/// Composed by `detect_layers` + `compose_state`, the same path `fleet/list`
/// takes, so the error and that view cannot disagree about the same machine.
/// A candidate that is `ready` carries no remedy because none applies: picking
/// it IS the remedy. Ids, detection and remedies only — never an environment
/// value, a credential path or anything shaped like a secret (§2).
#[must_use]
pub fn candidates(
    catalog: &Catalog,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Vec<meltemi_proto::AgentCandidate> {
    catalog
        .entries
        .iter()
        .filter_map(|entry| {
            detect(entry, path_var, bundled_dir)?;
            let layers = detect_layers(entry, path_var, bundled_dir);
            let (install_state, remedy, remedy_command) = compose_state(&layers, true);
            Some(meltemi_proto::AgentCandidate {
                id: entry.id.clone(),
                detected: true,
                install_state,
                remedy,
                remedy_command,
            })
        })
        .collect()
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
    let bundled = bundled_dir();
    let mut result = list(
        &config,
        configured_id.as_deref(),
        &path_var,
        bundled.as_deref(),
    );
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
pub fn resolve_agent_command(
    config: &Config,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> Result<Vec<String>, RpcError> {
    if let Some(argv) = &config.agent_command {
        return Ok(argv.clone());
    }
    let catalog = build_catalog(config);
    let Some(id) = &config.agent_id else {
        return Err(RpcError::application_with_candidates(
            error_codes::AGENT_COMMAND_NOT_CONFIGURED,
            "no agent configured",
            "agent_command_not_configured",
            "neither `agent.command` nor `agent.id` is configured",
            Some(
                "Set `agent.id` (see `meltemi fleet`) or `agent.command` in \
                 .meltemi/config.toml or the user config."
                    .into(),
            ),
            candidates(&catalog, path_var, bundled_dir),
        ));
    };
    let Some(entry) = catalog.entries.iter().find(|e| e.id == *id) else {
        return Err(not_detected(
            format!(
                "agent id `{id}` is not in the fleet catalog (registry {})",
                catalog.registry_version
            ),
            &catalog,
            path_var,
            bundled_dir,
        ));
    };
    match detect(entry, path_var, bundled_dir) {
        Some(binary) => {
            let mut argv = vec![binary.display().to_string()];
            argv.extend(entry.acp_args.iter().cloned());
            Ok(argv)
        }
        // The refusal names the missing LAYER and its install command, not just
        // "not detected". It is built by `levels::resolve_id_launch`, which every
        // production launch path goes through; this entry point keeps the same
        // diagnosis by delegating to it (flota-deteccion-guia design D5).
        None => Err(
            crate::levels::resolve_id_launch(&catalog, id, path_var, bundled_dir)
                .err()
                .unwrap_or_else(|| {
                    not_detected(
                        format!(
                            "the binary of agent `{id}` ({}) was not detected on this system",
                            entry.name
                        ),
                        &catalog,
                        path_var,
                        bundled_dir,
                    )
                }),
        ),
    }
}

fn not_detected(
    detail: String,
    catalog: &Catalog,
    path_var: &OsStr,
    bundled_dir: Option<&Path>,
) -> RpcError {
    RpcError::application_with_candidates(
        error_codes::AGENT_NOT_DETECTED,
        "agent not detected",
        "agent_not_detected",
        detail,
        Some(
            "Run `meltemi fleet` to see the detected agents, install the agent's \
             official CLI, or set `agent.command` explicitly."
                .into(),
        ),
        candidates(catalog, path_var, bundled_dir),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Windows shims: `.cmd` is a launch target, `.ps1` is evidence only
    /// (flota-deteccion-guia design D4).
    #[cfg(windows)]
    #[test]
    fn windows_script_shims_are_evidence_not_launch_targets() {
        let dir = std::env::temp_dir().join(format!("mel-shims-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path_var = std::ffi::OsString::from(dir.display().to_string());

        // A .ps1-only install: evidence, but nothing to launch.
        std::fs::write(dir.join("only-shim.ps1"), b"# shim\n").unwrap();
        let entry = CatalogEntry {
            id: "only-shim".into(),
            bin: Some("only-shim".into()),
            ..CatalogEntry::default()
        };
        assert!(
            detect(&entry, &path_var, None).is_none(),
            "a .ps1 is never returned as a launch target"
        );
        let layers = detect_layers(&entry, &path_var, None);
        assert_eq!(layers.len(), 1);
        assert!(layers[0].detected, "the shim proves an installation");
        assert!(layers[0].evidence_only);
        let (state, remedy, _) = compose_state(&layers, false);
        assert_eq!(state, FleetInstallState::NotLaunchable);
        assert!(remedy.expect("remedy").contains("shim"));

        // The same name with a .cmd beside it: launchable, no longer evidence.
        std::fs::write(dir.join("both.ps1"), b"# shim\n").unwrap();
        std::fs::write(dir.join("both.cmd"), b"@echo off\n").unwrap();
        let entry = CatalogEntry {
            id: "both".into(),
            bin: Some("both".into()),
            ..CatalogEntry::default()
        };
        let found = detect(&entry, &path_var, None).expect("the .cmd is launchable");
        assert!(found.to_string_lossy().ends_with("both.cmd"));
        let layers = detect_layers(&entry, &path_var, None);
        assert!(layers[0].detected && !layers[0].evidence_only);
        assert_eq!(compose_state(&layers, true).0, FleetInstallState::Ready);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_adapter_entry_without_its_cli_layer_is_refused_by_the_parser() {
        let bad = r#"
version = "test"

[[agents]]
id = "broken"
name = "Broken"
level = 2
bin = "broken-acp"
adapter = "broken-acp"
"#;
        let error = parse_registry(bad).expect_err("an adapter needs its cli-bin");
        assert!(error.contains("cli-bin"), "{error}");
    }

    #[test]
    fn a_bundled_layer_may_not_also_declare_a_third_party_install() {
        // The two declarations contradict each other: a layer that travels in
        // Meltemi's installers has no third-party command, and a surface should
        // never have to choose which of the two to believe.
        let bad = r#"
version = "test"

[[agents]]
id = "contradiction"
name = "Contradiction"
level = 2
bin = "meltemi-x-acp"
adapter = "meltemi-x-acp"
bundled = true
adapter-install = "npm i -g somebody-elses-adapter"
cli-bin = "x"
"#;
        let error = parse_registry(bad).expect_err("bundled and an install command cannot coexist");
        assert!(error.contains("bundled"), "{error}");

        // The same contradiction in the other shape an entry can have: one
        // layer, bundled, and no provider CLI beneath it — which is the shape a
        // pilot binary of our own has, and where the command is declared in
        // `cli-install` instead. The guard used to read only the field the
        // two-layer shape uses, so this parsed and yielded a bundled layer with
        // somebody else's command attached to it.
        let bad = r#"
version = "test"

[[agents]]
id = "single-layer"
name = "Single layer"
level = 1
bin = "meltemi-own-engine"
bundled = true
cli-install = "npm i -g somebody-elses-engine"
"#;
        let error = parse_registry(bad).expect_err("nor may they coexist in a single-layer entry");
        assert!(error.contains("bundled"), "{error}");
    }

    #[test]
    fn bundledness_beats_an_install_command_whoever_built_the_layer() {
        // Scenario: Capa empaquetada ausente remite a la instalación de Meltemi
        //
        // The registry refuses to declare both; this is the same rule one layer
        // down, for a layer built by something other than `detect_layers` — a
        // test, a caller of tomorrow. Both halves are asserted because the
        // remedy is composed from the layer twice: the sentence a surface
        // prints and the command a surface offers to copy. An invariant kept in
        // one of the two is not an invariant.
        let bundled = FleetLayer {
            bundled: true,
            ..layer(FleetLayerKind::Cli, false, false)
        };
        let (state, remedy, command) = compose_state(&[bundled], false);
        assert_eq!(state, FleetInstallState::NotDetected);
        let remedy = remedy.expect("a missing layer always carries its remedy");
        assert!(
            remedy.contains("reinstall or repair"),
            "the remedy for a bundled layer is Meltemi's own: {remedy}"
        );
        assert!(
            !remedy.contains("install me"),
            "and never the command that sat beside it: {remedy}"
        );
        assert!(
            command.is_none(),
            "which is not offered as a command either: {command:?}"
        );
    }

    /// A layer helper for the composition tests.
    fn layer(kind: FleetLayerKind, detected: bool, evidence_only: bool) -> FleetLayer {
        FleetLayer {
            kind,
            bin: match kind {
                FleetLayerKind::Cli => "provider-cli".into(),
                FleetLayerKind::Adapter => "provider-acp".into(),
            },
            detected,
            binary_path: detected.then(|| "/somewhere/bin".to_string()),
            evidence_only,
            install: Some("install me".into()),
            bundled: false,
            source: detected.then_some(FleetLayerSource::Path),
        }
    }

    // Scenario: El CLI oficial instalado sin adaptador se reporta como tal
    #[test]
    // Scenario: CLI oficial presente sin adaptador
    fn an_installed_cli_without_its_adapter_states_which_layer_is_missing() {
        let layers = vec![
            layer(FleetLayerKind::Cli, true, false),
            layer(FleetLayerKind::Adapter, false, false),
        ];
        let (state, remedy, command) = compose_state(&layers, false);
        assert_eq!(state, FleetInstallState::AdapterMissing);
        let remedy = remedy.expect("a remedy names the missing layer");
        assert!(remedy.contains("adapter"), "{remedy}");
        assert!(remedy.contains("provider-acp"), "{remedy}");
        assert_eq!(command.as_deref(), Some("install me"));
    }

    #[test]
    // Scenario: Ambas capas presentes
    fn both_layers_present_is_ready_without_a_remedy() {
        let layers = vec![
            layer(FleetLayerKind::Cli, true, false),
            layer(FleetLayerKind::Adapter, true, false),
        ];
        let (state, remedy, command) = compose_state(&layers, true);
        assert_eq!(state, FleetInstallState::Ready);
        assert!(remedy.is_none() && command.is_none());
    }

    #[test]
    // Scenario: Adaptador presente sin CLI oficial
    fn an_adapter_without_the_official_cli_is_reported_too() {
        let layers = vec![
            layer(FleetLayerKind::Cli, false, false),
            layer(FleetLayerKind::Adapter, true, false),
        ];
        let (state, remedy, _) = compose_state(&layers, true);
        assert_eq!(state, FleetInstallState::CliMissing);
        assert!(remedy.expect("remedy").contains("provider-cli"));
    }

    // Scenario: Un shim de script cuenta como evidencia, no como lanzable
    #[test]
    // Scenario: Shim de script cuenta como evidencia sin ser objetivo de lanzamiento
    fn a_script_shim_is_installed_but_not_launchable() {
        let layers = vec![layer(FleetLayerKind::Cli, true, true)];
        let (state, remedy, _) = compose_state(&layers, false);
        assert_eq!(state, FleetInstallState::NotLaunchable);
        assert!(remedy.expect("remedy").contains("shim"));
    }

    #[test]
    // Scenario: Ninguna capa presente
    // Scenario: Agente ausente sin error
    fn nothing_installed_is_not_detected() {
        let layers = vec![layer(FleetLayerKind::Cli, false, false)];
        let (state, remedy, _) = compose_state(&layers, false);
        assert_eq!(state, FleetInstallState::NotDetected);
        assert!(
            remedy.is_some(),
            "even here the remedy says what to install"
        );
    }

    #[test]
    // Scenario: Entrada de una sola capa conserva su estado
    fn a_single_layer_entry_needs_only_its_own_binary() {
        let layers = vec![layer(FleetLayerKind::Cli, true, false)];
        let (state, _, _) = compose_state(&layers, true);
        assert_eq!(state, FleetInstallState::Ready);
    }

    // Scenario: El registro declara las dos capas
    #[test]
    // Scenario: Capas de detección reportadas por entrada
    fn the_snapshot_declares_two_layers_for_its_adapter_entries() {
        let registry = parse_registry(EMBEDDED_REGISTRY).expect("snapshot parses");
        let two_layer: Vec<_> = registry
            .agents
            .iter()
            .filter(|agent| agent.cli_bin.is_some())
            .collect();
        assert!(
            !two_layer.is_empty(),
            "the snapshot declares the official CLI of its level-2 entries"
        );
        for agent in two_layer {
            assert_eq!(agent.level, 2, "{} declares a CLI layer", agent.id);
            assert!(
                agent.cli_install.is_some(),
                "{} states how to install the provider's own CLI",
                agent.id
            );
            // The pilot layer either says how to install it or says it travels
            // with Meltemi. Never both, never neither.
            assert_ne!(
                agent.bundled,
                agent.adapter_install.is_some(),
                "{} must declare its pilot layer bundled OR give a command for it",
                agent.id
            );
            assert!(
                agent.legal_status.is_some() && agent.legal_note.is_some(),
                "{} states its legal posture",
                agent.id
            );
        }
    }

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
    fn the_snapshot_declares_the_context_variable_per_entry() {
        // Scenario: El registro declara la variable por entrada
        // Subscription data lives in the snapshot, per entry: the two own-adapter
        // entries declare their variable and gesture, and an entry that has
        // none declared exposes none — absence is data too.
        let registry = parse_registry(EMBEDDED_REGISTRY).expect("embedded registry parses");
        let by_id = |id: &str| registry.agents.iter().find(|a| a.id == id).unwrap();
        let claude = by_id("claude-code");
        assert_eq!(
            claude.auth_context_var.as_deref(),
            Some("CLAUDE_CONFIG_DIR")
        );
        assert!(claude.login_hint.is_some(), "the gesture is snapshot data");
        let codex = by_id("codex-cli");
        assert_eq!(codex.auth_context_var.as_deref(), Some("CODEX_HOME"));
        assert!(codex.login_hint.is_some());
        assert!(by_id("gemini-cli").auth_context_var.is_none());

        // And the catalog row exposes it as consultable data.
        let config = Config::default();
        let result = list(&config, None, OsStr::new(""), None);
        let row = result
            .agents
            .iter()
            .find(|a| a.id == "claude-code")
            .unwrap();
        assert_eq!(row.auth_context_var.as_deref(), Some("CLAUDE_CONFIG_DIR"));
    }

    #[test]
    fn a_substituted_snapshot_declares_its_own_variables() {
        // Scenario: Registro sustituido declara sus propias variables
        // The variable comes from whichever snapshot is loaded — never from
        // knowledge fixed in code.
        let dir = temp_dir("ctx-var-override");
        let path = dir.join("registry.toml");
        std::fs::write(
            &path,
            "version = \"fixture-ctx\"

[[agents]]
id = \"claude-code\"
name = \"Other\"
level = 1
bin = \"other\"
auth-context-var = \"OTHER_HOME\"
login-hint = \"other login\"
",
        )
        .unwrap();
        let registry = load_registry(Some(&path));
        assert_eq!(registry.version, "fixture-ctx");
        let entry = registry
            .agents
            .iter()
            .find(|a| a.id == "claude-code")
            .unwrap();
        assert_eq!(
            entry.auth_context_var.as_deref(),
            Some("OTHER_HOME"),
            "the substituted snapshot's variable wins: nothing is hardcoded"
        );
        std::fs::remove_dir_all(&dir).ok();
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
    // Scenario: Agente presente
    fn detection_resolves_present_binaries_in_a_path_fixture() {
        // Scenarios: Agente presente / Agente ausente sin error.
        let dir = temp_dir("detect");
        fake_binary(&dir, "present-agent");
        let path_var = std::env::join_paths([dir.clone()]).unwrap();

        let hit = resolve_binary(Some("present-agent"), &[], &path_var, None)
            .expect("present binary found");
        assert!(hit.is_absolute(), "detection reports an absolute path");
        assert!(hit.starts_with(&dir));

        assert_eq!(
            resolve_binary(Some("absent-agent"), &[], &path_var, None),
            None
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(windows)]
    #[test]
    fn windows_detection_probes_only_the_bounded_extensions() {
        let dir = temp_dir("pathext");
        std::fs::write(dir.join("shim.cmd"), "@echo off\r\n").unwrap();
        std::fs::write(dir.join("script.ps1"), "").unwrap();
        let path_var = std::env::join_paths([dir.clone()]).unwrap();

        let hit = resolve_binary(Some("shim"), &[], &path_var, None).expect(".cmd shim detected");
        assert!(hit.extension().unwrap().eq_ignore_ascii_case("cmd"));
        // .ps1 lies outside the bounded set: not detectable.
        assert_eq!(resolve_binary(Some("script"), &[], &path_var, None), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn unix_detection_requires_the_executable_bit() {
        let dir = temp_dir("execbit");
        std::fs::write(dir.join("plain"), "not executable").unwrap();
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        assert_eq!(resolve_binary(Some("plain"), &[], &path_var, None), None);
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
            None,
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
        let result = list(&config, None, &path_var, None);
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
    // Scenario: Catálogo con configurado marcado
    fn fleet_list_includes_the_profiles() {
        // Scenario: fleet/list incluye los perfiles
        use crate::config::FleetProfile;
        let dir = temp_dir("profiles-list");
        fake_binary(&dir, "reg-agent");
        let registry = dir.join("registry.toml");
        std::fs::write(
            &registry,
            "version=\"fixture-1\"\n[[agents]]\nid=\"reg\"\nname=\"Reg\"\nlevel=2\n\
             bin=\"reg-agent\"\nacp-args=[\"--acp\"]\n",
        )
        .unwrap();
        let path_var = std::env::join_paths([dir.clone()]).unwrap();
        let config = Config {
            fleet_registry: Some(registry),
            fleet_profiles: vec![FleetProfile {
                name: "work".into(),
                agent: "reg".into(),
                env: vec![("MELTEMI_MOCK_MARKER".into(), "work-ctx".into())],
            }],
            ..Config::default()
        };
        let result = list(&config, None, &path_var, None);
        let profile = result
            .agents
            .iter()
            .find(|a| a.id == "work")
            .expect("profile row listed");
        // The profile is its own row, names the binary it launches, and inherits
        // the underlying agent's detection + integration level (D4).
        assert_eq!(profile.source, FleetAgentSource::Profile);
        assert_eq!(profile.underlying_agent.as_deref(), Some("reg"));
        assert!(profile.detected, "underlying binary is detected");
        assert_eq!(profile.integration_level, 2);
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
            resolve_agent_command(&config, OsStr::new(""), None).expect("the literal command wins");
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
        let argv = resolve_agent_command(&config, &path_var, None).expect("detected id resolves");
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
            None,
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
            None,
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
        let err = resolve_agent_command(&Config::default(), OsStr::new(""), None).unwrap_err();
        assert_eq!(err.code, error_codes::AGENT_COMMAND_NOT_CONFIGURED);
    }
}
