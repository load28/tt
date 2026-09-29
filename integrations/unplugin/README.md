# @openload28/unplugin-tt

`.tt`과 `.ttx` 모듈을 번들러에서 그대로 import합니다. 중간 `.ts`/`.tsx` 트리를 만들지 않고,
번들러가 소스를 직접 읽습니다.

[unplugin](https://github.com/unjs/unplugin) 기반이라 구현은 하나이고 번들러별
서브패스로 갈라져 나갑니다.

```ts
// vite.config.ts
import { defineConfig } from "vite";
import tt from "@openload28/unplugin-tt/vite";

export default defineConfig({ plugins: [tt()] });
```

```ts
// src/main.ts — 평범한 TypeScript
import { Notice, render } from "./notice.tt";
```

컴파일러는 [`@openload28/tt-lang`](https://www.npmjs.com/package/@openload28/tt-lang)을 함께
설치하면 자동으로 찾습니다 (`npm install --save-dev @openload28/tt-lang@next @openload28/unplugin-tt@next`). @openload28/tt-lang이
없으면 PATH의 `ttc`(`cargo install --path .`)로 폴백합니다.

## 서브패스

| import | 상태 |
|--------|------|
| `@openload28/unplugin-tt/vite`, `/rollup`, `/rolldown`, `/webpack`, `/rspack`, `/esbuild`, `/farm` | Every published adapter is constructed in the integration gate; the shared hooks compile `.tt`/`.ttx`, serve standard modules, return source maps, and forward diagnostics |

`@openload28/unplugin-tt`를 그대로 import하면 `unplugin` 객체와 `vitePlugin`·
`esbuildPlugin` 같은 이름들이 나옵니다.

## 동작

| 단계 | 하는 일 |
|------|---------|
| `resolveId` | Resolves a `.tt`/`.ttx` specifier to its file and returns that path with a query ending in `lang.ts` or `lang.tsx`, keeping any query the import already had. `@tt/std`, `@tt/std/option`, and `@tt/std/result` become virtual module ids |
| `load` | Returns what `ttc -p --rewrite-imports off` prints for the file, answered by the build's one `ttc --server` session (see [One compiler session per build](#one-compiler-session-per-build)). The standard library and the pipeline runtime use `ttc --emit-std types\|option\|result\|runtime` output per module |

The `lang.ts`/`lang.tsx` query ending **routes the module through the host's own
TypeScript handling**, so the plugin does not transpile anything itself. The part
before the query stays the real `.tt` file, so tools that strip the query (Vite's
`cleanUrl`, its worker and asset handling, and its dependency scanner) find a file
on disk. esbuild's `load` can return only JavaScript, so that path names the `ts`
or `tsx` loader that matches the source.

`--rewrite-imports off`인 것도 의도입니다. 지정자 재작성은 미리 컴파일하는
파이프라인을 위한 기능이고, 여기서는 `.tt`이 그대로 남아야 이 플러그인이
다음 모듈도 잡습니다.

컴파일 에러는 ttc의 진단이 그대로 빌드 에러가 됩니다.

```
[@openload28/unplugin-tt] src/notice.tt:22:16: match on variant Notice is not exhaustive:
              missing "Warn" (add the missing arms or a final `_` arm)
```

## 옵션

| 옵션 | 기본값 | 설명 |
|------|--------|------|
| `compiler` | 설치된 `@openload28/tt-lang`의 바이너리, 없으면 `"ttc"` | ttc 실행 파일 경로 |
| `verify` | `true` | `false`면 `--no-verify`를 넘겨 방출물 자가 검사를 생략합니다 |
| `sourcemap` | `true` | Set to `false` to omit the source map returned to the bundler. The map names each source by its absolute path, so every bundler resolves it to the `.tt` file whatever its output directory |

타입 선언(`index.d.ts`와 서브패스별 `.d.ts`)을 함께 싣습니다 — 소비자가
`vite.config.ts`를 타입 검사에 넣어도 `tt()`의 옵션이 그대로 검사됩니다.

## Type checking uses the content mapper

The bundler plugin handles runtime loading. TypeScript 7.1+ resolves `.tt` and
`.ttx` imports through the compiler package's content mapper without sidecar
files. Declare the mapper at the top level of `tsconfig.json`, then allow the
TypeScript CLI to start it:

```jsonc
{
  "contentMappers": [
    { "package": "@openload28/tt-lang", "extensions": [".tt", ".ttx"] }
  ]
}
```

```sh
tsc -p tsconfig.json --runExternalCode
```

See the [installation guide](../../docs/getting-started.md) for the complete
project setup. Use `ttc --types` only with legacy TypeScript hosts that cannot
load content mappers.

## 알려진 제약

- `enforce: "pre"`는 Rollup·esbuild에서 무시됩니다 (unplugin 문서의 지원 훅
  표). 그 두 곳에서는 플러그인 순서를 직접 앞에 두세요.
- `resolveId`는 Rspack·Rsbuild에서 최신 버전을 요구합니다.

## Module ids

A `.tt` module's id is its file path plus a query, for example
`/project/src/lib.tt?lang.ts`. An import's own query is kept in front of the
marker, so Vite's worker script request `./worker.tt?worker_file&type=module`
compiles to `/project/src/worker.tt?worker_file&type=module&lang.ts`. Imports that
ask Vite for something other than the module (`?raw`, `?url`, `?worker`,
`?sharedworker`, and their combinations) are left to Vite, which returns the raw
source, the file URL, or a worker constructor.

Vite applies `config.plugins` to workers only in development. To bundle a `.tt`
worker in a production build, register the plugin in `worker.plugins` as well:

```ts
export default defineConfig({
  plugins: [tt()],
  worker: { plugins: () => [tt()] },
});
```

Vite's dependency scanner reads the file in front of the query. The Vite adapter
adds `.tt` and `.ttx` to `optimizeDeps.extensions` and registers a scanner plugin
that compiles them: `optimizeDeps.esbuildOptions.plugins` before Vite 8, and
`optimizeDeps.rolldownOptions.plugins` on Rolldown-powered Vite (detected with
`this.meta.rolldownVersion`). As a result, bare dependencies imported only from
`.tt` files are pre-bundled when the dev server starts.

Rollup derives default chunk names from the id, so an entry or dynamic import of
`main.tt` is named `main.tt_lang`. Name entries with an input object
(`input: { main: "src/main.tt" }`) when the output file name matters.

The standard library has no file on disk, so `@tt/std`, `@tt/std/option`,
`@tt/std/result`, and `@tt/runtime` resolve to the virtual ids
`virtual:unplugin-tt/std/types.ts`, `…/option.ts`, `…/result.ts`, and
`…/runtime.ts`. The ids contain no file system path, so they stay the same when
Vite normalizes Windows separators. They keep the `virtual:` namespace but not
the `\0` prefix, because the host's TypeScript transform must still process
them; Vite serves them in development as `/@id/virtual:unplugin-tt/std/…`.

## Resolution and dependency invalidation

Bare package imports ending in `.tt` or `.ttx` use the bundler's resolver, including
package exports and external decisions. Vite/Rollup-compatible hooks use
`this.resolve`; esbuild uses `build.resolve`.

The plugin asks the compiler session for each module's dependencies (the answer
`ttc --dependencies` prints) and registers those paths with the bundler. Vite
invalidates consuming modules when a type-only import or compiler configuration
changes, including when HMR is disabled.

## One compiler session per build

With TypeScript installed, ttc refines the storage annotations it generates with
the project's types, which opens the whole TypeScript project. A `ttc -p` per
module would open it again for every module, so a build of N modules would open
it N times. Instead, the plugin starts one `ttc --server` session on the first
module it loads and asks it for every module: `print` answers exactly what
`ttc -p` prints for the same file and flags, and `dependencies` exactly what
`ttc --dependencies` prints. The project opens once per build.

On a generated 400-module project, loading 20 modules took 209 s with a process per
module and 10 s through the session; loading all 400 through the session took 39 s.

- **One session per compiler and working directory.** Requests from concurrent
  loads carry ids; the session answers them in order.
- **Restart once.** If the session process ends while a request is waiting, the
  request is asked once more of a fresh session. If that one ends too, the load
  fails with the exit status and what the compiler wrote on stderr. The plugin
  never falls back to another way of compiling.
- **Shutdown.** The session ends on Rollup's, Rolldown's and Vite's
  `closeBundle` (after the last rebuild in watch mode, on `closeWatcher`), on
  webpack's and Rspack's `shutdown`, and on esbuild's `onDispose`. An idle
  session never keeps the bundler's process alive, and it exits when that process
  does.

When overriding `compiler`, use a compiler version whose `--server` answers
`print` and `dependencies`.
