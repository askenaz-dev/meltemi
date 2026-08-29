---
name: no-hardcoded-secrets
kind: rule
version: 0.2.0 # x-release-please-version
scope: ["**/*.{ts,tsx,js,jsx,go,py,rs,java,kt,swift,rb,yaml,yml,toml,ini,env,sh,ps1}"]
severity: error
agents_supported: [claude-code, codex, copilot, opencode]
description: "Prohibits committed source/config containing API keys, OAuth tokens, AWS/GCP/Azure credentials, JWT bearers, database URIs with embedded passwords, or PEM-encoded private keys. Use environment variables (read from Vault) or the project's secret manager instead."
tags: [security, secrets, devsecops, vault, supply-chain]
owner_team: appsec
---

# no-hardcoded-secrets

## Rule

Do **not** commit any of the following in source, configuration, or CI files matched by the `scope` glob:

- API keys / personal access tokens with a recognizable prefix:
  - `sk-...` (OpenAI / Anthropic-style), `ghp_...` / `gho_...` / `github_pat_...` (GitHub),
    `xox[baprs]-...` (Slack), `AIza...` (Google), `AKIA...` (AWS Access Key ID).
- AWS / GCP / Azure credentials (long-form):
  - AWS secret access keys (40-char base64-ish strings adjacent to an `AKIA...` ID).
  - GCP service-account JSON blobs (`"type":"service_account"` + `"private_key":"-----BEGIN PRIVATE KEY-----..."`).
  - Azure connection strings (`AccountKey=...;` segments).
- JWT bearer tokens: `eyJhbGciOi...` with three `.`-separated base64 segments.
- Database URIs embedding a password: `postgres://user:secret@host`, `mongodb+srv://user:pwd@…`, `redis://:pwd@host`.
- PEM blocks for private material: `-----BEGIN (RSA |EC |OPENSSH |PGP |DSA )?PRIVATE KEY-----`.
- TLS certificates' private keys (paired with `.crt`/`.pem` in the same commit).

The `scope` glob spans application source (TS/JS/Go/Py/Rs/Java/Kt/Swift/Rb), declarative config (YAML/TOML/INI), env files, and scripts (Sh/PS1). Binary files are out of scope; commit-time secret scanning at the org level catches those.

## Why

- **GitHub Push Protection lags real exfil.** Once a secret hits the remote, even a force-push and `git filter-repo` don't help: a determined scraper saw it. The only safe state is "never committed".
- **Rotation costs scale with surface.** A leaked AWS key triggers an IAM rotation, audit log review, possibly notifying customers. Multiply by every service that touched the same key. Prevention is two orders of magnitude cheaper than cleanup.
- **Compliance + insurance.** SOC 2 / ISO 27001 / cyber insurance underwriters specifically ask "do you commit secrets?" Catching this at the agent layer + CI layer is part of the answer.
- **Insider attack vector.** A read-only repo clone is a credential dump. The principle is: a successful repo clone should grant **zero** production access. Vault + short-lived tokens make that true; hardcoded secrets break the invariant.

## What to use instead

```ts
// ❌ Bad: literal key in source
const stripe = new Stripe("sk_live_51HabcdEFGHIJKLMNOP…");

// ❌ Bad: also literal — committed to git is committed, even in a string template
const apiKey = process.env.STRIPE_KEY || "sk_test_default_for_dev";

// ✅ Good: env-only, fail loud if missing
const apiKey = required("STRIPE_KEY");
const stripe = new Stripe(apiKey);

function required(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`missing required env var: ${name}`);
  return v;
}
```

```yaml
# ❌ Bad: in a Helm values.yaml or CI workflow
db:
  url: postgres://app:hunter2@db-prod.internal:5432/main

# ✅ Good: reference the secret manager
db:
  urlSecretRef: { name: app-db, key: connection-string }
```

```bash
# ❌ Bad: in a deploy script
export AWS_SECRET_ACCESS_KEY="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"

# ✅ Good: pulled from Vault at runtime
vault kv get -field=secret_access_key secret/aws/deploy-bot/$ENV
```

### Where the actual secrets live in forge

| Use case                            | Storage                                                                | How code reads it                  |
|-------------------------------------|------------------------------------------------------------------------|------------------------------------|
| Service-to-service credentials      | HashiCorp Vault (`secret/services/<svc>/...`)                          | Vault Agent injector → env var     |
| Cloud-provider IAM (AWS/GCP/Azure)  | IRSA / Workload Identity (no static key)                               | SDK picks up from instance metadata|
| Third-party API keys (Stripe etc.)  | Vault → mounted into the pod                                          | env var, never disk                |
| Local development                   | `.env.local` (gitignored) sourced from `vault kv get | jq -r ...`      | env var                            |
| CI secrets                          | GitHub Actions Encrypted Secrets / GitLab CI Variables (Vault-backed)  | env var inside the runner          |

If a secret you need isn't in Vault yet, the right move is to add it (talk to appsec) — **not** to commit it "just for now".

## When this rule does not apply

- **Public test fixtures** containing intentionally-fake credentials clearly labeled as such:
  - `AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE` (AWS's own documented example value)
  - `password: "test-password-do-not-use"`
  - Tokens with obvious test markers (`test_`, `mock_`, `fake_`, `example_`) AND outside production code paths.
- **Documentation files** showing the shape of a secret (e.g. README explaining a config schema). Prefer placeholders (`<your-api-key>`) over realistic-looking strings.
- **Encrypted-at-rest secrets** committed deliberately (SOPS, sealed-secrets, age-encrypted blobs): the ciphertext is safe to commit; the recipient keys live elsewhere.
- **`.git/`, `node_modules/`, `vendor/`** and other generated trees: the rule's enforcement targets *committed* human-authored files.

If a file legitimately must contain a token-shaped string (rare — e.g. a security test asserting a parser handles a `eyJhbGciOi...` string correctly), suppress with a reason and an issue link:

```ts
// nosemgrep: secrets-detection -- test vector, see test_jwt_parser.spec.ts (issue #618)
const SAMPLE_JWT = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0In0.X8RpA...";
```

## How this rule is enforced

When `fdh install` materializes this rule into `.claude/rules/no-hardcoded-secrets.md` (and equivalents), the AI coding agent loads it and refuses to suggest or write strings matching the patterns above. Enforcement at the agent layer is **partially blocking** — agents trained on forge conventions should rewrite rather than insert.

The hard gates are all of the following, in defense-in-depth:

1. **Local pre-commit:** `gitleaks protect --staged` or `trufflehog filesystem --only-verified` runs on `git commit`. Blocks the commit before it hits the local repo.
2. **CI pipeline:** `gitleaks` runs on every PR; `secretlint` for richer pattern matching. Treats a single match as a hard fail (exit code → red check, blocks merge).
3. **Server-side:** GitHub Push Protection (for the org tier that supports it) catches known-pattern secrets even if all of the above are bypassed.
4. **Periodic:** `gitleaks detect` runs on a nightly cron against the full history, flagging historical leaks that survived earlier scans.

This rule complements the gates above by catching the problem during the agent's edit cycle (the cheapest possible moment to fix it). It also serves as a teaching surface — the AI explains *why* and *where to put the secret* rather than just refusing.

For the deeper context on what to scan for and how, the `devsecops` skill (in this hub) covers the broader supply-chain + runtime security model that this rule plugs into.
