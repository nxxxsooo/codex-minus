import assert from "node:assert/strict";
import { test } from "node:test";
import { cleanupSessions } from "./session-cleanup.ts";
import type { LocalSession, LocalSessionsResult } from "./backend-types.ts";

const session = (id: string, archived = true): LocalSession => ({
  id, archived, title: id, cwd: "", modelProvider: "openai", updatedAtMs: 1, rolloutPath: "", dbPath: "",
});
const page = (sessions: LocalSession[], nextCursor: string | null = null): LocalSessionsResult => ({
  status: "ok", message: "", sessions, nextCursor, archived: true, activeCount: 1,
  archivedCount: 201, pageSize: 200, elapsedMs: 0, dbPath: "", dbPaths: [],
});

function harness(pages: LocalSessionsResult[] = [], confirmed = true) {
  const invocations: { command: string; args: Record<string, unknown> }[] = [];
  const confirmations: string[] = [];
  const notices: unknown[][] = [];
  let refreshed = 0;
  return {
    invocations, confirmations, notices, get refreshed() { return refreshed; },
    ports: {
      invoke: async <T>(command: string, args: Record<string, unknown>): Promise<T> => {
        invocations.push({ command, args });
        return (command === "list_local_sessions" ? pages.shift() : {
          status: "ok", deletedCount: (args.request as { sessionIds: string[] }).sessionIds.length, failures: [],
        }) as T;
      },
      confirm: async (_title: string, message: string) => { confirmations.push(message); return confirmed; },
      notice: (...args: unknown[]) => { notices.push(args); },
      refresh: async () => { refreshed += 1; },
    },
  };
}

test("clear archive confirms all pages and sends exactly that archived snapshot", async () => {
  const first = Array.from({ length: 200 }, (_, index) => session(String(index)));
  const h = harness([page(first, "next"), page([session("last")])]);
  await cleanupSessions("archived", h.ports);
  assert.equal(h.invocations.length, 3);
  assert.deepEqual(h.invocations[1].args, { request: { archived: true, cursor: "next", pageSize: 200 } });
  assert.deepEqual(h.invocations[2], {
    command: "permanently_delete_local_sessions",
    args: { request: { sessionIds: [...first.map((item) => item.id), "last"], archivedOnly: true } },
  });
  assert.match(h.confirmations[0], /201/);
  assert.match(h.confirmations[0], /不留备份/);
  assert.equal(h.refreshed, 1);
});

test("canceling single, selected or clear-all never invokes deletion", async () => {
  for (const target of [[session("one", false)], [session("one"), session("two")], "archived"] as const) {
    const h = harness([page([session("one")])], false);
    await cleanupSessions(target === "archived" ? target : [...target], h.ports);
    assert.ok(h.invocations.every((call) => call.command === "list_local_sessions"));
    assert.equal(h.refreshed, 0);
  }
});

test("incomplete, mixed and expired archive pages abort before confirmation", async () => {
  for (const pages of [
    [{ ...page([]), status: "failed" }],
    [page([session("active", false)])],
    [page([session("one")], "loop"), page([session("two")], "loop")],
  ]) {
    const h = harness(pages);
    await assert.rejects(cleanupSessions("archived", h.ports));
    assert.equal(h.confirmations.length, 0);
    assert.ok(h.invocations.every((call) => call.command === "list_local_sessions"));
  }
});

test("selected deletion deduplicates IDs and allows explicitly selected active sessions", async () => {
  const h = harness();
  await cleanupSessions([session("one", false), session("one", false), session("two")], h.ports);
  assert.deepEqual(h.invocations[0].args, { request: { sessionIds: ["one", "two"], archivedOnly: false } });
});

test("partial failures stay failed with details and refresh; transport errors also refresh", async () => {
  const h = harness();
  h.ports.invoke = async <T>() => ({ status: "failed", deletedCount: 1, failures: [{ sessionId: "two", message: "locked" }] }) as T;
  await cleanupSessions([session("one"), session("two")], h.ports);
  assert.equal(h.notices[0][2], "failed");
  assert.match(String(h.notices[0][3]), /two: locked/);
  assert.equal(h.refreshed, 1);
  h.ports.invoke = async () => { throw new Error("transport lost"); };
  await assert.rejects(cleanupSessions([session("one")], h.ports), /transport lost/);
  assert.equal(h.refreshed, 2);
});
