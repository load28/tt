# TASK-182: Windows npm 패키지의 고유 이름 적용

- **상태**: 진행 중
- **시작일**: 2026-08-23
- **완료일**: —
- **커밋**: —

## 목적

npm 스팸 탐지에 두 차례 차단된 Windows 바이너리 패키지에 프로젝트 고유 이름을
적용해 언스코프 배포 가능성을 높인다.

## 범위

- 포함: Windows npm 게시 이름 변경, optional dependency와 테스트 갱신
- 제외: 런타임 플랫폼 키와 Rust target 변경, npm 스코프 도입

## 의사결정

### 결정 1: `ttlang-native-msvc` 사용

- **상황**: `tt-lang-win32-x64`와 `tt-lang-win32-x64-msvc`가 모두 npm의
  `Package name triggered spam detection`으로 차단됐다. 미등록 여부뿐 아니라
  흔한 플랫폼 패키지 이름과의 유사도를 줄여야 한다.
- **검토한 대안**: SWC·Rollup·Tailwind·Biome·esbuild는 조직 스코프 아래에
  표준 target 이름을 둔다. Lightning CSS는 언스코프 표준 target 이름을 쓰지만
  이미 신뢰가 쌓인 패키지다. `ttc-native-msvc`는 짧지만 `ttc`가 Tencent Cloud
  CLI 이름으로 이미 등록돼 있다. `ttlang-native-msvc`는 표준형보다 덜 익숙하지만
  기존 패키지 접두사와 충돌하지 않고 내부 매핑으로 사용자에게 노출되지 않는다.
- **선택과 근거**: 언스코프 조건을 유지하면서 토큰 조합을 고유하게 만드는
  `ttlang-native-msvc`를 선택했다. 2026-08-23 `npm view`가 E404를 반환해
  미등록 상태를 확인했다.

## 작업 내역

- 2026-08-23: 실패한 Dev Release run `32641598660`의 로그에서 Windows 패키지만
  npm 스팸 탐지 403으로 실패한 사실을 확인했다.
- 2026-08-23: 기존 네이티브 라이브러리 이름과 후보 이름을 npm registry에서
  조회하고 `ttc` 접두사의 기존 소유자도 확인했다.
- 2026-08-23: 플랫폼 manifest와 `tt-lang` optional dependency를
  `ttlang-native-msvc`로 변경하고 README와 패키지 생성 회귀 테스트를 갱신했다.

## 이슈 및 해결

없음.

## 검증

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## 결과

작업 중.
