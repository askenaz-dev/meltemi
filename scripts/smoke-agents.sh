#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The integrated run of Meltemi against the agent CLIs installed on this
# machine (corrida-integrada) — the macOS/Linux twin of smoke-agents.ps1, with
# the same isolation, the same seven verbs and the same report.
#
# Manual and opt-in: every real agent it drives spends turns on the user's own
# subscription, so it never runs in CI and never runs by itself.
#
# Isolated: the daemon it talks to is its own — endpoint, data and
# configuration in a temporary directory (MELTEMI_ENDPOINT, MELTEMI_DATA_DIR,
# MELTEMI_CONFIG_DIR, MELTEMI_DAEMON_BIN, and MELTEMI_START_HERE to ask for a
# start at that endpoint). The user's daemon, data and configuration are never
# read or touched.
#
# Usage:
#   scripts/smoke-agents.sh [--mock] [--agent <id>]... [--report <path>] [--bin-dir <dir>]
#
# Needs bash (3.2 or later: the one macOS ships is enough), git and jq, and a
# workspace built with `cargo build --workspace`.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="$repo/target/debug"
report=""
mock=0
timeout_s=300
agents=()
while [ $# -gt 0 ]; do
  case "$1" in
    --mock) mock=1 ;;
    --agent) agents+=("$2"); shift ;;
    --report) report="$2"; shift ;;
    --bin-dir) bin_dir="$2"; shift ;;
    --timeout) timeout_s="$2"; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done
command -v jq >/dev/null || { echo "jq is required to read the product's JSON answers" >&2; exit 2; }
meltemi="$bin_dir/meltemi"
meltemid="$bin_dir/meltemid"
for needed in "$meltemi" "$meltemid"; do
  [ -x "$needed" ] || { echo "not found: $needed — run \`cargo build --workspace\` first" >&2; exit 2; }
done

# ---- the isolated environment ------------------------------------------------

stamp_day="$(date -u +%Y-%m-%d)"
stamp_time="$(date -u +'%Y-%m-%d %H:%M')"
base="$(mktemp -d "${TMPDIR:-/tmp}/meltemi-smoke-agents-XXXXXX")"
fixture="$base/fixture"
run_dir="$base/out"
mkdir -p "$fixture" "$run_dir" "$base/data" "$base/config"
export MELTEMI_ENDPOINT="$base/meltemid.sock"
export MELTEMI_DATA_DIR="$base/data"
export MELTEMI_CONFIG_DIR="$base/config"
export MELTEMI_DAEMON_BIN="$meltemid"
export MELTEMI_START_HERE=1
unset MELTEMI_AGENT_COMMAND || true

# ---- running one command -------------------------------------------------------
# Output goes to files, never to a pipe: a `meltemi` in a pipeline hands the
# daemon it starts the pipe's handle, and the pipeline never ends.

step=0
last_out=""; last_err=""; last_exit=0; last_seconds=0
run() {
  step=$((step + 1))
  last_out="$run_dir/$(printf '%03d' "$step").out"
  last_err="$run_dir/$(printf '%03d' "$step").err"
  local start end
  start="$(date +%s)"
  # `timeout` is GNU coreutils: present on Linux, absent on a stock macOS,
  # where a command simply runs to its end.
  local bound=()
  if command -v timeout >/dev/null; then bound=(timeout "$timeout_s"); fi
  set +e
  (cd "$fixture" && "${bound[@]+"${bound[@]}"}" "$meltemi" "$@" >"$last_out" 2>"$last_err")
  last_exit=$?
  set -e
  end="$(date +%s)"
  last_seconds=$((end - start))
}
json() { jq -r "$1 // empty" "$last_out" 2>/dev/null || true; }

