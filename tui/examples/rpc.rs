// SPDX-License-Identifier: Apache-2.0

//! Development RPC caller: invokes one daemon method with JSON params.
//!
//! Internal tooling (an example, never installed): the scriptable surface is
//! `meltemi` itself. This exists for the method verbs that only have a home in
//! the interactive palettes today (`sdd/verify-mark`, `sdd/review-decide`), so
//! the project can dogfood its own cycle from a script.
//!
//! ```text
//! cargo run -q --example rpc -- sdd/verify-mark '{"projectRoot":"...", ...}'
//! cargo run -q --example rpc -- batch @calls.json
//! ```
//!
//! `batch` takes a JSON array of `{ "method": "...", "params": { ... } }` and
//! applies them over ONE connection, which is what makes a 60-call review or
//! verification pass practical from a script.

use meltemi_client::bootstrap;
use meltemi_client::paths;
use meltemi_client::rpc::Peer;
use meltemi_proto::{InitializeParams, PROTOCOL_VERSION, PeerInfo, methods};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(method), Some(params)) = (args.next(), args.next()) else {
        eprintln!("usage: rpc <method> '<json params>' | rpc <method> @params.json");
        std::process::exit(2);
    };
    // `@path` reads the params from a file: shells mangle nested JSON quoting.
    let params = match params.strip_prefix('@') {
        Some(path) => std::fs::read_to_string(path).expect("params file readable"),
        None => params,
    };
    let params: serde_json::Value = serde_json::from_str(&params).expect("params are valid JSON");

    let endpoint = paths::endpoint();
    let stream = bootstrap::connect_or_start(&endpoint)
        .await
        .expect("daemon reachable");
    let (peer, mut incoming) = Peer::start(stream);
    tokio::spawn(async move { while incoming.recv().await.is_some() {} });
    peer.request(
        methods::INITIALIZE,
        &InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            client: PeerInfo {
                name: "meltemi-dev-rpc".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        },
    )
    .await
    .expect("initialize");

    if method == "batch" {
        let calls: Vec<serde_json::Value> = params
            .as_array()
            .cloned()
            .expect("batch takes a JSON array of {method, params}");
        let total = calls.len();
        let mut failed = 0usize;
        for call in calls {
            let name = call["method"].as_str().unwrap_or_default().to_string();
            let call_params = call["params"].clone();
            match peer.request(&name, &call_params).await {
                Ok(_) => {}
                Err(error) => {
                    failed += 1;
                    eprintln!("{name}: {error}");
                    if let Some(data) = &error.data {
                        eprintln!("{}", serde_json::to_string_pretty(data).unwrap());
                    }
                }
            }
        }
        println!("batch: {} ok, {failed} failed", total - failed);
        peer.close();
        if failed > 0 {
            std::process::exit(1);
        }
        return;
    }

    match peer.request(&method, &params).await {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            if let Some(data) = &error.data {
                eprintln!("{}", serde_json::to_string_pretty(data).unwrap());
            }
            peer.close();
            std::process::exit(1);
        }
    }
    peer.close();
}
