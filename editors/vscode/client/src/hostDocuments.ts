import { ExtensionContext, TextDocument, workspace } from "vscode";
import {
  DidChangeTextDocumentNotification,
  DidCloseTextDocumentNotification,
  DidOpenTextDocumentNotification,
  LanguageClient,
  State,
} from "vscode-languageclient/node";

/** Synchronize host buffers without registering tt providers for TypeScript.
 * The engine must see unsaved dependencies; TypeScript still owns their UI. */
export function synchronizeHostDocuments(context: ExtensionContext, client: LanguageClient): void {
  const opened = new Set<string>();
  const isHost = (doc: TextDocument) => doc.uri.scheme === "file" &&
    (doc.languageId === "typescript" || doc.languageId === "typescriptreact");
  const open = (doc: TextDocument) => {
    if (client.state !== State.Running || !isHost(doc)) return;
    const uri = doc.uri.toString();
    if (opened.has(uri)) return;
    opened.add(uri);
    void client.sendNotification(DidOpenTextDocumentNotification.type, {
      textDocument: { uri, languageId: doc.languageId, version: doc.version, text: doc.getText() },
    });
  };
  context.subscriptions.push(
    client.onDidChangeState(({ newState }) => {
      opened.clear();
      if (newState === State.Running) workspace.textDocuments.forEach(open);
    }),
    workspace.onDidOpenTextDocument(open),
    workspace.onDidChangeTextDocument(({ document, contentChanges }) => {
      if (client.state !== State.Running || !isHost(document) || contentChanges.length === 0) return;
      if (!opened.has(document.uri.toString())) { open(document); return; }
      void client.sendNotification(DidChangeTextDocumentNotification.type, {
        textDocument: { uri: document.uri.toString(), version: document.version },
        contentChanges: [{ text: document.getText() }],
      });
    }),
    workspace.onDidCloseTextDocument(doc => {
      if (!opened.delete(doc.uri.toString()) || client.state !== State.Running) return;
      void client.sendNotification(DidCloseTextDocumentNotification.type, {
        textDocument: { uri: doc.uri.toString() },
      });
    }),
  );
}