# What the session's own record says: permissions asked and who decided them,
# empty turns, and the usage the agent reported over the protocol.
facts() {
  local id="$1"
  if [ -z "$id" ]; then echo "0 · 0 · 0 · 0|0|"; return; fi
  run --json log "$id"
  local asked by_rule by_client denied empty usage
  asked="$(grep -c '"type":"permission_requested"' "$last_out" || true)"
  by_rule="$(grep '"type":"permission_decided"' "$last_out" | grep -c '"decidedBy":"rule"' || true)"
  by_client="$(grep '"type":"permission_decided"' "$last_out" | grep -c '"decidedBy":"client"' || true)"
  denied="$(grep '"type":"permission_decided"' "$last_out" | grep -c '"denied":true' || true)"
  empty="$(grep -c '"type":"turn_empty"' "$last_out" || true)"
  usage="$(jq -r 'select(.type == "agent_update" and .payload.update.sessionUpdate == "usage_update")
                  | "\(.payload.update.cost.amount // "") \(.payload.update.cost.currency // "") · \(.payload.update.used // "")/\(.payload.update.size // "")"' \
           "$last_out" 2>/dev/null | tail -n 1 || true)"
  echo "$asked · $by_rule · $by_client · $denied|$empty|$usage"
}

# ---- the fixture ---------------------------------------------------------------

mkdir -p "$fixture/src" "$fixture/.meltemi/changes/greeting"
printf '# harbour\nA tiny fixture for the integrated run.\nIt greets people.\n' >"$fixture/README.md"
printf 'def greet(name):\n    return f"hello {name}"\n' >"$fixture/src/greet.py"
printf '# greeting\n\n## Why\n\nPeople leave as well as arrive.\n\n## What Changes\n\n- A farewell next to the greeting.\n' \
  >"$fixture/.meltemi/changes/greeting/proposal.md"
printf '## 1. Farewell\n\n- [ ] 1.1 Add a function `farewell(name)` to src/greet.py that returns `goodbye {name}`\n' \
  >"$fixture/.meltemi/changes/greeting/tasks.md"
if [ "$mock" = 1 ]; then
  printf "[agent]\ncommand = ['%s', '--turn-delay-ms', '1500', '--honor-cancel', '--titled-permission']\n" \
    "$bin_dir/mock-agent" >"$fixture/.meltemi/config.toml"
fi
git -C "$fixture" init -q
git -C "$fixture" config user.email 'smoke@meltemi.test'
git -C "$fixture" config user.name 'Meltemi Smoke'
git -C "$fixture" config commit.gpgsign false
git -C "$fixture" add -A
git -C "$fixture" commit -q -m fixture

# The rule the second phase runs under: reading and editing inside the fixture,
# and nothing else. Running commands stays without a rule, and without a human
# it is denied.
ruled="[[rule]]
effect = \"allow\"
tool = \"read\"
path-prefix = '$fixture'

[[rule]]
effect = \"allow\"
tool = \"edit\"
path-prefix = '$fixture'
"

# ---- who runs ------------------------------------------------------------------

run --version
version="$(tr -d '\r\n' <"$last_out")"
run --json fleet
fleet_file="$base/fleet.json"
cp "$last_out" "$fleet_file"
# Plain arrays and files, no associative arrays: bash 3.2 has none.
versions_file="$base/versions.tsv"
: >"$versions_file"
ready=()
if [ "$mock" = 1 ]; then
  ready=(mock)
  printf 'mock\tmock-agent (scripted)\n' >>"$versions_file"
else
  while IFS= read -r id; do ready+=("$id"); done < <(jq -r '.agents[] | select(.installState == "ready") | .id' "$fleet_file")
  if [ "${#agents[@]}" -gt 0 ]; then
    filtered=()
    for id in "${ready[@]}"; do
      for want in "${agents[@]}"; do [ "$id" = "$want" ] && filtered+=("$id"); done
    done
    ready=("${filtered[@]+"${filtered[@]}"}")
  fi
  for id in "${ready[@]}"; do
    cli="$(jq -r --arg id "$id" '.agents[] | select(.id == $id) | .layers[] | select(.kind == "cli") | .binaryPath' "$fleet_file" | head -n 1)"
    printf '%s\t%s\n' "$id" "$("$cli" --version 2>&1 | head -n 1 | tr -d '\r' || echo unknown)" >>"$versions_file"
  done
fi
not_installed=()
while IFS= read -r id; do not_installed+=("$id"); done < <(jq -r '.agents[] | select(.installState != "ready") | .id' "$fleet_file")
version_of() { awk -F '\t' -v id="$1" '$1 == id { print $2 }' "$versions_file"; }
echo "$version · agents: ${ready[*]} · fixture: $fixture"

