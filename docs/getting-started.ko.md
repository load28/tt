# tt 설치

권장 설치 명령에는 [Bun](https://bun.sh/)이 필요합니다. TT 패키지와 프리빌트
`ttc` 컴파일러는 npm 공식 배포본을 설치합니다.

`ttc`는 TypeScript 7을 구동하며, 그것을 **프로젝트 자신의 `node_modules`**
에서만 찾습니다 — 빌드가 쓰는 바로 그 패키지입니다. TT와 함께 설치합니다.

```sh
bun add -d typescript@7.1.0-dev.20260826.1
```

내보낼 환경 변수도, 에디터 전용 설정도 없습니다. 확장은 프로젝트의 `ttc`를
띄우고, 그 `ttc`가 프로젝트의 TypeScript를 찾습니다. 선언 방출
(`ttc --types`와 에디터의 `.tt.d.ts` 사이드카)과 content mapper(`.ts`가
`.tt`을 디스크 생성물 없이 import하는 경로)는 TypeScript 7.1에서 들어온
API를 사용하며, 나머지 기능은 7.0에서 모두 동작합니다.

## 자동 설치

첫 `.tt` 모듈이 포함된 Vite + TypeScript 프로젝트를 만듭니다.

```sh
bunx @load28/create-tt@0.3.0 my-app
cd my-app
bun run dev
```

기존 TypeScript 프로젝트에는 다음과 같이 추가합니다.

```sh
cd existing-project
bunx @load28/create-tt@0.3.0 init
bun run tt:check
```

`init`은 `package.json`에서 Vite, Rollup, Rolldown, webpack, Rspack,
esbuild, Farm을 감지합니다. `0.3.0` 설치기는 `@load28/tt-lang`과
`@load28/unplugin-tt`의 Stable 채널과 TT용 스크립트를 추가합니다. npm TypeScript
패키지는 추가하지 않으므로 위와 같이 `typescript@7.1.0-dev.20260826.1`을 직접 설치합니다.
선언형 설정을 쓰는 번들러에는 기존 설정을 합성하는
`tt.*.config.mjs` 래퍼를 생성하며 사용자 설정 소스는 고치지 않습니다. 생성된
래퍼는 `bun run tt:dev` 또는 `bun run tt:build`로 사용합니다. esbuild 빌드
스크립트는 임의의 JavaScript이므로 자동 수정하지 않고 추가할 플러그인 한 줄을
출력합니다.

비대화형 실행 옵션은 다음과 같습니다.

```sh
bunx @load28/create-tt@0.3.0 init --bundler vite
bunx @load28/create-tt@0.3.0 init --bundler none
bunx @load28/create-tt@0.3.0 init --no-install
bunx @load28/create-tt@0.3.0 init --package-manager bun
```

새 프로젝트는 항상 Bun을 사용합니다. 기존 프로젝트는 `--package-manager`를
지정하지 않으면 `packageManager` 필드나 lockfile의 패키지 매니저를 유지합니다.

## 저장소 개발: 로컬 빌드 패키지를 레지스트리로 설치

컴파일러를 개발할 때는 의존성을 `file:` 경로로 바꾸지 않고 실제 npm 호환
레지스트리를 사용합니다. 첫 번째 터미널에서 Verdaccio를 실행합니다.

```sh
bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873
```

현재 OS와 CPU용 `ttc`를 빌드하고 `@load28/tt-lang`, 플랫폼 바이너리, `@load28/unplugin-tt`,
`@load28/create-tt`을 로컬 레지스트리에 게시합니다.

```sh
bun scripts/publish-local-registry.mjs http://127.0.0.1:4873
```

게시 스크립트가 다음 형태의 정확한 생성 명령을 출력합니다.

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx @load28/create-tt@latest my-app --registry http://127.0.0.1:4873
```

`--registry`는 같은 레지스트리를 `bun install`에 넘기고 새 프로젝트의
`bunfig.toml`에도 기록합니다. Verdaccio는 로컬에서 빌드한 TT 패키지를 제공하고
Vite 같은 외부 패키지는 프록시합니다.

## 컴파일러 수동 설치

컴파일러와 그것이 구동할 TypeScript를 설치합니다.

```sh
bun add -d @load28/tt-lang@next typescript@7.1.0-dev.20260826.1
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
bun add -d @load28/tt-lang@next @load28/unplugin-tt@next
```

각 번들러의 plugins 배열 맨 앞에 `tt()`을 넣습니다.

```ts
// Vite: vite.config.ts
import tt from "@load28/unplugin-tt/vite";
export default { plugins: [tt()] };

// Rollup: rollup.config.js
import tt from "@load28/unplugin-tt/rollup";
export default { plugins: [tt()] };

// Rolldown: rolldown.config.js
import tt from "@load28/unplugin-tt/rolldown";
export default { plugins: [tt()] };

// webpack: webpack.config.mjs
import tt from "@load28/unplugin-tt/webpack";
export default { plugins: [tt()] };

// Rspack: rspack.config.mjs
import tt from "@load28/unplugin-tt/rspack";
export default { plugins: [tt()] };

// Farm: farm.config.ts
import tt from "@load28/unplugin-tt/farm";
export default { plugins: [tt()] };
```

esbuild는 JavaScript API에서 연결합니다.

```js
import { build } from "esbuild";
import tt from "@load28/unplugin-tt/esbuild";

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

두 번째 VSIX가 에디터용 TypeScript 자체입니다. tt은 성능을 위해
TypeScript 7(네이티브 컴파일러)을 구동하고, 그중 7.1 라인의 API —
`.ts`가 `.tt`을 디스크 생성물 없이 import하게 하는 content mapper — 를
사용합니다. Marketplace의 TypeScript 확장은 아직 이 API를 싣지 않았으므로,
같은 나이틀리 릴리스의 빌드로 TypeScript 확장을 직접 설치하고 `useTsgo`
설정을 켜서 TypeScript 확장이 최신 API로 `.ts`/`.tsx`를 서빙하게 합니다.

```jsonc
// .vscode/settings.json (또는 사용자 설정)
"js/ts.experimental.useTsgo": true,
"typescript.experimental.useTsgo": true
```

TypeScript 7.1이 정식 릴리스되면 확장은 공식 루트로 설치하면 되고,
`useTsgo` 설정도 필요 없어집니다.

프로젝트 루트를 엽니다. 확장은 프로젝트가 설치한 `ttc`를 띄우고 그 `ttc`는
프로젝트가 설치한 TypeScript를 쓰므로, 에디터와 빌드가 다른 TypeScript를 쓰는
상황이 성립하지 않습니다.
