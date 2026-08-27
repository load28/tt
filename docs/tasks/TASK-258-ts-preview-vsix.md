# TASK-258: TypeScript 프리뷰 확장 VSIX를 릴리스에 동봉

- **상태**: 진행 중
- **시작일**: 2026-08-27
- **완료일**: —
- **커밋**: —

## 목적

TASK-257로 나이틀리 ttc는 content mapper를 싣지만, 에디터 짝이 없다:
마켓플레이스의 TypeScript Native Preview 확장(v0.20260708.2)은 content
mapper 이전 빌드다. tt이 고정한 TypeScript 나이틀리의 **정확히 같은
커밋**(npm 메타데이터의 gitHead)에서 확장을 빌드해 릴리스 자산으로
동봉하면, 소비자가 에디터에서도 매퍼 경로를 쓸 수 있다. 7.1 정식과 함께
마켓플레이스 판이 매퍼를 실으면 이 동봉은 제거한다.

## 범위

- 포함:
  1. `npm/scripts/build-ts-preview-vsix.mjs` — 핀에서 gitHead를 얻어
     `packages/vscode-typescript`를 빌드하고 플랫폼별 VSIX 5개를 패키징
  2. CI `release-ts-preview` 잡 — 산출물을 30일 아티팩트로 보관,
     `releasable artifact` 게이트에 포함
  3. `Publish Release`가 그 VSIX들을 GitHub Release 자산으로 첨부
  4. 확장 신원 개명 — `load28.tt-typescript-preview` (Microsoft 퍼블리셔
     사칭 방지, Apache-2.0 LICENSE/NOTICE 유지). tt 확장의 자동 등록
     lookup에 이 ID 추가
  5. 문서 — releasing, 확장 README 설치 안내, 이 문서
- 제외:
  - 마켓플레이스 게시 (릴리스 자산 다운로드 + `code --install-extension`)
  - tsgo 자체 빌드 (실행 파일은 같은 버전의 플랫폼 npm 패키지에서 복사 —
    공식 Herebyfile 패키징과 동일한 절차)

## 의사결정

### 결정 1: 배포 위치는 GitHub Release 자산

- **상황**: VSIX를 어디로 내보낼지 (사용자 선택).
- **검토한 대안**: GitHub Packages(npm) — VSIX는 어차피 파일로 설치해야
  해서 단계만 늘고, GH Packages는 읽기에도 인증 토큰을 요구한다.
- **선택과 근거**: Release 자산. `tt-language` VSIX가 이미 가는 곳이고,
  기존 `Publish Release`의 자산 첨부에 얹기만 하면 된다.

### 결정 2: 확장 ID는 `load28.tt-typescript-preview`

- **상황**: 업스트림 ID(`TypeScriptTeam.native-preview`) 그대로 재배포하면
  Microsoft 퍼블리셔를 사칭하는 모양이 된다.
- **검토한 대안**: ID 유지 — 정식판 전환은 매끄럽지만 신원이 거짓이 된다.
- **선택과 근거**: 개명. Apache-2.0은 재배포를 허용하고(LICENSE/NOTICE
  유지), 출처는 README·displayName에 명시한다. tt 확장의 lookup 목록에
  새 ID를 추가하되 공식 확장을 앞순위로 둔다.

### 결정 3: 실행 파일 동봉, 플랫폼별 5개

- **상황**: TASK-257에서 실측: 확장은 패키지 내 `lib/` 실행 파일이
  기본값이고 없으면 활성화가 실패한다 (`getPackagedExePath` throw).
- **검토한 대안**: 동봉 없이 워크스페이스 tsdk 의존 — 활성화 실패 재현.
- **선택과 근거**: 공식 배포판과 같은 구조 — 플랫폼 npm 패키지
  `@typescript/typescript-<os>`의 `lib/`(실행 파일 + 기본 lib.d.ts)를
  복사해 `vsce package --target <플랫폼>` 5개.

### 결정 4: 확장 버전은 핀에서 유도

- **상황**: 재빌드마다 버전이 바뀌면 같은 핀에 다른 산출물이 생긴다.
- **선택과 근거**: `7.1.0-dev.YYYYMMDD.N` → `0.YYYYMMDD.N`. 핀이 움직일
  때만 버전이 움직이고, 같은 날의 나이틀리 재실행은 같은 VSIX를 다시
  만든다 (불변 버전은 npm 패키지의 일이고 이것은 릴리스 자산이다).

