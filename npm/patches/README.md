# TypeScript editor feature ownership

`typescript-content-mapper-ownership.patch` adds a companion-language ownership
contract to the TypeScript extension at
`microsoft/TypeScript@5739027c9a7df24e27123f453a50c011b37717b6`, the source of this
repository's `typescript@7.1.0-dev.20260826.1` pin.

The release builder applies and tests this patch before packaging the existing
platform VSIX assets. A different source commit fails the build until the patch
is reviewed for that version. The compiler executable remains the pinned npm
artifact; this patch changes the VS Code client only.

The API advertises `contentMapperFeatureOwnership: true`. A registered content
mapper may specify `languageFeatures: "external"`. Native TypeScript retains
document open/change/close synchronization and module resolution, while the
companion owns UI features for those extensions. The same policy handles initial
capabilities, already registered capabilities, custom hover providers, and lease
disposal. Releasing the registration restores native features. Other extensions
and ordinary TypeScript files retain their native providers.

tt claims ownership after its language client starts. Older native extensions
without this capability retain their existing behavior; they can still produce
duplicate UI features. Install the matching newly built pair of extensions to
use this contract. No global installation is modified by the build or tests.

Verification commands, after building the patched upstream extension:

```sh
npm --prefix editors/vscode run compile
VSCODE_TYPESCRIPT_EXTENSION=/absolute/path/to/TypeScript/packages/vscode-typescript \
  TT_EDITOR_TEST_SUITE=ownership node editors/vscode/scripts/test-editor.mjs
VSCODE_TYPESCRIPT_EXTENSION=/absolute/path/to/TypeScript/packages/vscode-typescript \
  node editors/vscode/scripts/test-editor.mjs
```

The ownership suite covers a single completion/hover/diagnostic provider for tt
and ttx, incomplete-arm inference, late delegation, unsaved native consumer
updates, and restoration after disposal. Direct desktop typing evidence is
recorded in [TASK-372](../../docs/tasks/TASK-372-live-typing-feedback.md).
