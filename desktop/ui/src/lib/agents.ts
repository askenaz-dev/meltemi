// SPDX-License-Identifier: Apache-2.0
// Entity identity: an initials avatar whose color is stable for a given id, so
// the same agent looks the same in every view and every session. The palette
// and the per-agent assignments come from the design system's fleet identity
// section; unknown ids (custom agents, profiles) fall back to a hash over the
// same palette — deterministic, no state, no configuration.

export interface AvatarTone {
  bg: string;
  fg: string;
}

/** The design system's fleet palette (dark chips, legible in both themes). */
const PALETTE: AvatarTone[] = [
  { bg: "#72243e", fg: "#f4c0d1" },
  { bg: "#333f52", fg: "#c7d2e0" },
  { bg: "#5a1f6b", fg: "#eec2f7" },
  { bg: "#0b4a5c", fg: "#a6dfec" },
  { bg: "#3d4d10", fg: "#d3e59a" },
  { bg: "#085041", fg: "#9fe1cb" },
  { bg: "#3c3489", fg: "#cecbf6" },
  { bg: "#0c447c", fg: "#b5d4f4" },
  { bg: "#633806", fg: "#fac775" },
  { bg: "#6b2318", fg: "#f6b6a6" },
];

/** Assignments fixed by the design system, so the app matches the UI kit. */
const ASSIGNED: Record<string, number> = {
  "gemini-cli": 0,
  "copilot-cli": 1,
  "cursor-cli": 2,
  "kiro-cli": 3,
  "kilo-code": 4,
  opencode: 5,
  "claude-code": 6,
  "codex-cli": 7,
  aider: 8,
  antigravity: 9,
};

/** FNV-1a over the id: stable across runs, platforms and locales. */
function hash(id: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < id.length; i += 1) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

export function toneFor(id: string): AvatarTone {
  const assigned = ASSIGNED[id];
  if (assigned !== undefined) return PALETTE[assigned];
  return PALETTE[hash(id) % PALETTE.length];
}

/** Two-letter initials from a display name (or the id as a fallback). */
export function initialsFor(name: string, id?: string): string {
  const source = name.trim() || id?.trim() || "?";
  const words = source
    .split(/[\s\-_./]+/)
    .filter((word) => /[a-z0-9]/i.test(word));
  if (words.length >= 2) {
    return (words[0][0] + words[1][0]).toUpperCase();
  }
  return source.replace(/[^a-z0-9]/gi, "").slice(0, 2).toUpperCase() || "?";
}

/** The last path segment of a binary path — the name the user recognizes. */
export function binaryName(command: string[] | string | undefined): string {
  const program = Array.isArray(command) ? command[0] : command;
  if (!program) return "";
  return program.replaceAll("\\", "/").split("/").pop() ?? program;
}
