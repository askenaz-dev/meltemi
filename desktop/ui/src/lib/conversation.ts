// SPDX-License-Identifier: Apache-2.0
// The fold that turns a session's event log into a conversation
// (lanzador-conversacional design D4). It is a READING of the log and never a
// second source: every event handed in comes out somewhere — as a turn, as a
// card, or as a neutral system line in its own position — and nothing is
// invented that the log does not carry.

/** One event of the transcript, as the surface holds it. */
export interface FoldEvent {
  id: number;
  ts: string;
  type: string;
  payload: unknown;
}

export interface AgentText {
  kind: "text";
  text: string;
}
export interface AgentThought {
  kind: "thought";
  text: string;
}
export interface AgentTool {
  kind: "tool";
  toolCallId: string;
  title: string;
  status: string | null;
}
export interface AgentPlan {
  kind: "plan";
  entries: string[];
}

export type AgentPart = AgentText | AgentThought | AgentTool | AgentPlan;

/**
 * Every item carries the ids of the events it accounts for, so "the reading
 * omits nothing" is a property that can be checked rather than promised: the
 * union of these is exactly the set of events handed in, each exactly once.
 */
interface Accounted {
  eventIds: number[];
}

export interface HumanTurn extends Accounted {
  kind: "human";
  id: number;
  ts: string;
  text: string;
  /** Queued and not yet dispatched: shown as pending, never as attended. */
  pending: boolean;
}

export interface AgentTurn extends Accounted {
  kind: "agent";
  id: number;
  ts: string;
  parts: AgentPart[];
  /** The stop reason of `turn_completed`, shown rather than swallowed. */
  stopReason: string | null;
  closed: boolean;
}

export interface PermissionCard extends Accounted {
  kind: "permission";
  id: number;
  ts: string;
  /** What the agent asked to do, in the tool call's own words. */
  title: string;
  options: { optionId: string; name: string; kind?: string }[];
  /** Filled by a later `permission_decided`; until then the card is live. */
  decided: { by: string; denied: boolean | null; rule: string | null } | null;
}

export interface SystemLine extends Accounted {
  kind: "system";
  id: number;
  ts: string;
  type: string;
  text: string;
}

export type ConversationItem = HumanTurn | AgentTurn | PermissionCard | SystemLine;

/** The events that close an agent turn. `turn_completed` carries its reason. */
const CLOSERS = new Set(["turn_completed", "session_cancelled", "session_ended", "error"]);

