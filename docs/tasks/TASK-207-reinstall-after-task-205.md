# TASK-207: TASK-205 로컬 개발 환경 재설치

- **상태**: 완료
- **시작일**: 2026-08-24
- **완료일**: 2026-08-24
- **커밋**: 이 커밋

## 목적

TASK-205가 포함된 현재 main을 공식 `scripts/setup` 절차로 다시 빌드하고 로컬
`ttc`와 VS Code 확장에 설치한다.

## 범위

- 포함: 저장된 typescript-go checkout 재사용, toolchain·release ttc·VS Code 확장 빌드 및 재설치, 설치 결과 확인
- 제외: git pull, typescript-go 소스 변경, npm·GitHub 원격 배포

## 의사결정

### 결정 1: 저장된 checkout 설정을 인자 없이 재사용한다

- **상황**: `.tt-dev/toolchain.json`은 로컬 typescript-go checkout을 가리키고,
  `scripts/setup`은 인자가 없을 때 이 설정을 재사용하도록 설계됐다.
- **검토한 대안**: `--tsgo-root`로 같은 경로를 다시 지정하면 결과는 같지만 저장
  설정의 후속 실행 경로를 검증하지 못한다. npm 모드로 바꾸면 현재 개발 환경의
  toolchain 계약이 달라진다.
- **선택과 근거**: 사용자가 요청한 스크립트 재설치를 `./scripts/setup`으로
  실행했다. 로그에서 저장된 checkout 경로가 선택됐음을 확인했다.

## 작업 내역

- 2026-08-24: main과 origin/main이 동기화된 상태에서
  `.tt-dev/toolchain.json`의 checkout 경로를 확인했다.
- 2026-08-24: `./scripts/setup`을 실행했다. typescript-go native compiler와 API
  client를 빌드하고, `cargo clean`으로 6,460개·1.6 GiB의 기존 산출물을 제거한
  뒤 release `ttc`를 새로 빌드했다.
- 2026-08-24: VS Code 확장을 VSIX로 패키징했다. 기존
  `tt-lang.tt-language`를 제거하고 새 확장을 설치했다.
- 2026-08-24: release `ttc` 버전, typescript-go 실행 파일·API 파일, 설치된
  VS Code 확장 버전을 각각 확인했다.

## 이슈 및 해결

### 이슈 1: VSIX 패키징과 설치에서 경고가 출력됐다

- **증상**: 패키징은 519개 파일 중 JavaScript 269개를 번들링하라는 성능 경고를
  냈다. 설치 과정은 Node의 `url.parse()` deprecation 경고를 냈다.
- **원인**: 현재 확장은 client/server 의존성과 출력 파일을 번들 없이 VSIX에
  포함한다. 설치 CLI의 의존 코드가 deprecated Node API를 사용한다.
- **해결**: 두 메시지는 종료 코드와 설치 결과에 영향을 주지 않았다. 스크립트가
  성공했고 확장 목록에서 설치 버전을 확인했다. 이번 재설치 범위에서는 제품
  파일을 변경하지 않았다.

## 검증

- [x] `./scripts/setup`
- [x] `target/release/ttc --version` — `0.3.0-dev.6`
- [x] 설치된 VS Code 확장 버전 확인 — `tt-lang.tt-language@0.1.0`

## 결과

TASK-205가 포함된 release `ttc`와 VS Code 확장을 공식 setup 절차로 다시
설치했다. 로컬 typescript-go compiler·API 산출물도 현재 checkout에서 다시
빌드됐다.
