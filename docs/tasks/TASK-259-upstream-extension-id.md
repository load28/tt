# TASK-259: 프리뷰 확장을 업스트림 ID로 배포

- **상태**: 진행 중
- **시작일**: 2026-08-27
- **완료일**: —
- **커밋**: —

## 목적

TASK-258은 사칭을 피하려고 확장 ID를 `load28.tt-typescript-preview`로
개명했다. 실사용에서 그 대가가 드러났다: VS Code **내장** TypeScript
확장은 `useTsgo` 설정과 함께 **하드코딩된 ID 목록**
(`typescriptteam.vscode-typescript`, `typescriptteam.native-preview`)의
확장이 설치된 경우에만 자기 semantic 서버를 내린다. 개명된 ID로는 내장
서버가 계속 떠서 `.tt` import마다 `TS2307`을 낸다 (rl-tour 실측 —
tsgo 쪽은 매퍼 구성까지 정상인데 내장 서버의 진단이 겹친다).

이 빌드는 마켓플레이스 정식 프리뷰가 content mapper를 실을 때까지의
과도기 산물이고 tt이 제품으로 배포할 물건이 아니므로, 업스트림 ID
그대로 배포한다 (사용자 결정). 같은 ID의 더 높은 버전이 마켓플레이스에
올라오면 자동 업데이트가 이 빌드를 자연 교체한다 — 과도기 종료가
메커니즘으로 보장된다.

## 범위

- 포함: 빌드 스크립트의 신원 상수 되돌림, tt 확장 lookup에서 load28 ID
  제거, 테스트·문서 갱신
- 제외: 진단 끄기(`typescript.validate.enable: false`) 류의 우회 — 내장
  확장의 양보가 정도(正道)다

## 의사결정

### 결정 1: 업스트림 ID 그대로 (TASK-258 결정 2 파기)

- **상황**: 개명 ID로는 내장 확장이 양보하지 않는다. 하드코딩 목록은
  우리가 바꿀 수 없다.
- **검토한 대안**: (A) 내장 진단만 끄기(`validate.enable: false`) —
  자동완성 중복이 남고, 소비 프로젝트마다 설정이 또 늘어난다(이 작업
  전체가 없애려던 것). (B) 내장 확장을 워크스페이스에서 비활성 — 수동
  UI 단계, 자동화 불가. (C) 업스트림 ID 유지 — 내장 양보가 설계대로
  동작하고 설정 0이 회복된다.
- **선택과 근거**: C. 이 VSIX는 무수정 업스트림 빌드에 실행 파일을
  동봉한 것으로, 신원이 가리키는 코드가 실제로 그 신원의 코드다.
  description에 출처(커밋)를 명시하고, 마켓플레이스 정식판이 같은 ID로
  올라오는 순간 자동 업데이트로 대체된다.

## 작업 내역

- 2026-08-27: 내장 확장 번들에서 하드코딩 목록 확인
  (`DE=["typescriptteam.vscode-typescript","typescriptteam.native-preview"]`,
  VS Code 1.134). `EXTENSION_IDENTITY`를 업스트림 값으로 되돌리고, tt
  확장 lookup에서 `load28.tt-typescript-preview` 제거, 테스트 5케이스
  갱신, README·releasing 문서 갱신.

## 이슈 및 해결

없음.

## 검증

- [x] `node --test npm/scripts/*.test.mjs` — 34 pass
- [x] 빌드 스크립트 로컬 실행 → 5개 VSIX 재생성, darwin-arm64 설치 —
      `typescriptteam.native-preview@0.20260826.1`로 등록 확인. 같은 ID의
      수제 빌드에서 내장 양보 + `.tt` import 무오류는 이미 실증된 상태
      (TASK-257/258 로그)이고, 개명 ID로만 재현되던 회귀를 되돌렸다.
- [x] `./scripts/ci npm agents`

## 결과

릴리스 자산의 프리뷰 확장이 업스트림 ID로 배포된다. 내장 TypeScript
확장의 양보가 설계대로 동작해 소비 프로젝트의 추가 설정 없이(useTsgo만)
에디터에서 `.tt` import가 해석된다. 마켓플레이스 정식 프리뷰가 올라오면
같은 ID의 자동 업데이트가 이 빌드를 교체한다.

변경 파일: `npm/scripts/build-ts-preview-vsix.mjs`(신원 상수),
`npm/scripts/build-ts-preview-vsix.test.mjs`,
`editors/vscode/client/src/contentMapper.ts`(load28 ID 제거),
`editors/vscode/README.md`, `docs/releasing.ko.md`, `docs/tasks/INDEX.md`
