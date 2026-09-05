/* What the language client promises the server.
 *
 * The server recovers from a compiler it could not run by re-arming on
 * `onDidChangeConfiguration`. That notification only ever arrives if the
 * client asked for it: `SyncConfigurationFeature.initialize` registers its
 * workspace listener when `synchronize.configurationSection` is set and
 * returns without registering anything when it is not
 * (vscode-languageclient 9.0.1). The section is therefore part of the
 * contract, not a detail of how the options happen to be spelled
 * (TASK-340). */
import * as assert from "node:assert/strict";
import { test } from "node:test";

import { ttClientOptions } from "../options";

test("configuration changes are synchronized, so the server can be re-armed", () => {
  const options = ttClientOptions([]);
  assert.equal(options.synchronize?.configurationSection, "tt");
});

test("the file watchers stay wired alongside the configuration section", () => {
  const watchers = [{} as never, {} as never];
  const options = ttClientOptions(watchers);
  assert.deepEqual(options.synchronize?.fileEvents, watchers);
});

test("both tt languages are served, saved and unsaved", () => {
  const options = ttClientOptions([]);
  assert.deepEqual(options.documentSelector, [
    { scheme: "file", language: "tt" },
    { scheme: "untitled", language: "tt" },
    { scheme: "file", language: "ttx" },
    { scheme: "untitled", language: "ttx" },
  ]);
});
