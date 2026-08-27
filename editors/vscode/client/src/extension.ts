/* --------------------------------------------------------------------------
 * tt language client — follows the official VS Code LSP extension pattern
 * (client launches the server over Node IPC and wires the `tt` language).
 * ----------------------------------------------------------------------- */
import * as path from "path";
import { ExtensionContext, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

import { registerContentMappers } from "./contentMapper";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
  // TypeScript 7.1+ holds `.tt`/`.ttx` virtually through a content mapper;
  // registering is fire-and-forget and never blocks the language client.
  void registerContentMappers(context);

  const serverModule = context.asAbsolutePath(
    path.join("server", "out", "server.js"),
  );

  const serverOptions: ServerOptions = {
    run: { module: serverModule, transport: TransportKind.ipc },
    debug: {
      module: serverModule,
      transport: TransportKind.ipc,
      options: { execArgv: ["--nolazy", "--inspect=6009"] },
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "tt" },
      { scheme: "untitled", language: "tt" },
      { scheme: "file", language: "ttx" },
      { scheme: "untitled", language: "ttx" },
    ],
    synchronize: {
      // Re-validate when the locally built compiler appears or changes.
      fileEvents: workspace.createFileSystemWatcher(
        "**/target/{debug,release}/ttc",
      ),
    },
  };

  client = new LanguageClient(
    "tt",
    "tt Language Server",
    serverOptions,
    clientOptions,
  );
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
