// SPDX-License-Identifier: Apache-2.0
// The unified-diff grammar, shared by every surface that renders one
// (tablero-de-carrera design D3). Pure functions over text: the review
// drill-in and the race board must show the same diff the same way, and a
// parser private to one component guarantees they eventually will not.

/** One rendered line of a diff, classed and numbered against the NEW file. */
export interface DiffLine {
  text: string;
  /** `ctx` | `add` | `del` | `hunk` | `meta`. */
  kind: string;
  /** Line number in the new file, or null where the line has none. */
  newLine: number | null;
}

/** The lines of one file within a unified diff. */
export interface FileSection {
  file: string;
  lines: DiffLine[];
}

/** One hunk of a file section: the review and merge unit. */
export interface Hunk {
  header: string;
  lines: DiffLine[];
  /** First line of the NEW file this hunk touches, when it has one. */
  startLine: number | null;
}

/** Splits a unified diff into per-file sections with classed lines. */
export function fileSections(diff: string): FileSection[] {
  const sections: FileSection[] = [];
  let current: FileSection | null = null;
  let newLine = 0;
  for (const line of diff.split("\n")) {
    if (line.startsWith("diff --git")) {
      const file = line.split(" b/").pop() ?? line;
      current = { file, lines: [] };
      sections.push(current);
      continue;
    }
    if (!current) continue;
    let kind = "ctx";
    let numbered: number | null = null;
    if (line.startsWith("@@")) {
      kind = "hunk";
      const match = /\+(\d+)/.exec(line);
      newLine = match ? Number(match[1]) : 0;
    } else if (line.startsWith("+++") || line.startsWith("---")) {
      kind = "meta";
    } else if (line.startsWith("+")) {
      kind = "add";
      numbered = newLine;
      newLine += 1;
    } else if (line.startsWith("-")) {
      kind = "del";
    } else {
      numbered = newLine;
      newLine += 1;
    }
    current.lines.push({ text: line, kind, newLine: numbered });
  }
  return sections;
}

/**
 * The hunks of a file section, so review has a per-hunk unit: its header, its
 * lines, and the first line of the new file it touches (where editing and
 * "open with" land).
 */
export function hunksOf(section: FileSection): Hunk[] {
  const hunks: Hunk[] = [];
  let current: Hunk | null = null;
  for (const line of section.lines) {
    if (line.kind === "hunk") {
      current = { header: line.text, lines: [], startLine: null };
      hunks.push(current);
      continue;
    }
    if (line.kind === "meta") continue;
    if (!current) {
      // Lines before the first @@ (rename/mode headers): keep them visible in
      // their own unlabeled hunk rather than dropping them.
      current = { header: "", lines: [], startLine: null };
      hunks.push(current);
    }
    current.lines.push(line);
    if (current.startLine === null && line.newLine !== null) {
      current.startLine = line.newLine;
    }
  }
  return hunks;
}
