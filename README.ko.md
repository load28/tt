# tt

[공식 홈페이지](https://load28.github.io/tt/) · [English](./README.md) · [한국어](./README.ko.md)

tt은 TypeScript에 표현력 높은 데이터·제어 흐름 기능을 더하고, 다시 순수 TypeScript로 컴파일하는 작은 언어입니다.

> [!WARNING]
> **개발 중:** tt은 아직 프로덕션 사용을 권장하지 않습니다. 릴리스 사이에 API와 언어 동작이 바뀔 수 있습니다.

```tt
export enum Shape {
  Circle(radius: number),
  Rectangle(width: number, height: number),
  Point,
}

export const area = (shape: Shape): number =>
  match (shape) {
    Circle(radius) => Math.PI * radius ** 2,
    Rectangle(width, height) => width * height,
    Point => 0,
  };
```

모든 유효한 TypeScript 파일은 그대로 유효한 `.tt` 파일이고, 모든 유효한 TSX 파일은 유효한 `.ttx` 파일입니다. tt은 자신이 소유한 구문만 변환하고, 소진되지 않은 match 같은 언어 오류를 직접 보고하며, 런타임 의존성 없는 읽기 쉬운 TypeScript 또는 TSX를 방출합니다.

## 설치와 사용

npm에서 TT의 공식 개발 패키지를 설치합니다. 지원 플랫폼에서는 Rust가 필요하지
않습니다. `ttc`가 사용할 typescript-go는 최신 소스에서 직접 빌드합니다.

```sh
git clone https://github.com/microsoft/typescript-go.git
cd typescript-go
npm ci
mkdir -p built/local
go build -o built/local/tsgo ./cmd/tsgo
npx tsc -b _packages/native-preview
export TTC_TSGO_ROOT="$PWD"
```

`ttc`를 실행하는 환경에 `TTC_TSGO_ROOT`를 유지하고 같은 셸에서 VS Code를
실행합니다. 실행 파일과 API 클라이언트는 같은 체크아웃에서 빌드해야 합니다.

```sh
bunx @load28/create-tt@dev my-app
bunx @load28/create-tt@dev init       # 기존 TypeScript 프로젝트에서 실행
```

새 프로젝트의 자동 설치에는 Bun을 사용합니다. 자동 설치와 번들러별 수동 절차는
[설치 가이드](./docs/getting-started.ko.md)에 정리되어 있습니다.

개발용 VS Code 확장은 Marketplace에 게시하지 않습니다. 최신
[GitHub Releases](https://github.com/load28/tt/releases) pre-release에서
`tt-language-<버전>.vsix`를 내려받고 명령 팔레트에서
**Extensions: Install from VSIX...**를 실행하거나 다음 명령으로 설치합니다.

```sh
code --install-extension ./tt-language-<버전>.vsix
```

컴파일러만 수동으로 설치할 때도 먼저 위의 typescript-go 빌드를 완료하고
`TTC_TSGO_ROOT`를 설정한 상태로 유지해야 합니다.

```sh
bun add -d @load28/tt-lang@dev
```

파일이나 소스 트리를 컴파일하거나, 출력 없이 검사합니다.

```sh
bunx ttc -o build src
bunx ttc --check src
bunx ttc --check-types src
```

`ttc`는 `.tt`을 `.ts`로, `.ttx`를 `.tsx`로 방출합니다. JSX는 그대로 보존하므로 React 프로젝트의 기존 `jsx` 컴파일러 옵션과 JSX 런타임을 계속 사용합니다. `.tt` 또는 `.ttx` 파일을 번들러에서 직접 import하려면 Vite, Rollup, webpack, Rspack, esbuild, Farm을 지원하는 [`@load28/unplugin-tt`](./integrations/unplugin)을 사용하세요.

프리빌트 바이너리는 Linux x64/arm64, macOS x64/arm64, Windows x64를 지원합니다. 다른 플랫폼에서는 소스에서 빌드하세요.

```sh
cargo install --git https://github.com/load28/tt
```

컴파일러 옵션은 `ttc --help`, 내장 언어 가이드는 `ttc help <topic>`으로 확인할 수 있습니다.

## 언어 한눈에 보기

- Rust 스타일 `enum`으로 데이터를 모델링하고, 가드·튜플·리터럴·or-패턴·중첩 패턴을 지원하는 소진적 `match`로 값을 추출합니다.
- `@tt/std`의 `TOption`과 `TResult`를 `try`, `let-else`, `if let`, `result` 블록으로 다룹니다. 트리셰이킹 가능한 연산은 `@tt/std/option`과 `@tt/std/result`에서 가져옵니다.
- `|>`와 `flow`로 값 파이프라인과 함수 파이프라인을 만듭니다.
- 변경을 허용하지 않을 바인딩과 매개변수에는 `val`을 붙입니다.

나머지는 모두 평범한 TypeScript입니다. 기존 TypeScript의 타입, 모듈, 도구, 런타임 동작을 그대로 기반으로 사용합니다.

언어를 만들게 된 배경은 [tt를 만든 이유](./docs/why-tt.ko.md)에 적어 두었습니다.

## tt 개발하기

컴파일러는 작은 공개 API와 `ttc` CLI를 제공하는 Rust 크레이트입니다. Rust 1.98
이상이 필요합니다 — `rust-toolchain.toml`이 고정한, 그리고 모든 빌드가 실제로
검증되는 그 버전입니다. 전체 로컬 환경에는 Bun, Node.js, Go, typescript-go
체크아웃도 필요합니다.

```sh
git clone https://github.com/load28/tt.git
git clone https://github.com/microsoft/typescript-go.git
cd tt
./scripts/setup --tsgo-root ../typescript-go
```

`scripts/setup`은 현재 typescript-go 체크아웃, release `ttc`, VS Code 확장을
빌드합니다. 이후 실행은 `.tt-dev/toolchain.json`의 설정을 재사용하며 두 Git
체크아웃을 자동으로 갱신하지 않습니다.

패키지 사용자가 받는 형태를 그대로 시험하려면 로컬 TT 패키지를 npm 호환
레지스트리에 게시합니다.

```sh
bunx verdaccio@6 --config scripts/verdaccio.local.yaml --listen 127.0.0.1:4873
bun scripts/publish-local-registry.mjs http://127.0.0.1:4873
```

두 번째 명령이 같은 레지스트리를 사용하는 `create-tt` 실행 명령을 출력합니다.
전체 기여자 설정은 [CONTRIBUTING.md](./CONTRIBUTING.md)에 있습니다.

변경을 제출하기 전에 저장소 검증 게이트를 실행합니다.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

컴파일러는 Rust 라이브러리로도 포함할 수 있습니다.

```rust
use ttc::{compile, Options};

let typescript = compile(source, &Options::default())?;
```

아키텍처 기록은 [`docs/design`](./docs/design)에 있습니다.

## 라이선스

[MIT](./LICENSE)
