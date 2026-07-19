// SPDX-License-Identifier: Apache-2.0

//! MCP passthrough (mcp-passthrough D2/D3): declared servers are injected into
//! the agent at session creation, but only when the agent announced MCP
//! support in its handshake. Sensitive values are `$VAR` references resolved
//! from the user's environment at injection time and **never logged** — the
//! audit records only server names.

use agent_client_protocol::schema::v1::{
    EnvVariable, McpCapabilities, McpServer, McpServerHttp, McpServerStdio,
};

use crate::config::{McpServerConfig, McpTransport};

/// Whether the agent announced MCP support (any transport). Without this, the
/// session opens but no servers are delivered — honest degradation.
pub fn announces_mcp(caps: &McpCapabilities) -> bool {
    caps.http || caps.sse
}

/// Converts a declared server to the ACP shape, resolving `$VAR` env
/// references from the user's environment (the resolved value never leaves
/// this process boundary in any log).
pub fn to_acp(server: &McpServerConfig) -> McpServer {
    match &server.transport {
        McpTransport::Stdio { command, args, env } => {
            let mut stdio = McpServerStdio::new(server.name.clone(), command.clone());
            stdio.args = args.clone();
            stdio.env = env
                .iter()
                .map(|(k, v)| EnvVariable::new(k.clone(), resolve_ref(v)))
                .collect();
            McpServer::Stdio(stdio)
        }
        McpTransport::Http { url } => {
            McpServer::Http(McpServerHttp::new(server.name.clone(), url.clone()))
        }
    }
}

/// Resolves a `$VAR` or `${VAR}` reference from the environment; a literal is
/// passed through (the hygiene lint already flagged literal secrets).
pub fn resolve_ref(value: &str) -> String {
    let name = value
        .strip_prefix("${")
        .and_then(|v| v.strip_suffix('}'))
        .or_else(|| value.strip_prefix('$'));
    match name {
        Some(var) => std::env::var(var).unwrap_or_default(),
        None => value.to_string(),
    }
}

/// The names of the declared servers (the only thing safe to log/surface).
pub fn names(servers: &[McpServerConfig]) -> Vec<String> {
    servers.iter().map(|s| s.name.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_references_resolve_without_leaking_into_names() {
        // SAFETY: single-threaded test.
        unsafe {
            std::env::set_var("MELTEMI_TEST_MCP_KEY", "s3cr3t");
        }
        let server = McpServerConfig {
            name: "fs".into(),
            transport: McpTransport::Stdio {
                command: "mcp-fs".into(),
                args: vec!["--root".into()],
                env: vec![("API_KEY".into(), "$MELTEMI_TEST_MCP_KEY".into())],
            },
        };
        let acp = to_acp(&server);
        match acp {
            McpServer::Stdio(stdio) => {
                assert_eq!(stdio.name, "fs");
                assert_eq!(stdio.env[0].value, "s3cr3t", "the ref resolved");
            }
            _ => panic!("expected stdio"),
        }
        // The audit-safe names never carry the secret.
        assert_eq!(names(std::slice::from_ref(&server)), vec!["fs".to_string()]);
        unsafe {
            std::env::remove_var("MELTEMI_TEST_MCP_KEY");
        }
    }
}
