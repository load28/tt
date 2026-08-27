/* --------------------------------------------------------------------------
 * Content mapper auto-registration (TASK-257).
 *
 * TypeScript 7.1's language server holds `.tt`/`.ttx` files virtually when
 * a content mapper serves them, but it can only know to care about those
 * extensions once something tells it — and for a single `.tt` file opened
 * before any tsconfig is discovered, that something has to be an editor
 * extension. The TypeScript "native preview" extension publishes exactly
 * that hook (`ExtensionAPI.registerContentMappers`); this module calls it,
 * so a workspace that installed `@load28/tt-lang` needs no editor
 * configuration at all.
 *
 * The mapper process is the workspace's own install: the package's
 * `binaryPath()` names the exact compiler that `npx ttc` runs, which keeps
 * TASK-256's promise — the editor and the command line provably drive the
 * same ttc — and is why there is no bundled fallback. No install, no
 * inferred-project mapper; the extensions-only registration still lets the
 * server discover configured projects (tsconfig `contentMappers`) for
 * already-open `.tt` documents.
 * ----------------------------------------------------------------------- */
import { createRequire } from "module";
import * as path from "path";
import { Disposable, ExtensionContext, Uri, extensions, workspace } from "vscode";

/** The mapper manifest the TypeScript extension accepts (its
 * `ContentMapperManifest`, structurally). */
interface ContentMapperManifest {
  readonly name: string;
  readonly version?: string;
  readonly exec: readonly string[];
  readonly cwd?: Uri;
}

/** One registration entry (the TypeScript extension's
 * `ContentMapperContribution`, structurally). */
interface ContentMapperContribution {
  readonly extensions: readonly string[];
  readonly inferredProject?: {
    readonly options?: Readonly<Record<string, unknown>>;
    readonly manifest: ContentMapperManifest;
  };
}

/** The slice of the TypeScript extension's exported API this module uses. */
interface TypeScriptExtensionApi {
  registerContentMappers?(
    contributorId: string,
    contributions: readonly ContentMapperContribution[],
  ): Disposable;
}

/** Extension ids the TypeScript 7 language server has shipped under.
 * Microsoft's own first; tt's release-asset build (TASK-258) answers when
 * the marketplace preview predates content mappers. */
const TYPESCRIPT_EXTENSION_IDS = [
  "TypeScriptTeam.native-preview",
  "load28.tt-typescript-preview",
  "typescript.native-preview",
];

/**
 * Registers `.tt`/`.ttx` with the TypeScript extension, when it is
 * installed and exports the hook. Quietly does nothing otherwise — a
 * workspace on the classic tsserver keeps the sidecar path, and nothing
 * here may break activation of the tt language client.
 */
export async function registerContentMappers(context: ExtensionContext): Promise<void> {
  try {
    const api = await typeScriptExtensionApi();
    if (api?.registerContentMappers === undefined) return;
    const registration = api.registerContentMappers(
      "tt-lang.tt-language",
      [contribution()],
    );
    context.subscriptions.push(registration);
  } catch {
    // The TypeScript extension failed to activate or rejected the
    // registration; the tt language client is unaffected.
  }
}

/** The exported API of whichever TypeScript 7 extension is installed. */
async function typeScriptExtensionApi(): Promise<TypeScriptExtensionApi | undefined> {
  for (const id of TYPESCRIPT_EXTENSION_IDS) {
    const extension = extensions.getExtension<TypeScriptExtensionApi>(id);
    if (extension !== undefined) {
      return await extension.activate();
    }
  }
  return undefined;
}

/**
 * The `.tt`/`.ttx` contribution: always the extensions (they trigger
 * configured-project discovery for open documents), plus an
 * inferred-project manifest when a workspace folder has `@load28/tt-lang`
 * installed.
 */
function contribution(): ContentMapperContribution {
  const resolved = workspaceMapper();
  if (resolved === undefined) {
    return { extensions: [".tt", ".ttx"] };
  }
  return {
    extensions: [".tt", ".ttx"],
    inferredProject: { manifest: resolved },
  };
}

/**
 * The mapper manifest of the first workspace folder that installed
 * `@load28/tt-lang`, or `undefined` when none has: the package's own
 * `binaryPath()` answers with the exact compiler `npx ttc` would run
 * (published platform package, `file:` development install, `TTC_BINARY`),
 * and the manifest execs that binary directly — no `node` on the language
 * server's PATH required.
 */
function workspaceMapper(): ContentMapperManifest | undefined {
  for (const folder of workspace.workspaceFolders ?? []) {
    if (folder.uri.scheme !== "file") continue;
    try {
      const require = createRequire(path.join(folder.uri.fsPath, "package.json"));
      const manifestPath = require.resolve("@load28/tt-lang/package.json");
      const loaded = require("@load28/tt-lang") as { binaryPath?: () => string };
      const binary = loaded.binaryPath?.();
      if (binary === undefined || binary === "") continue;
      const version = (require(manifestPath) as { version?: string }).version;
      return {
        name: "@load28/tt-lang",
        version,
        exec: [binary, "--content-mapper"],
        cwd: Uri.file(path.dirname(manifestPath)),
      };
    } catch {
      // This folder has no resolvable install; try the next.
    }
  }
  return undefined;
}
