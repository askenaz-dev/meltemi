// SPDX-License-Identifier: Apache-2.0

/**
 * How this surface reaches the daemon, when it is not this machine's own socket
 * (via-ssh-del-cliente design D8).
 *
 * An endpoint can name a command instead of a socket — `exec:ssh my-pc meltemi
 * bridge` — which means the fleet on screen belongs to another machine. A
 * surface that stayed quiet about that would make an action on the wrong machine
 * look like an action on your own, so the chrome says it.
 *
 * A leaf module with no imports, so the node test runner can load it without the
 * Tauri bridge, exactly like `session-state.ts`.
 */

const PREFIX = "exec:";

/**
 * Splits a command line the same way the client does: whitespace separates and
 * double quotes group. Deliberately not a shell, and deliberately the same rule
 * on both sides, so what the chrome shows is what the client launched.
 */
function split(command: string): string[] {
  const parts: string[] = [];
  let current = "";
  let inQuotes = false;
  let started = false;
  for (const ch of command) {
    if (ch === '"') {
      inQuotes = !inQuotes;
      started = true;
    } else if (/\s/.test(ch) && !inQuotes) {
      if (started) {
        parts.push(current);
        current = "";
        started = false;
      }
    } else {
      current += ch;
      started = true;
    }
  }
  if (started) parts.push(current);
  return parts;
}

/**
 * The via worth showing: the program's own name and its first argument, which
 * for the case this exists for is `ssh <host>`. `null` for a local socket, which
 * has no via to name.
 */
export function via(endpoint: string | undefined | null): string | null {
  if (!endpoint || !endpoint.startsWith(PREFIX)) return null;
  const parts = split(endpoint.slice(PREFIX.length));
  const program = parts[0];
  if (!program) return null;
  // Just the program's name, not a path somebody spelled out in full.
  const name = (program.split(/[/\\]/).pop() ?? program).replace(/\.exe$/i, "");
  return parts[1] ? `${name} ${parts[1]}` : name;
}
