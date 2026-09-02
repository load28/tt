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
| `@openload28/unplugin-tt/vite` | 예제(`tt-interop`)로 검증 |
| `@openload28/unplugin-tt/esbuild` | 번들·실행 검증 |
| `@openload28/unplugin-tt/rollup`, `/rolldown`, `/webpack`, `/rspack`, `/farm` | unplugin이 제공하는 어댑터 — 미검증 |

`@openload28/unplugin-tt`를 그대로 import하면 `unplugin` 객체와 `vitePlugin`·
`esbuildPlugin` 같은 이름들이 나옵니다.

## 동작

| 단계 | 하는 일 |
|------|---------|
| `resolveId` | `.tt`/`.ttx` 지정자를 파일 경로로 풀고 각각 `.ts`/`.tsx`를 덧붙인 가상 id를 돌려줍니다. `@tt/std`, `@tt/std/option`, `@tt/std/result`는 각각 가상 모듈 id로 바꿉니다 |
| `load` | `ttc -p --rewrite-imports off`의 출력을 돌려줍니다. 표준 라이브러리와 파이프 런타임은 모듈별 `ttc --emit-std types|option|result|runtime` 출력을 사용합니다 |

id에 `.ts` 또는 `.tsx`를 붙이는 이유는 **호스트의 TypeScript 처리에 그대로 태우기**
위해서입니다. 덕분에 플러그인이 변환을 직접 하지 않습니다. 다만 esbuild의
`load`는 JavaScript만 반환할 수 있어서, 그 경로에는 소스 종류에 맞는 `ts`/`tsx`
loader를 명시합니다.

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
