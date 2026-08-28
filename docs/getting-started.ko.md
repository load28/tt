# tt 설치

권장 설치 명령에는 [Bun](https://bun.sh/)이 필요합니다.

## 자동 설치

첫 `.tt` 모듈이 포함된 Vite + TypeScript 프로젝트를 만듭니다.

```sh
bunx @openload28/create-tt@next my-app
cd my-app
bun run dev
```

기존 TypeScript 프로젝트에는 다음과 같이 추가합니다.

```sh
cd existing-project
bunx @openload28/create-tt@next init
bun run tt:check
```

`init`이 수행하는 작업:

- Vite, Rollup, Rolldown, webpack, Rspack, esbuild, Farm 감지
- `@openload28/tt-lang`, `@openload28/unplugin-tt`, TypeScript, TT 스크립트 추가
- 선언형 번들러용 `tt.*.config.mjs` 생성
- esbuild에 추가할 플러그인 코드 출력

비대화형 실행 옵션은 다음과 같습니다.

```sh
bunx @openload28/create-tt@next init --bundler vite
bunx @openload28/create-tt@next init --bundler none
bunx @openload28/create-tt@next init --no-install
bunx @openload28/create-tt@next init --package-manager bun
```

새 프로젝트는 Bun을 사용합니다. 기존 프로젝트는 `packageManager` 필드나
lockfile의 패키지 매니저를 유지합니다.

## 컴파일러 수동 설치

컴파일러와 그것이 구동할 TypeScript를 설치합니다.

```sh
bun add -d @openload28/tt-lang@next typescript@7.1.0-dev.20260826.1
```

소스는 `src/**/*.tt` 또는 `src/**/*.ttx`에 두고 다음 스크립트를 추가합니다.

```json
{
  "scripts": {
    "build:tt": "ttc -o .tt-build src",
    "check:tt": "ttc --check-types src"
  }
}
```

`bun run build:tt`은 `.tt-build`에 일반 `.ts`/`.tsx` 파일을 만듭니다. 기존
TypeScript 빌드의 입력을 이 트리로 지정합니다. `.tt-build/`와 `.tt-types/`는
`.gitignore`에 넣고 생성 파일은 직접 수정하지 않습니다.

## 번들러 수동 설치

컴파일러와 함께 직접 소스 플러그인을 설치합니다.

```sh
bun add -d @openload28/tt-lang@next @openload28/unplugin-tt@next
```

각 번들러의 plugins 배열 맨 앞에 `tt()`을 넣습니다.

```ts
// Vite: vite.config.ts
import tt from "@openload28/unplugin-tt/vite";
export default { plugins: [tt()] };

// Rollup: rollup.config.js
import tt from "@openload28/unplugin-tt/rollup";
export default { plugins: [tt()] };

// Rolldown: rolldown.config.js
import tt from "@openload28/unplugin-tt/rolldown";
export default { plugins: [tt()] };

// webpack: webpack.config.mjs
import tt from "@openload28/unplugin-tt/webpack";
export default { plugins: [tt()] };

// Rspack: rspack.config.mjs
import tt from "@openload28/unplugin-tt/rspack";
export default { plugins: [tt()] };

// Farm: farm.config.ts
import tt from "@openload28/unplugin-tt/farm";
export default { plugins: [tt()] };
```

esbuild는 JavaScript API에서 연결합니다.

```js
import { build } from "esbuild";
import tt from "@openload28/unplugin-tt/esbuild";

await build({ entryPoints: ["src/main.tt"], bundle: true, plugins: [tt()] });
```

플러그인을 쓰면 번들러가 `.tt`과 `.ttx`를 직접 읽습니다. 변환 전용 번들러는
TypeScript 타입 검사를 대신하지 않으므로 `ttc --check-types src` 검사는 별도로
유지합니다.

## 기존 파일 마이그레이션

tt 문법을 사용할 파일만 `.ts`에서 `.tt`로, `.tsx`에서 `.ttx`로 바꿉니다.
상대 import에는 `.tt` 또는 `.ttx` 확장자를 명시합니다. 일반 TypeScript와 TSX는
그대로 두고 점진적으로 옮길 수 있습니다.

```ts
import { render } from "./notice.tt";
```

기존 빌드 전에 `bunx ttc --check-types src`를 실행합니다. 에디터 진단과 탐색에는
최신 [GitHub Releases](https://github.com/load28/tt/releases) pre-release에서
`tt-language-<버전>.vsix`를 내려받습니다. VS Code 명령 팔레트의
**Extensions: Install from VSIX...**를 실행하거나 다음 명령으로 설치합니다.

```sh
code --install-extension ./tt-language-<버전>.vsix
code --install-extension ./tt-typescript-preview-<버전>-<플랫폼>.vsix
```

두 VSIX를 설치한 뒤 에디터에서 TypeScript 7을 사용하도록 설정합니다.

```jsonc
// .vscode/settings.json (또는 사용자 설정)
"js/ts.experimental.useTsgo": true,
"typescript.experimental.useTsgo": true
```

VS Code에서 프로젝트 루트를 엽니다.