# ---- the matrix ----------------------------------------------------------------

rows_file="$base/rows.tsv"
: >"$rows_file"
add_row() { # id verb verdict facts note
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$last_seconds" "$4" "$5" >>"$rows_file"
  echo "  $1  $2  $3 (${last_seconds} s)"
}
agent_args() { if [ "$mock" = 1 ]; then :; else echo "--agent $1"; fi; }

read_file="$base/read-sessions.tsv"
: >"$read_file"
read_session_of() { awk -F '\t' -v id="$1" '$1 == id { print $2 }' "$read_file"; }
# Phase 1, with no rule: a reading session, and a write that must be denied.
for id in "${ready[@]}"; do
  # shellcheck disable=SC2046
  run --json session 'Read README.md and reply with its first line only. Change nothing.' "$fixture" $(agent_args "$id")
  sid="$(json .sessionId)"; printf '%s\t%s\n' "$id" "$sid" >>"$read_file"
  status="$(json .status)"
  verdict="exit $last_exit"; [ "$last_exit" = 0 ] && [ "$status" = completed ] && verdict=completed
  seconds=$last_seconds; f="$(facts "$sid")"; last_seconds=$seconds
  add_row "$id" 'session (read)' "$verdict" "$f" ''

  file="NOTES-$id-unruled.md"
  # shellcheck disable=SC2046
  run --json session "Create a file named $file containing the single line: hello. Do nothing else." "$fixture" $(agent_args "$id")
  sid="$(json .sessionId)"
  verdict='not written (denied)'; [ -e "$fixture/$file" ] && verdict='WRITTEN without a rule'
  seconds=$last_seconds; f="$(facts "$sid")"; last_seconds=$seconds
  add_row "$id" 'session (write, no rule)' "$verdict" "$f" ''
done

printf '%s' "$ruled" >"$fixture/.meltemi/permissions.toml"

# Phase 2, under the fixture's rule.
for id in "${ready[@]}"; do
  file="NOTES-$id-ruled.md"
  # shellcheck disable=SC2046
  run --json session "Create a file named $file containing the single line: hello. Do nothing else." "$fixture" $(agent_args "$id")
  sid="$(json .sessionId)"
  verdict='not written'; [ -e "$fixture/$file" ] && verdict='written by rule'
  seconds=$last_seconds; f="$(facts "$sid")"; last_seconds=$seconds
  add_row "$id" 'session (write, rule)' "$verdict" "$f" ''

  # `propose` answers with the change, not the session: the session is the one
  # the fixture did not have before.
  run --json sessions "$fixture"; before="$(jq -r '.sessions[].sessionId' "$last_out" 2>/dev/null || true)"
  # shellcheck disable=SC2046
  run --json propose "add a farewell function next to greet, drafted by $id" "$fixture" $(agent_args "$id")
  verdict="exit $last_exit"; [ "$last_exit" = 0 ] && verdict="$(json .status)"
  change="$(json .changeName)"; seconds=$last_seconds
  run --json sessions "$fixture"
  proposed="$(jq -r '.sessions[].sessionId' "$last_out" 2>/dev/null | grep -vxF -f <(printf '%s\n' "$before") | head -n 1 || true)"
  f="$(facts "$proposed")"; last_seconds=$seconds
  add_row "$id" 'propose' "$verdict" "$f" "$change"

  run --json assign greeting 1.1 "$id" "$fixture"; assigned=$last_exit
  run --json dispatch greeting 1.1 "$id" "$fixture"
  verdict="exit $last_exit"; [ "$last_exit" = 0 ] && verdict="$(json .status)"
  note='no commit'; [ "$(json .committed)" = true ] && note="commit $(json .sha)"
  [ "$assigned" != 0 ] && note="assign exit $assigned"
  sid="$(json .sessionId)"; seconds=$last_seconds; f="$(facts "$sid")"; last_seconds=$seconds
  add_row "$id" 'dispatch' "$verdict" "$f" "$note"

  run --json direct "$(read_session_of "$id")" 'Reply with the second line of README.md only.' "$fixture"
  verdict="exit $last_exit"; note=''
  if [ "$last_exit" = 0 ]; then verdict="$(json .disposition)"; else note="$(json .error.message | cut -c1-160)"; fi
  sid="$(json .sessionId)"; seconds=$last_seconds; f="$(facts "$sid")"; last_seconds=$seconds
  add_row "$id" 'direct' "$verdict" "$f" "$note"

  # A long turn, cancelled from another command as soon as it is live.
  run --json status; known="$(jq -r '.sessions[].sessionId' "$last_out" 2>/dev/null || true)"
  step=$((step + 1))
  bg_out="$run_dir/$(printf '%03d' "$step").out"
  start="$(date +%s)"
  # shellcheck disable=SC2046
  (cd "$fixture" && "$meltemi" --json session 'Write a long essay about lighthouses, at least twenty paragraphs.' "$fixture" $(agent_args "$id") >"$bg_out" 2>&1) &
  bg=$!
  live=''
  for _ in $(seq 1 200); do
    sleep 0.3
    run --json status
    live="$(jq -r '.sessions[].sessionId' "$last_out" 2>/dev/null | grep -vxF -f <(printf '%s\n' "$known") | head -n 1 || true)"
    [ -n "$live" ] && break
  done
  cancel_exit=''
  if [ -n "$live" ]; then run --json cancel "$live"; cancel_exit=$last_exit; fi
  wait "$bg" || true
  last_seconds=$(( $(date +%s) - start ))
  verdict='never live'
  [ -n "$live" ] && verdict="cancel exit $cancel_exit"
  [ -n "$live" ] && [ "$cancel_exit" = 0 ] && verdict="cancelled ($(jq -r '.status // empty' "$bg_out" 2>/dev/null || true))"
  seconds=$last_seconds; f="$(facts "$live")"; last_seconds=$seconds
  add_row "$id" 'cancel' "$verdict" "$f" ''
