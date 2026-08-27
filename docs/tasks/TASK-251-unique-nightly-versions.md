# TASK-251: TypeScript 방식의 고유한 Nightly 버전

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: —

## 목적

같은 날짜의 여러 CI 실행이 동일한 npm 버전을 생성해 Nightly 게시가 충돌하는 문제를
해결한다. TypeScript처럼 실행별 불변 버전을 만들고 npm `next` 태그가 최신 Nightly를
가리키게 한다.

## 범위

- 포함: Nightly 버전 공식, CI 입력, main 개발 버전, 릴리스 문서와 회귀 테스트
- 제외: Beta·RC·Stable·Patch의 버전 공식과 production 승인 정책

## 의사결정

### 결정 1: Nightly는 날짜와 CI 실행 번호로 식별한다

- **상황**: 날짜만 포함한 `X.Y.Z-dev.YYYYMMDD`는 같은 날의 새 빌드를 구분하지 못한다.
- **검토한 대안**: 커밋 SHA / 초 단위 시각 / TypeScript식 날짜와 실행 순번.
- **선택과 근거**: 사람이 읽을 수 있고 같은 날짜에도 고유한
  `X.Y.Z-dev.YYYYMMDD.N`을 사용한다. 동일 CI 재시도는 같은 버전을 유지한다.

### 결정 2: npm `next`는 최신 Nightly를 가리키는 이동식 태그로 유지한다

- **상황**: npm 버전은 게시 후 바꿀 수 없지만 사용자는 고정 버전을 몰라도 최신
  Nightly를 설치할 수 있어야 한다.
- **검토한 대안**: `next`를 버전 문자열로 사용 / 날짜 버전을 직접 설치 / dist-tag 사용.
- **선택과 근거**: 각 빌드는 고유 버전으로 게시하고 `next` dist-tag를 그 버전으로
  이동하는 npm과 TypeScript의 모델을 따른다.

### 결정 3: main은 현재 0.4 개발선을 나타낸다

- **상황**: `release-0.4`가 Beta인데 main은 이미 Stable인 0.3 계열을 기준으로
  Nightly를 만들고 있다.
- **검토한 대안**: 0.3 유지 / 0.4 개발선 / 0.5 개발선.
- **선택과 근거**: TypeScript 절차에서 RC 전 main은 현재 릴리스 개발선이므로
  `0.4.0-dev.1`을 기준 버전으로 사용한다.

## 작업 내역

- 2026-08-27: npm 게시 실패와 TypeScript 공식 빌드·게시 파이프라인을 비교했다.
- 2026-08-27: 작업 브랜치와 TASK-251 문서를 만들었다.
- 2026-08-27: Nightly 버전에 GitHub Actions 실행 번호를 추가하고 버전 계산을 단일
  CI job으로 모아 모든 산출물이 같은 버전을 사용하게 했다.
- 2026-08-27: main 기준 버전을 `0.4.0-dev.1`로 맞추고 npm `next` dist-tag 계약을
  릴리스 문서에 기록했다.
- 2026-08-27: 릴리스 스크립트·워크플로 회귀 테스트와 전체 로컬 CI를 통과했다.

## 이슈 및 해결

### 이슈 1: 같은 날짜의 다른 SHA가 동일한 npm 버전을 생성함

- **증상**: `0.3.0-dev.20260826`이 이전 SHA로 게시되어 예약 Nightly 게시가 실패했다.
- **원인**: Nightly 버전 계산이 커밋 시각의 날짜만 남기고 CI 실행 식별자를 버렸다.
- **해결**: 실행 생성 날짜와 `GITHUB_RUN_NUMBER`로
  `X.Y.Z-dev.YYYYMMDD.N`을 만든다. 생성 날짜는 GitHub API의 run `created_at`에서
  읽으므로 동일 실행을 다른 날 재시도해도 버전이 바뀌지 않는다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `node --test npm/scripts/*.test.mjs packages/create-tt/test/*.test.mjs`
- [x] `./scripts/ci`
- [x] `git diff --check`
- [x] `ci.yml` YAML 파싱

## 결과

`Cargo.toml`·`Cargo.lock`의 main 개발선을 0.4로 맞췄다. 릴리스 버전 계산 스크립트와
CI를 실행별 고유 Nightly 버전에 맞추고, 문서·TASK 인덱스·회귀 테스트를 갱신했다.
