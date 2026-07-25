// SPDX-License-Identifier: Apache-2.0
// Own line iconography (design system: iconos de línea, trazo 1.5, geometría
// del mark). Inline SVG paths — no icon font, no platform emoji, nothing
// remote. Every icon that carries state is paired with a visible or
// accessible label by its caller.

export type IconName =
  | "sessions"
  | "project"
  | "permissions"
  | "fleet"
  | "editor"
  | "analytics"
  | "settings"
  | "search"
  | "plus"
  | "chevronDown"
  | "chevronRight"
  | "close"
  | "check"
  | "warning"
  | "refresh"
  | "external"
  | "copy"
  | "save"
  | "diff";

/** Path data for a 24×24 viewBox, stroked (never filled). */
export const ICONS: Record<IconName, string> = {
  // Two asymmetric sails over a tapered hull: the mark's geometry.
  fleet: "M12 3v9M12 3l6 9h-6M12 6l-4 6h4M4 16h16l-2.5 4h-11z",
  sessions: "M4 12h3l2.5-6 3 12 2.5-6H20",
  project: "M4 6h6l1.5 2H20v11H4zM4 6v13",
  permissions: "M12 3l7 3v6c0 4-3 6.5-7 8-4-1.5-7-4-7-8V6zM9 12l2 2 4-4",
  editor: "M4 5h9M4 10h6M4 15h5M13.5 17.5l6-6 2.5 2.5-6 6H13.5z",
  analytics: "M4 20V10M9.5 20V5M15 20v-7M20.5 20v-11",
  settings: "M12 9.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5zM4 12h2M18 12h2M12 4v2M12 18v2M6.5 6.5l1.5 1.5M16 16l1.5 1.5M17.5 6.5L16 8M8 16l-1.5 1.5",
  search: "M11 4a7 7 0 100 14 7 7 0 000-14zM16 16l4 4",
  plus: "M12 5v14M5 12h14",
  chevronDown: "M6 9.5l6 6 6-6",
  chevronRight: "M9.5 6l6 6-6 6",
  close: "M6 6l12 12M18 6L6 18",
  check: "M5 13l4.5 4.5L19 7",
  warning: "M12 4l8.5 15H3.5zM12 10v4M12 16.5v.5",
  refresh: "M20 12a8 8 0 11-2.5-5.8M20 4v4h-4",
  external: "M14 5h5v5M19 5l-7 7M10 6H6v12h12v-4",
  copy: "M9 9h10v11H9zM15 9V4H5v11h4",
  save: "M5 5h11l3 3v11H5zM8 5v5h7M8 15h8",
  diff: "M6 4v10a3 3 0 003 3h9M18 14l-3 3 3 3M9 4h9M13.5 8H9",
};
