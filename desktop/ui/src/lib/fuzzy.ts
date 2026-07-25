// SPDX-License-Identifier: Apache-2.0
// Subsequence fuzzy matching for the palette and quick-open. Own code, no
// dependency: the universe is ~40 registry entries and a project tree, and a
// library would be an unjustifiable dependency (constitution §10).
//
// Scoring rewards, in order: a match at the start of a word, a match at the
// start of a path/method segment, and contiguity. "wapp" therefore reaches
// worktree/apply-edit, and "sdv" reaches sdd/validate.

const SEGMENT_BREAK = /[/\-_. ]/;

export interface FuzzyMatch {
  score: number;
  /** Indices of the matched characters in the haystack (for highlighting). */
  positions: number[];
}

/**
 * Matches `needle` as a subsequence of `haystack` (both compared
 * case-insensitively). Returns `null` when no subsequence exists.
 */
export function fuzzyMatch(needle: string, haystack: string): FuzzyMatch | null {
  const query = needle.trim().toLowerCase();
  if (query === "") return { score: 0, positions: [] };
  const target = haystack.toLowerCase();

  const positions: number[] = [];
  let score = 0;
  let cursor = 0;
  let previousIndex = -2;

  for (const char of query) {
    let index = -1;
    // Prefer a boundary occurrence (start of a segment) over the first one.
    for (let i = cursor; i < target.length; i += 1) {
      if (target[i] !== char) continue;
      const boundary = i === 0 || SEGMENT_BREAK.test(target[i - 1]);
      if (boundary) {
        index = i;
        break;
      }
      if (index === -1) index = i;
    }
    if (index === -1) return null;

    positions.push(index);
    score += 1;
    if (index === 0) score += 6;
    else if (SEGMENT_BREAK.test(target[index - 1])) score += 4;
    if (index === previousIndex + 1) score += 3;
    previousIndex = index;
    cursor = index + 1;
  }

  // Shorter haystacks win ties: a whole-name match beats a buried one.
  score += Math.max(0, 12 - Math.round(target.length / 4));
  return { score, positions };
}

/** Sorts candidates by score (descending), dropping non-matches. */
export function fuzzyRank<T>(
  needle: string,
  candidates: T[],
  key: (item: T) => string,
  bonus: (item: T) => number = () => 0,
): { item: T; match: FuzzyMatch }[] {
  const ranked: { item: T; match: FuzzyMatch }[] = [];
  for (const item of candidates) {
    const match = fuzzyMatch(needle, key(item));
    if (!match) continue;
    ranked.push({ item, match: { ...match, score: match.score + bonus(item) } });
  }
  ranked.sort((a, b) => b.match.score - a.match.score);
  return ranked;
}

/** Splits a string into matched/unmatched runs for highlighted rendering. */
export function highlightRuns(
  text: string,
  positions: number[],
): { text: string; hit: boolean }[] {
  if (positions.length === 0) return [{ text, hit: false }];
  const hits = new Set(positions);
  const runs: { text: string; hit: boolean }[] = [];
  let current = "";
  let currentHit = hits.has(0);
  for (let i = 0; i < text.length; i += 1) {
    const hit = hits.has(i);
    if (hit !== currentHit && current !== "") {
      runs.push({ text: current, hit: currentHit });
      current = "";
    }
    currentHit = hit;
    current += text[i];
  }
  if (current !== "") runs.push({ text: current, hit: currentHit });
  return runs;
}
