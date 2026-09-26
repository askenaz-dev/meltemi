// SPDX-License-Identifier: Apache-2.0

//! Identity read from the user's own mesh (identidad-propia).
//!
//! Meltemi issues no identity and holds no accounts. What it does is read the
//! official client of the mesh the user already runs — the way the daemon reads
//! `git` — and compose the gesture that links a machine to it. The client's
//! tokens are never read, stored or forwarded (§2).
//!
//! Two things live here, and only the second one exists yet:
//!
//! - **The linking gesture**, composed from the user's configuration and handed
//!   back as text. The daemon never runs it, because a daemon that executed a
//!   login would be a daemon holding somebody's session (design D3).
//! - **Reading status** from the client, which is deliberately absent: its task
//!   begins by capturing a real redacted `status --json` as a fixture, because
//!   the shape of that JSON is not verifiable from this repository and a
//!   mis-named field would give a green parser in CI and a refusal against the
//!   real client.

use meltemi_proto::{IdentityLinkParams, IdentityLinkResult, IdentityUnavailable};

use crate::config::{Config, IdentityConfig};

/// The mesh client's own name, when the configuration did not name a binary.
fn client_binary(identity: &IdentityConfig) -> String {
    identity
        .command
        .clone()
        .unwrap_or_else(|| identity.mesh.clone())
}

/// Composes the gesture that links THIS machine to the user's mesh.
///
/// Pure composition: nothing is launched, nothing is read from the client, and
/// no secret is put in. A pre-authorisation key would be a secret, so none is
/// composed — the user signs in through their own browser, which is the whole
/// point of handing back a gesture instead of running one.
///
/// # Errors
///
/// Refuses when identity is not configured, which is the same refusal
/// `identity/status` gives and for the same reason: there is nothing to link to.
pub fn link(
    config: &Config,
    params: &IdentityLinkParams,
    this_machine: &str,
) -> Result<IdentityLinkResult, (IdentityUnavailable, String)> {
    let Some(identity) = config.identity.as_ref() else {
        return Err((
            IdentityUnavailable::NotConfigured,
            "identity is off: add an [identity] table with `mesh` to your user config.toml"
                .to_string(),
        ));
    };

    let binary = client_binary(identity);
    let hostname = params
        .hostname
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(this_machine);

    let mut parts = vec![binary, "up".to_string()];
    if let Some(server) = &identity.login_server {
        parts.push("--login-server".to_string());
        parts.push(server.clone());
    }
    parts.push("--hostname".to_string());
    parts.push(hostname.to_string());

    // The same words in both shells, quoted the way each one wants them. Not one
    // string shown twice: a user pasting a POSIX line into PowerShell is exactly
    // how a hostname with a space becomes two arguments.
    let posix = parts
        .iter()
        .map(|part| quote_posix(part))
        .collect::<Vec<_>>()
        .join(" ");
    let powershell = parts
        .iter()
        .map(|part| quote_powershell(part))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(IdentityLinkResult {
        // The plain reading, for a person looking at it rather than pasting it.
        command: parts.join(" "),
        posix,
        powershell,
        // Signing in opens a browser, and a surface says so before the user is
        // surprised by one. An enumerated fact, not prose: the contract's strings
        // are English and the localisation lives in the surfaces (§11).
        opens_browser: true,
        login_server: identity.login_server.clone(),
    })
}

/// Quotes one argument for a POSIX shell, and only when it needs it.
fn quote_posix(part: &str) -> String {
    if !part.is_empty() && part.chars().all(safe_bare) {
        return part.to_string();
    }
    format!("'{}'", part.replace('\'', r"'\''"))
}

/// Quotes one argument for PowerShell, and only when it needs it.
fn quote_powershell(part: &str) -> String {
    if !part.is_empty() && part.chars().all(safe_bare) {
        return part.to_string();
    }
    format!("'{}'", part.replace('\'', "''"))
}

