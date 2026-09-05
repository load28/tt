/* The language client's options, apart from the client itself so the
 * contract they carry can be read — and tested — without an extension
 * host. */
import type { FileSystemWatcher } from "vscode";
import type { LanguageClientOptions } from "vscode-languageclient/node";

/** The section whose changes the client forwards to the server.
 *
 * `SyncConfigurationFeature` registers its `onDidChangeConfiguration`
 * listener only when `synchronize.configurationSection` is set
 * (vscode-languageclient 9.0.1), so without this the server's own
 * configuration handler — which re-arms a compiler that struck out and
 * re-validates every open buffer — is unreachable. A workspace-scoped edit
 * still recovered through the file watchers below, by accident; a
 * user-settings edit, which is where someone fixes `tt.compilerPath` after
 * the "compiler not found" notification, never did. */
export const CONFIGURATION_SECTION = "tt";

/** The languages the server is asked about, on disk and unsaved alike. */
export const DOCUMENT_SELECTOR = [
  { scheme: "file", language: "tt" },
  { scheme: "untitled", language: "tt" },
  { scheme: "file", language: "ttx" },
  { scheme: "untitled", language: "ttx" },
];

export function ttClientOptions(
  fileEvents: FileSystemWatcher[],
): LanguageClientOptions {
  return {
    documentSelector: [...DOCUMENT_SELECTOR],
    synchronize: {
      fileEvents,
      configurationSection: CONFIGURATION_SECTION,
    },
  };
}
