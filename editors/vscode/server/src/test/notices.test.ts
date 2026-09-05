/* Standing notices, and the recovery that ends them.
 *
 * The server says each of these once, so it does not repeat itself on
 * every keystroke. That is only correct while the notice still stands: a
 * settings change, a folder change, or a compiler replaced on disk can all
 * fix the thing it describes, and after any of them the next failure has to
 * be reported again. As separate flags only the one whose reset someone
 * remembered to write was re-armed (TASK-343). */
import * as assert from "node:assert/strict";
import { test } from "node:test";

import { NoticeLedger, type NoticeId } from "../notices";

const ALL: NoticeId[] = [
  "compiler-unusable",
  "typed-check-unavailable",
  "typed-compiler-failure",
  "sidecar-dir-unresolved",
];

test("a notice is said once and then stands", () => {
  const notices = new NoticeLedger();
  assert.equal(notices.raise("compiler-unusable"), true);
  assert.equal(notices.raise("compiler-unusable"), false);
  assert.equal(notices.raise("compiler-unusable"), false);
});

test("notices do not silence one another", () => {
  const notices = new NoticeLedger();
  for (const id of ALL) {
    assert.equal(notices.raise(id), true, id);
  }
});

test("every notice is re-armed by one reset", () => {
  const notices = new NoticeLedger();
  for (const id of ALL) notices.raise(id);
  notices.reset();
  for (const id of ALL) {
    assert.equal(notices.raise(id), true, `${id} was not re-armed`);
  }
});

test("a reset with nothing standing changes nothing", () => {
  const notices = new NoticeLedger();
  notices.reset();
  assert.equal(notices.raise("typed-check-unavailable"), true);
  assert.equal(notices.raise("typed-check-unavailable"), false);
});
