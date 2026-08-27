# tt 설치

권장 설치 명령에는 [Bun](https://bun.sh/)이 필요합니다. TT 패키지와 프리빌트
`ttc` 컴파일러는 npm 공식 배포본을 설치합니다.

현재 개발 단계에서는 `ttc`가 사용할 도구 체인을 최신
[typescript-go 소스](https://github.com/microsoft/typescript-go)에서 직접
빌드합니다. TT 자체는 npm 공식 배포본을 사용합니다.

```sh
git clone https://github.com/microsoft/typescript-go.git
cd typescript-go
npm ci
mkdir -p built/local
go build -o built/local/tsgo ./cmd/tsgo
npx tsc -b _packages/native-preview
export TTC_TSGO_ROOT="$PWD"
```

`ttc`를 실행하는 모든 셸에 `TTC_TSGO_ROOT`를 유지합니다. TT 에디터 서비스를
사용할 때는 같은 셸에서 VS Code를 실행합니다. 실행 파일과 API 클라이언트의
프로토콜에는 버전 협상이 없으므로 둘은 같은 체크아웃에서 빌드해야 합니다.

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
패키지는 추가하지 않으며 `ttc`는 위에서 빌드한 typescript-go를 사용합니다.
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

이 절차를 사용하기 전에 이 문서 상단의 typescript-go 소스 빌드를 완료하고
`TTC_TSGO_ROOT`가 설정된 상태를 유지해야 합니다. 그다음 컴파일러를 설치합니다.

```sh
bun add -d @load28/tt-lang@0.3.0
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

이 절차에도 빌드된 typescript-go 체크아웃과 `TTC_TSGO_ROOT`가 필요합니다.
컴파일러와 함께 직접 소스 플러그인을 설치합니다.

```sh
bun add -d @load28/tt-lang@0.3.0 @load28/unplugin-tt@0.1.0
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
```

`TTC_TSGO_ROOT`가 설정된 셸에서 프로젝트 루트를 엽니다.