function record(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

function str(value: unknown): string | null {
  return typeof value === "string" && value.trim() !== "" ? value : null;
}

/** A short, flat rendering of any payload, for the neutral system line. */
export function flatten(payload: unknown): string {
  if (payload === undefined || payload === null) return "";
  if (typeof payload !== "object") return String(payload);
  const fields = record(payload);
  for (const key of ["text", "message", "instruction", "detail", "reason", "title", "file"]) {
    const value = str(fields[key]);
    if (value) return value.replaceAll(/\s+/g, " ").trim();
  }
  return JSON.stringify(payload);
}

/**
 * Classifies one `agent_update`. The three shapes the contract can deliver are
 * all handled: an ACP update (level 1/2), a level-3 line the daemon mapped to
 * `{type: "text" | "message"}`, and a raw string for a line it could not map.
 * Anything else returns null and is rendered in place as a system line —
 * folding an update we do not understand into the prose would be inventing.
 */
export function classifyUpdate(payload: unknown): AgentPart | null {
  const update = record(payload).update;
  if (typeof update === "string") {
    return update.trim() ? { kind: "text", text: update } : null;
  }
  const fields = record(update);

  const sessionUpdate = str(fields.sessionUpdate);
  if (sessionUpdate === "agent_message_chunk" || sessionUpdate === "agent_thought_chunk") {
    const content = record(fields.content);
    if (str(content.type) !== "text") return null;
    const text = str(content.text);
    if (!text) return null;
    return sessionUpdate === "agent_thought_chunk"
      ? { kind: "thought", text }
      : { kind: "text", text };
  }
  if (sessionUpdate === "tool_call" || sessionUpdate === "tool_call_update") {
    const id = str(fields.toolCallId) ?? "";
    // NOT defaulted to the id: an update that carries only a status must not
    // overwrite the title the opening call gave, and it would if the id stood
    // in for a missing title.
    const title = str(fields.title) ?? str(fields.kind) ?? "";
    if (!id && !title) return null;
    return { kind: "tool", toolCallId: id || title, title, status: str(fields.status) };
  }
  if (sessionUpdate === "plan") {
    const entries = Array.isArray(fields.entries) ? fields.entries : [];
    return {
      kind: "plan",
      entries: entries.map((entry) => str(record(entry).content) ?? flatten(entry)),
    };
  }

  // The level-3 dialect: `map_headless_line` delivers `{type, ...}` prose.
  const mapped = str(fields.type);
  if (mapped === "text" || mapped === "message") {
    const text = str(fields.text) ?? str(fields.message);
    return text ? { kind: "text", text } : null;
  }
  return null;
}

/** Folds the transcript into a conversation. Nothing handed in is dropped. */
export function fold(events: FoldEvent[]): ConversationItem[] {
  const items: ConversationItem[] = [];
  let open: AgentTurn | null = null;

  const system = (event: FoldEvent) => {
    open = null;
    items.push({
      kind: "system",
      id: event.id,
      ts: event.ts,
      type: event.type,
      text: flatten(event.payload),
      eventIds: [event.id],
    });
  };

  for (const event of events) {
    const payload = record(event.payload);

    if (event.type === "instruction_queued") {
      open = null;
      items.push({
        kind: "human",
        id: event.id,
        ts: event.ts,
        text: str(payload.instruction) ?? "",
        pending: true,
        eventIds: [event.id],
      });
      continue;
    }

    if (event.type === "prompt_sent") {
      open = null;
      const text = str(payload.text) ?? "";
      // A queued instruction that now became the prompt resolves in place: the
      // text is the only handle the log gives, and two bubbles for one
      // instruction would read as two instructions.
      const queued = items.find(
        (item) => item.kind === "human" && item.pending && item.text === text,
      ) as HumanTurn | undefined;
      if (queued) {
        queued.pending = false;
        queued.eventIds.push(event.id);
        continue;
      }
      items.push({
        kind: "human",
        id: event.id,
        ts: event.ts,
        text,
        pending: false,
        eventIds: [event.id],
      });
      continue;
    }

    if (event.type === "agent_update") {
      const part = classifyUpdate(event.payload);
      if (!part) {
        system(event);
        continue;
      }
      if (!open) {
        open = {
          kind: "agent",
          id: event.id,
          ts: event.ts,
          parts: [],
          stopReason: null,
          closed: false,
          eventIds: [],
        };
        items.push(open);
      }
      open.eventIds.push(event.id);
      if (part.kind === "tool") {
        // Updated in place by id: one tool call is one chip, not a stream of them.
        const existing = open.parts.find(
          (candidate) => candidate.kind === "tool" && candidate.toolCallId === part.toolCallId,
        ) as AgentTool | undefined;
        if (existing) {
          existing.title = part.title || existing.title;
          existing.status = part.status ?? existing.status;
          continue;
        }
      }
      if (part.kind === "text" || part.kind === "thought") {
        const last = open.parts[open.parts.length - 1];
        if (last && last.kind === part.kind) {
          // Chunks are chunks: prose accumulates, and thinking accumulates
          // apart from it — never concatenated into the answer.
          last.text += part.text;
          continue;
        }
      }
      open.parts.push(part);
      continue;
    }

    if (event.type === "permission_requested") {
      open = null;
      const request = record(payload.request);
      const toolCall = record(request.toolCall);
      const options = Array.isArray(request.options)
        ? (request.options as PermissionCard["options"])
        : [];
      items.push({
        kind: "permission",
        id: event.id,
        ts: event.ts,
        title: str(toolCall.title) ?? str(toolCall.kind) ?? str(toolCall.toolCallId) ?? "",
        options,
        decided: null,
        eventIds: [event.id],
      });
      continue;
    }

    if (event.type === "permission_decided") {
      open = null;
      const card = [...items]
        .reverse()
        .find((item) => item.kind === "permission" && item.decided === null) as
        | PermissionCard
        | undefined;
      const decided = {
        by: str(payload.decidedBy) ?? "",
        denied: typeof payload.denied === "boolean" ? payload.denied : null,
        rule: payload.rule ? flatten(payload.rule) : null,
      };
      if (card) {
        card.decided = decided;
        card.eventIds.push(event.id);
        continue;
      }
      // A decision with no request in view still has to be seen.
      system(event);
      continue;
    }

    if (CLOSERS.has(event.type)) {
      if (event.type === "turn_completed" && open) {
        open.stopReason = str(payload.stopReason);
        open.closed = true;
        open.eventIds.push(event.id);
        open = null;
        continue;
      }
      if (open) {
        open.closed = true;
        open = null;
      }
      system(event);
      continue;
    }

    system(event);
  }

  return items;
}