done

# ---- the report ----------------------------------------------------------------

commit="$(git -C "$repo" rev-parse --short HEAD)"
os="$(uname -sr)"
if [ -z "$report" ]; then
  if [ "$mock" = 1 ]; then report="$base/report.md"; else report="$repo/docs/qa/$stamp_day-corrida-integrada.md"; fi
fi
{
  echo '<!-- SPDX-License-Identifier: Apache-2.0 -->'
  echo "# Corrida integrada — $stamp_day"
  echo
  echo "Escrita por \`scripts/smoke-agents.sh\` el $stamp_time UTC, sobre \`$version\` (commit \`$commit\`), en $os. Daemon, datos y configuración aislados; un repositorio fixture temporal."
  echo
  echo '## Agentes'
  echo
  echo '| Agente | Versión |'
  echo '| --- | --- |'
  for id in "${ready[@]}"; do echo "| \`$id\` | $(version_of "$id") |"; done
  for id in "${not_installed[@]+"${not_installed[@]}"}"; do echo "| \`$id\` | **no medido**: no instalado |"; done
  echo
  echo '## La matriz'
  echo
  echo 'Permisos: pedidos · decididos por regla · por el cliente · denegados. Uso: el que el agente reportó por el protocolo (costo, contexto usado/tamaño), vacío si no reportó.'
  for id in "${ready[@]}"; do
    echo
    echo "### \`$id\`"
    echo
    echo '| Verbo | Resultado | Segundos | Permisos | Turnos vacíos | Uso | Nota |'
    echo '| --- | --- | --- | --- | --- | --- | --- |'
    while IFS=$'\t' read -r rid verb verdict seconds f note; do
      [ "$rid" = "$id" ] || continue
      IFS='|' read -r perms empty usage <<<"$f"
      echo "| $verb | $verdict | $seconds | $perms | $empty | $usage | ${note//|//} |"
    done <"$rows_file"
  done
  echo
  echo '## Hallazgos'
  echo
  echo '_Los escribe quien corre: qué no se esperaba, y la change que lo aborda._'
} >"$report"
echo "report: $report"

# ---- teardown: only what this run started ---------------------------------------

run stop || true
sleep 1
pkill -f "^$meltemid" 2>/dev/null || true
echo "fixture and logs kept for inspection: $base"