### 결정 5: `releasable artifact` 게이트에 포함

- **상황**: 빌드가 microsoft/TypeScript 클론과 npm 레지스트리라는 외부
  의존을 갖는다 — 실패하면 나이틀리 전체가 막힌다.
- **검토한 대안**: best-effort (없으면 자산 생략) — 조용한 반쪽 릴리스.
- **선택과 근거**: 게이트 포함. 이 저장소의 릴리스 계약은 "성공한 CI의
  완전한 산출물 세트"다. GitHub→GitHub 클론과 npm view는 이미 게시
  경로가 쓰는 수준의 의존이다.

## 작업 내역

- 2026-08-27: 태스크 생성. TASK-257의 수동 절차(스파스 클론 → 확장 빌드 →
  플랫폼 lib 동봉 → VSIX)를 검증 근거로 삼는다.
- 2026-08-27: `npm/scripts/build-ts-preview-vsix.mjs` 작성 — 핀 →
  `npm view gitHead` → blobless 클론 → 업스트림 빌드 스크립트로 번들 →
  신원 개명(NOTICE.txt 동봉) → 플랫폼 패키지 `lib/` 복사 →
  `vsce package --target` ×5. 순수 헬퍼(버전 유도·플랫폼 표·신원)는
  export해 `build-ts-preview-vsix.test.mjs` 5케이스로 고정.
- 2026-08-27: CI 배선 — `release-ts-preview` 잡 신설, `releasable
  artifact`의 needs에 포함, `Publish Release`가 `ts-preview/*.vsix`를
  자산으로 첨부. `workflow-publish-paths.test.mjs`에 경로 계약 추가.
- 2026-08-27: tt 확장 lookup에 `load28.tt-typescript-preview` 추가
  (공식 ID 우선). 테스트가 lookup 포함 여부를 고정한다.
- 2026-08-27: 문서 — `docs/releasing.ko.md` Nightly 절,
  `editors/vscode/README.md` 설치 안내(동시 설치 시 명령 ID 충돌 경고).
- 2026-08-27: 로컬 E2E — 스크립트가 5개 VSIX(각 ~10MB)를 생성.
  darwin-arm64를 설치해 `load28.tt-typescript-preview@0.20260826.1`로
  등록되고 동봉 `lib/tsc`가 `7.1.0-dev.20260826.1`을 답하는 것 확인.

## 이슈 및 해결

### 이슈 1: Microsoft Native Preview와 동시 설치 시 명령 ID 충돌

- **증상**: 두 확장이 같은 `typescript.native-preview.*` 명령을 기여한다.
- **원인**: 신원(publisher.name)만 개명하고 업스트림 코드는 무수정으로
  쓰기 때문 — 명령·설정 키는 업스트림 것이 그대로다.
- **해결**: 코드를 고치지 않는 선택을 유지하고(무수정 재배포가 검증·추적
  양쪽에서 안전), 문서에 "한쪽만 설치"를 명시했다. 마켓플레이스 판이
  매퍼를 실으면 이 동봉 자체가 제거된다.

## 검증

- [x] `node --test npm/scripts/*.test.mjs` — 34 pass
- [x] 빌드 스크립트 로컬 실행 — 5개 VSIX 생성, darwin-arm64 설치 스모크
- [ ] `./scripts/ci`
- [ ] CI 실행에서 `release-ts-preview` 아티팩트 확인 (merge 후 nightly)

## 결과

Nightly·정식 릴리스에 `tt-typescript-preview-<버전>-<플랫폼>.vsix` 5개가
자산으로 첨부된다. 소비자는 내려받아 `code --install-extension` 한 번으로
고정 나이틀리와 같은 커밋의 content-mapper 지원 에디터를 얻는다.

변경 파일:

- 신규: `npm/scripts/build-ts-preview-vsix.mjs`,
  `npm/scripts/build-ts-preview-vsix.test.mjs`, 이 문서
- 수정: `.github/workflows/ci.yml`(release-ts-preview 잡·게이트),
  `.github/workflows/release-publish.yml`(다운로드·자산),
  `npm/scripts/workflow-publish-paths.test.mjs`,
  `editors/vscode/client/src/contentMapper.ts`(lookup ID),
  `docs/releasing.ko.md`, `editors/vscode/README.md`, `docs/tasks/INDEX.md`