/// Whether a character can stand unquoted in either shell.
///
/// Deliberately narrow. The cost of quoting something that did not need it is a
/// pair of quotes; the cost of not quoting something that did is a command that
/// does something else.
fn safe_bare(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured(login_server: Option<&str>, command: Option<&str>) -> Config {
        Config {
            identity: Some(IdentityConfig {
                mesh: "tailscale".into(),
                login_server: login_server.map(str::to_string),
                command: command.map(str::to_string),
            }),
            ..Config::default()
        }
    }

    // Scenario: El gesto de vínculo se imprime, no se ejecuta
    #[test]
    fn the_gesture_names_the_mesh_the_server_and_this_machine() {
        let config = configured(Some("https://headscale.askenaz.dev"), None);
        let gesture = link(&config, &IdentityLinkParams::default(), "mi-pc").expect("configured");

        assert_eq!(
            gesture.command,
            "tailscale up --login-server https://headscale.askenaz.dev --hostname mi-pc"
        );
        assert_eq!(gesture.posix, gesture.command, "nothing needed quoting");
        assert_eq!(gesture.powershell, gesture.command);
        assert!(
            gesture.opens_browser,
            "a surface warns before a browser appears"
        );
        assert_eq!(
            gesture.login_server.as_deref(),
            Some("https://headscale.askenaz.dev")
        );
    }

    #[test]
    fn without_a_login_server_the_gesture_is_the_meshs_own_default() {
        // Somebody on the mesh's hosted service names no server, and the gesture
        // must not invent one.
        let config = configured(None, None);
        let gesture = link(&config, &IdentityLinkParams::default(), "mi-pc").expect("configured");
        assert_eq!(gesture.command, "tailscale up --hostname mi-pc");
        assert!(gesture.login_server.is_none());
    }

    #[test]
    fn a_named_hostname_wins_over_this_machines_name() {
        let config = configured(None, None);
        let asked = IdentityLinkParams {
            hostname: Some("el-portatil".into()),
        };
        let gesture = link(&config, &asked, "mi-pc").expect("configured");
        assert!(
            gesture.command.ends_with("--hostname el-portatil"),
            "{gesture:?}"
        );

        // And a blank one falls back rather than asking the mesh for an empty
        // name.
        let blank = IdentityLinkParams {
            hostname: Some("   ".into()),
        };
        let gesture = link(&config, &blank, "mi-pc").expect("configured");
        assert!(gesture.command.ends_with("--hostname mi-pc"), "{gesture:?}");
    }

    #[test]
    fn a_name_with_a_space_is_quoted_for_each_shell_and_not_shared() {
        // The failure this prevents: a POSIX line pasted into PowerShell where a
        // hostname with a space becomes two arguments.
        let config = configured(None, Some("C:/Program Files/Tailscale/tailscale.exe"));
        let asked = IdentityLinkParams {
            hostname: Some("el portatil".into()),
        };
        let gesture = link(&config, &asked, "mi-pc").expect("configured");

        assert!(
            gesture
                .posix
                .contains("'C:/Program Files/Tailscale/tailscale.exe'"),
            "{}",
            gesture.posix
        );
        assert!(
            gesture.posix.ends_with("'el portatil'"),
            "{}",
            gesture.posix
        );
        assert!(
            gesture.powershell.ends_with("'el portatil'"),
            "{}",
            gesture.powershell
        );
        // A quote inside a name is escaped the way each shell wants, which is not
        // the same way.
        let tricky = IdentityLinkParams {
            hostname: Some("it's-mine".into()),
        };
        let gesture = link(&config, &tricky, "mi-pc").expect("configured");
        assert!(
            gesture.posix.contains(r"'it'\''s-mine'"),
            "{}",
            gesture.posix
        );
        assert!(
            gesture.powershell.contains("'it''s-mine'"),
            "{}",
            gesture.powershell
        );
    }

    // Scenario: El gesto no lleva secretos
    #[test]
    fn the_gesture_carries_no_secret_of_any_kind() {
        // A pre-authorisation key would be a secret, so none is composed: the
        // user signs in through their own browser, which is the whole reason a
        // gesture is handed back instead of being run.
        let config = configured(Some("https://headscale.askenaz.dev"), None);
        let gesture = link(&config, &IdentityLinkParams::default(), "mi-pc").expect("configured");
        let whole = format!(
            "{} {} {}",
            gesture.command, gesture.posix, gesture.powershell
        );
        for secret in ["--authkey", "--auth-key", "tskey-", "token", "password"] {
            assert!(
                !whole.to_lowercase().contains(secret),
                "`{secret}` must never be composed into the gesture: {whole}"
            );
        }
    }

    #[test]
    fn without_identity_configured_there_is_nothing_to_link_to() {
        let config = Config::default();
        let (reason, remedy) =
            link(&config, &IdentityLinkParams::default(), "mi-pc").expect_err("off");
        assert_eq!(reason, IdentityUnavailable::NotConfigured);
        assert!(remedy.contains("[identity]"), "{remedy}");
        assert!(remedy.contains("user config.toml"), "{remedy}");
    }
}
