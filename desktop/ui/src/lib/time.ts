// SPDX-License-Identifier: Apache-2.0
// Human time: localized relative labels with the absolute instant kept
// accessible (spec "Tiempos relativos con absoluto accesible").

import type { Locale } from "./messages";

/** `hace 3 min` / `3 min ago`, via Intl.RelativeTimeFormat. */
export function relativeTime(iso: string, locale: Locale): string {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return iso;
  const seconds = Math.round((then - Date.now()) / 1000);
  const format = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const steps: [Intl.RelativeTimeFormatUnit, number][] = [
    ["second", 60],
    ["minute", 60],
    ["hour", 24],
    ["day", 7],
    ["week", 4.35],
    ["month", 12],
  ];
  let value = seconds;
  for (const [unit, span] of steps) {
    if (Math.abs(value) < span) return format.format(Math.round(value), unit);
    value /= span;
  }
  return format.format(Math.round(value), "year");
}

/** The full local timestamp, for the accessible/title attribute. */
export function absoluteTime(iso: string, locale: Locale): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(date);
}

/** `hh:mm:ss` of an ISO instant, for transcript lines. */
export function clockTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso.slice(11, 19);
  return date.toTimeString().slice(0, 8);
}

/** A compact duration between two instants (or until now). */
export function durationLabel(startedAt: string, endedAt?: string): string {
  const start = Date.parse(startedAt);
  const end = endedAt ? Date.parse(endedAt) : Date.now();
  if (Number.isNaN(start) || Number.isNaN(end)) return "";
  const seconds = Math.max(0, Math.round((end - start) / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}
