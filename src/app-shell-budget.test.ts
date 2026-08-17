import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const shell = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

/// Logic still living in the shell. This list may get shorter. It may not get longer.
///
/// `src/App.tsx` is a wiring file: the App component's state and handlers, the screen components,
/// and JSX. A function that can be called from a test without rendering anything is a rule, not
/// wiring, and a rule that lives here is one nobody can test directly and one that every unrelated
/// change has to merge around. Two people — or two agents — working on different features both end
/// up editing the same file, and git resolves that as a conflict on every hunk.
///
/// So when you write a new pure function, put it in a module beside its own `.test.ts` and import
/// it. When you touch one of the names below, consider taking it with you: delete its line here and
/// this test agrees.
///
/// What remains is mostly presentation — label tables that translate a backend enum for one screen,
/// and small input parsers used by one form. They are the least valuable to move and the least
/// costly to leave, which is why they are last.
const LOGIC_STILL_IN_THE_SHELL = [
  "aggregateStrategyHelp",
  "aggregateStrategyLabel",
  "boundedPercentOrDefault",
  "boundedPercentOrNull",
  "catalogDraftErrorLabel",
  "envConflictSourceLabel",
  "formatTime",
  "integerOrDefault",
  "integerOrNull",
  "loadInitialRoute",
  "loadInitialTheme",
  "managedCatalogMode",
  "parseCommaListOrNull",
  "parseReasoningLevels",
  "positiveNumberOrDefault",
  "positiveNumberOrNull",
  "providerInitial",
  "reasoningEffortsText",
  "relayModeLabel",
  "relayProfileConfigBrief",
  "relayProfileEditorStatus",
  "relayProfileModeHelp",
  "relayProtocolLabel",
  "routeSubtitle",
  "routeTitle",
  "stringifyError",
  "truncateSessionDeletePreview",
];

/// A React component. Distinguished from a rule by its capital letter, which is the convention JSX
/// already requires — so the rule needs no annotation and cannot be opted out of by accident.
const isComponent = (name: string) => /^[A-Z]/.test(name);

const declared = [...shell.matchAll(/^(?:export )?function ([A-Za-z_]\w*)/gm)].map((match) => match[1]);

describe("the application shell holds wiring, not rules", () => {
  it("was read from the real file", () => {
    assert.ok(declared.length > 20, "App.tsx declares functions");
    assert.ok(declared.includes("App"), "the App component was located");
  });

  it("adds no new logic to the shell", () => {
    const added = declared.filter((name) => !isComponent(name) && !LOGIC_STILL_IN_THE_SHELL.includes(name));
    assert.deepEqual(
      added,
      [],
      `put these in a module beside their own test instead of in App.tsx: ${added.join(", ")}`,
    );
  });

  it("keeps the list honest as functions leave", () => {
    const stale = LOGIC_STILL_IN_THE_SHELL.filter((name) => !declared.includes(name));
    assert.deepEqual(stale, [], `these already left the shell — delete their lines: ${stale.join(", ")}`);
  });

  it("holds no more of the shell than it did", () => {
    // A ceiling, not a target. Lower it when work makes it true; never raise it to make a change fit.
    const lines = shell.split("\n").length;
    assert.ok(
      lines <= 3490,
      `App.tsx is ${lines} lines, over the 3490 ceiling — move something out rather than raising it`,
    );
  });
});
