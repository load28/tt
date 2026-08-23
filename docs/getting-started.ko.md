# rl 설치

권장 설치 명령에는 [Bun](https://bun.sh/)이 필요합니다. 생성된 프로젝트가
프리빌트 `rlc` 컴파일러와 TypeScript 7을 설치하므로 Rust, Go, 별도
typescript-go 체크아웃은 필요하지 않습니다.

## 자동 설치

첫 `.rl` 모듈이 포함된 Vite + TypeScript 프로젝트를 만듭니다.

```sh
bun create rl@latest my-app
cd my-app
bun run dev
```

기존 TypeScript 프로젝트에는 다음과 같이 추가합니다.

```sh
cd existing-project
bun create rl@latest init
bun run rl:check
```

`init`은 `package.json`에서 Vite, Rollup, Rolldown, webpack, Rspack,
esbuild, Farm을 감지합니다. `rl-lang`, TypeScript 7, `unplugin-rl`과 RL용
스크립트를 추가합니다. 선언형 설정을 쓰는 번들러에는 기존 설정을 합성하는
`rl.*.config.mjs` 래퍼를 생성하며 사용자 설정 소스는 고치지 않습니다. 생성된
래퍼는 `bun run rl:dev` 또는 `bun run rl:build`로 사용합니다. esbuild 빌드
스크립트는 임의의 JavaScript이므로 자동 수정하지 않고 추가할 플러그인 한 줄을
출력합니다.

비대화형 실행 옵션은 다음과 같습니다.

```sh
bun create rl@latest init --bundler vite
bun create rl@latest init --bundler none
bun create rl@latest init --no-install
bun create rl@latest init --package-manager bun
```

새 프로젝트는 항상 Bun을 사용합니다. 기존 프로젝트는 `--package-manager`를
지정하지 않으면 `packageManager` 필드나 lockfile의 패키지 매니저를 유지합니다.

### 로컬 빌드 패키지를 레지스트리로 설치

컴파일러를 개발할 때는 의존성을 `file:` 경로로 바꾸지 않고 실제 npm 호환
레지스트리를 사용합니다. 첫 번째 터미널에서 Verdaccio를 실행합니다.

```sh
bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873
```

현재 OS와 CPU용 `rlc`를 빌드하고 `rl-lang`, 플랫폼 바이너리, `unplugin-rl`,
`create-rl`을 로컬 레지스트리에 게시합니다.

```sh
bun scripts/publish-local-registry.mjs http://127.0.0.1:4873
```

게시 스크립트가 다음 형태의 정확한 생성 명령을 출력합니다.

```sh
BUN_CONFIG_REGISTRY=http://127.0.0.1:4873 \
  bunx create-rl@latest my-app --registry http://127.0.0.1:4873
```

`--registry`는 같은 레지스트리를 `bun install`에 넘기고 새 프로젝트의
`bunfig.toml`에도 기록합니다. Verdaccio는 로컬에서 빌드한 RL 패키지를 제공하고
Vite와 TypeScript 같은 외부 패키지는 프록시합니다.

## 컴파일러 수동 설치

컴파일러와 컴파일러가 구동하는 TypeScript를 설치합니다.

```sh
bun add -d rl-lang typescript@7
```

소스는 `src/**/*.rl` 또는 `src/**/*.rlx`에 두고 다음 스크립트를 추가합니다.

```json
{
  "scripts": {
    "build:rl": "rlc -o .rl-build src",
    "check:rl": "rlc --check-types src"
  }
}
```

`bun run build:rl`은 `.rl-build`에 일반 `.ts`/`.tsx` 파일을 만듭니다. 기존
TypeScript 빌드의 입력을 이 트리로 지정합니다. `.rl-build/`와 `.rl-types/`는
`.gitignore`에 넣고 생성 파일은 직접 수정하지 않습니다.

## 번들러 수동 설치

컴파일러와 함께 직접 소스 플러그인을 설치합니다.

```sh
bun add -d rl-lang typescript@7 unplugin-rl
```

각 번들러의 plugins 배열 맨 앞에 `rl()`을 넣습니다.

```ts
// Vite: vite.config.ts
import rl from "unplugin-rl/vite";
export default { plugins: [rl()] };

// Rollup: rollup.config.js
import rl from "unplugin-rl/rollup";
export default { plugins: [rl()] };

// Rolldown: rolldown.config.js
import rl from "unplugin-rl/rolldown";
export default { plugins: [rl()] };

// webpack: webpack.config.mjs
import rl from "unplugin-rl/webpack";
export default { plugins: [rl()] };

// Rspack: rspack.config.mjs
import rl from "unplugin-rl/rspack";
export default { plugins: [rl()] };

// Farm: farm.config.ts
import rl from "unplugin-rl/farm";
export default { plugins: [rl()] };
```

esbuild는 JavaScript API에서 연결합니다.

```js
import { build } from "esbuild";
import rl from "unplugin-rl/esbuild";

await build({ entryPoints: ["src/main.rl"], bundle: true, plugins: [rl()] });
```

플러그인을 쓰면 번들러가 `.rl`과 `.rlx`를 직접 읽습니다. 변환 전용 번들러는
TypeScript 타입 검사를 대신하지 않으므로 `rlc --check-types src` 검사는 별도로
유지합니다.

## 기존 파일 마이그레이션

rl 문법을 사용할 파일만 `.ts`에서 `.rl`로, `.tsx`에서 `.rlx`로 바꿉니다.
상대 import에는 `.rl` 또는 `.rlx` 확장자를 명시합니다. 일반 TypeScript와 TSX는
그대로 두고 점진적으로 옮길 수 있습니다.

```ts
import { render } from "./notice.rl";
```

기존 빌드 전에 `bunx rlc --check-types src`를 실행합니다. 에디터 진단과 탐색에는
RL VS Code 확장을 설치하고 프로젝트 루트를 엽니다.
