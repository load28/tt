# TASK-248: 공식 홈페이지 `variant` 전환

- **상태**: 완료
- **시작일**: 2026-08-27
- **완료일**: 2026-08-27
- **커밋**: —

## 목적

TASK-245에서 확정한 `variant` 언어 계약을 공식 홈페이지의 내비게이션,
레퍼런스, 코드 예제, 배경 글과 검색 색인에 일관되게 반영한다.

## 범위

- 포함: `enum` 토픽을 `variant`로 전환, TTX·모듈 예제와 영문·한글
  문구 갱신, 하이라이트·배경 글·사이트맵 재생성, 정적 빌드 검증
- 제외: 컴파일러·편집기 구현 변경, TypeScript `enum` 자체의 설명 제거

## 의사결정

### 결정 1: 토픽 ID와 공개 URL을 `variant`로 전환

- **상황**: 화면 문구만 바꾸면 canonical URL과 sitemap에 `/enum`이 남는다.
- **검토한 대안**: `enum` ID를 유지하고 표시만 변경 / ID·URL·표시를 함께 변경.
- **선택과 근거**: 언어 표면을 찾는 사용자가 `/variant`라는 단일한 이름을
  보도록 토픽 ID·URL·canonical·sitemap을 함께 전환한다.

### 결정 2: 소스 문서로부터 생성 산출물을 다시 만듦

- **상황**: `docs/why-tt*.md`는 이미 `variant`를 쓰지만 `essay.json`은 이전
  `enum` 상태다.
- **검토한 대안**: JSON 산출물 수동 편집 / 기존 `highlight` 생성기 재실행.
- **선택과 근거**: 생성 계약을 유지하고 누락을 반복하지 않도록 공식 생성기로
  하이라이트, 배경 글, sitemap을 함께 갱신한다.

## 작업 내역

- 2026-08-27: `./scripts/doctor`로 개발 환경을 확인했다.
- 2026-08-27: 최신 `main`에서 홈페이지 콘텐츠·생성물·sitemap의 `enum`
  잔여를 조사했다.
- 2026-08-27: 언어 토픽 ID와 URL을 `variant`로 바꾸고 전용 레퍼런스,
  TTX 예제, 모듈 설명을 TASK-245 계약에 맞춰 갱신했다.
- 2026-08-27: `bun run highlight`로 하이라이트, 배경 글, sitemap을 다시
  생성했다.
- 2026-08-27: 타입 검사와 `/tt/` 기준 정적 빌드를 통과하고 `/variant`·
  `/ko/variant` HTML의 canonical·hreflang·Analytics 태그를 확인했다.

## 이슈 및 해결

없음.

## 검증

- [x] `bun run typecheck` (`website/`)
- [x] `SITE_BASE_PATH=/tt/ bun run build` (`website/`) — 37개 경로 프리렌더
- [x] 생성 HTML·sitemap의 `/variant`·`variant` 계약 확인
- [x] 홈페이지 소스와 생성물의 폐기된 tt `enum` 잔여 검토

## 결과

공식 홈페이지의 언어 내비게이션과 레퍼런스가 `variant`를 사용하고,
TTX·모듈 예제와 영문·한글 배경 글도 현재 컴파일러 계약과 일치한다.
`/variant`·`/ko/variant`가 canonical 경로와 sitemap 항목으로 생성된다.
