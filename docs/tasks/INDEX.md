# 태스크 인덱스

이 파일은 이 저장소 모든 작업의 **단일 진실 소스**입니다.
모든 작업은 태스크 문서로 관리·기록되어야 합니다 — 규칙은 [`AGENTS.md`](../../AGENTS.md)의
"태스크 관리 규칙" 참조. 새 태스크는 [`TEMPLATE.md`](./TEMPLATE.md)로 만듭니다.

## 태스크 목록

| ID | 제목 | 상태 | 시작일 | 완료일 | 문서 |
|----|------|------|--------|--------|------|
| TASK-001 | 태스크 관리 체계 및 CLAUDE.md 구축 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-001](./TASK-001-task-system-and-claude-docs.md) |
| TASK-002 | transform.rs 모듈 분리 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-002](./TASK-002-transform-module-split.md) |
| TASK-003 | 포매팅 표준화 및 린트 게이트 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-003](./TASK-003-formatting-and-lint-gates.md) |
| TASK-004 | 패키지 메타데이터·라이선스·거버넌스 문서 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-004](./TASK-004-governance-and-metadata.md) |
| TASK-005 | CI 파이프라인 구축 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-005](./TASK-005-ci-pipeline.md) |
| TASK-006 | 태스크 기록 상세화 규칙 도입 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-006](./TASK-006-detailed-task-records.md) |
| TASK-007 | 라이브러리 수준 문서화 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-007](./TASK-007-library-level-docs.md) |
| TASK-008 | README 라이브러리 스타일 재작성 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-008](./TASK-008-library-style-readme.md) |
| TASK-009 | 레퍼런스 문서 사용자 관점 단순화 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-009](./TASK-009-user-facing-reference-simplify.md) |
| TASK-010 | swc 스타일 컴파일러 아키텍처 재구성 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-010](./TASK-010-swc-style-compiler-architecture.md) |
| TASK-011 | Option/Result 표준 라이브러리와 내장 enum 소진성 검사 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-011](./TASK-011-option-result-stdlib.md) |
| TASK-012 | try 문 — Rust 스타일 에러 전파 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-012](./TASK-012-try-error-propagation.md) |
| TASK-013 | 파이프라인 연산자 `\|>` 설계 제안 | 완료 | 2026-08-16 | 2026-08-16 | [TASK-013](./TASK-013-pipeline-operator-proposal.md) |
| TASK-014 | match or-패턴 (`A \| B => ...`) | 완료 | 2026-08-16 | 2026-08-16 | [TASK-014](./TASK-014-match-or-patterns.md) |
| TASK-015 | match 가드 (`패턴 if 조건 => ...`) | 완료 | 2026-08-16 | 2026-08-16 | [TASK-015](./TASK-015-match-guards.md) |
| TASK-016 | let-else 문 (`const Tag(x) = 식 else { ... };`) | 완료 | 2026-08-16 | 2026-08-16 | [TASK-016](./TASK-016-let-else.md) |
| TASK-017 | std 콤비네이터 확장 (zip/flatten/transpose/collect/fromPromise) | 완료 | 2026-08-16 | 2026-08-16 | [TASK-017](./TASK-017-std-combinators.md) |
| TASK-018 | VSCode 언어 서비스 (LSP 확장) | 완료 | 2026-08-16 | 2026-08-16 | [TASK-018](./TASK-018-vscode-language-service.md) |
| TASK-019 | 모듈 그래프 설계 제안 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-019](./TASK-019-module-graph-proposal.md) |
| TASK-020 | import 지정자 재작성 (모듈 그래프 1단계) | 완료 | 2026-08-17 | 2026-08-17 | [TASK-020](./TASK-020-import-specifier-rewrite.md) |
| TASK-021 | swc 스타일 렉서 도입 — 토큰 기반 파서 재구성 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-021](./TASK-021-lexer-token-parser.md) |
| TASK-022 | 선언 수집과 프로젝트 단위 소진성 검사 (모듈 그래프 2단계) | 완료 | 2026-08-17 | 2026-08-17 | [TASK-022](./TASK-022-project-exhaustiveness.md) |
| TASK-023 | 심볼 인터페이스와 언어 서버 크로스 파일 기능 (모듈 그래프 3단계) | 완료 | 2026-08-17 | 2026-08-17 | [TASK-023](./TASK-023-symbol-interface.md) |
| TASK-024 | 언어 서버 TS 위임 — rl 파일 전반의 심볼 이동 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-024](./TASK-024-ts-language-service-delegation.md) |
| TASK-025 | TS 위임 확장 — 자동완성·참조 찾기·이름 변경 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-025](./TASK-025-ts-completion-references-rename.md) |
| TASK-026 | 프로젝트 프론트엔드 역할 변경 설계 제안 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-026](./TASK-026-project-front-end.md) |
| TASK-027 | `--rewrite-imports ts` 모드 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-027](./TASK-027-rewrite-imports-ts-mode.md) |
| TASK-028 | TypeScript 사이드카 선언 설계 제안 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-028](./TASK-028-ts-sidecar-declarations.md) |
| TASK-029 | `rlc --sidecar` — 에디터 사이드카 생성 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-029](./TASK-029-sidecar-mode.md) |
| TASK-030 | 저장 시 사이드카 갱신 (언어 서버) | 완료 | 2026-08-17 | 2026-08-17 | [TASK-030](./TASK-030-sidecar-on-save.md) |
| TASK-031 | 사이드카가 소스 트리를 어지럽히지 않게 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-031](./TASK-031-sidecar-visibility.md) |
| TASK-032 | 사이드카를 별도 트리로 — 소스/출력 완전 분리 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-032](./TASK-032-sidecar-out-dir.md) |
| TASK-033 | vite-plugin-rl — 번들러가 `.rl`을 직접 읽는다 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-033](./TASK-033-vite-plugin.md) |
| TASK-034 | `rlc -w` — 감시 모드 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-034](./TASK-034-watch-mode.md) |
| TASK-035 | `@rl/std` — 표준 라이브러리 지정자와 자동 방출 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-035](./TASK-035-std-bare-specifier.md) |
| TASK-036 | 타입·빌드 파이프라인 통일 계획 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-036](./TASK-036-unified-type-build-plan.md) |
| TASK-037 | CLI 통일 — 기본 build 모드와 `--types` 파이프라인 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-037](./TASK-037-cli-unification.md) |
| TASK-038 | unplugin-rl — 번들러 어댑터 통합 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-038](./TASK-038-unplugin.md) |
| TASK-039 | 예제를 표준 라이브러리 정식 참조 방식으로 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-039](./TASK-039-examples-use-std-specifier.md) |
| TASK-040 | `--types`를 메모리 방출로 — 캐시 트리 제거 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-040](./TASK-040-in-memory-types.md) |
| TASK-041 | 레퍼런스 문서를 읽는 문서로 정리 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-041](./TASK-041-reference-docs-slimming.md) |
| TASK-042 | TS↔Rust 타입 추론 격차 분석과 rl 기능 제안 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-042](./TASK-042-type-inference-gaps-proposal.md) |
| TASK-043 | 파이프라인 연산자 `\|>` 구현 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-043](./TASK-043-pipeline-operator-impl.md) |
| TASK-044 | 튜플 match — 다중 스크루티니와 곱집합 소진성 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-044](./TASK-044-tuple-match.md) |
| TASK-045 | 중첩 패턴 — `Ok(value: Some(v))` | 완료 | 2026-08-17 | 2026-08-17 | [TASK-045](./TASK-045-nested-patterns.md) |
| TASK-046 | `if let` 문 — 조건부 값 추출 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-046](./TASK-046-if-let.md) |
| TASK-047 | 에디터 `.rl` 파일 아이콘 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-047](./TASK-047-file-icon.md) |
| TASK-048 | npm 패키징 — `npm install rl-lang`으로 rlc 설치 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-048](./TASK-048-npm-packaging.md) |
| TASK-049 | 버저닝 가이드 수립 (CLAUDE.md) | 완료 | 2026-08-17 | 2026-08-17 | [TASK-049](./TASK-049-versioning-guide.md) |
| TASK-050 | 방출 매핑 기반 TS 위임 — 컴파일 출력을 가상 문서로 | 완료 | 2026-08-17 | 2026-08-17 | [TASK-050](./TASK-050-emit-map-virtual-ts.md) |
| TASK-051 | TypeScript 7 릴리스로 인한 CI 복구 — `--types` 진단과 게이트 고정 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-051](./TASK-051-typescript-7-ci-fix.md) |
| TASK-052 | AI 코딩 도구용 rl 사용 가이드 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-052](./TASK-052-ai-usage-guide.md) |
| TASK-053 | AI 가이드 압축 — 파일 사이즈 최소화 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-053](./TASK-053-ai-guide-compression.md) |
| TASK-054 | AI 가이드 확장(설치·업데이트·워크플로)과 `rlc help` 주제별 헬프 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-054](./TASK-054-ai-guide-full-and-cli-help.md) |
| TASK-055 | 에디터에서 `Option`/`Result`·파이프라인이 `any`로 추론되는 문제 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-055](./TASK-055-editor-any-inference-fix.md) |
| TASK-056 | 대규모 코드베이스 대비 컴파일러 성능 개선 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-056](./TASK-056-large-codebase-performance.md) |
| TASK-057 | 타입 에러를 `.rl` 원본 위치로 — `--types` 위치 매핑과 에디터 TS 진단 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-057](./TASK-057-type-errors-at-source.md) |
| TASK-058 | 에디터 타입 진단 오탐 — lib.d.ts 없는 프로그램이 지어낸 `TS2488` | 완료 | 2026-08-18 | 2026-08-18 | [TASK-058](./TASK-058-editor-type-environment-guard.md) |
| TASK-059 | VSIX 패키징 검증 — TypeScript `lib*.d.ts` 누락 확정과 절차 문서화 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-059](./TASK-059-vsix-lib-packaging.md) |
| TASK-060 | 에디터 타입 진단이 워크스페이스 `tsconfig.json`을 반영할지 검토 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-060](./TASK-060-editor-tsconfig-adoption.md) |
| TASK-061 | 검증 안 된 lib 폴백 제거와 패키징 CI 게이트 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-061](./TASK-061-lib-fallback-trim-and-ci-guard.md) |
| TASK-062 | 표준 라이브러리 메서드가 자동완성에 안 나오는 문제 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-062](./TASK-062-editor-member-completion.md) |
| TASK-063 | `flow` — 함수 합성 (포인트프리 파이프라인) | 완료 | 2026-08-18 | 2026-08-18 | [TASK-063](./TASK-063-flow-composition.md) |
| TASK-064 | `result` 계산 블록 — Result 바인딩 `<-` | 완료 | 2026-08-18 | 2026-08-18 | [TASK-064](./TASK-064-result-computation-block.md) |
| TASK-065 | `Result` 타입 모델 개선 — `Ok<T>` / `Err<E>` 변종 타입 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-065](./TASK-065-result-variant-types.md) |
| TASK-066 | `Result` 에러 타입 합성 — `andThen`/`andThenP`의 유니언 누적 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-066](./TASK-066-result-error-union-composition.md) |
| TASK-067 | unplugin-rl 타입 선언 — 소비자의 `vite.config.ts` 타입 검사 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-067](./TASK-067-unplugin-type-declarations.md) |
| TASK-068 | AGENTS.md와 리터럴 패턴 설계 문서 편입 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-068](./TASK-068-agents-guide-and-literal-pattern-design.md) |
| TASK-069 | match 리터럴 패턴 — 문자열/숫자/불리언 | 완료 | 2026-08-18 | 2026-08-18 | [TASK-069](./TASK-069-match-literal-patterns.md) |
| TASK-070 | `val` — 변경 금지 바인딩 수식자 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-070](./TASK-070-val-binding-modifier.md) |
| TASK-071 | `val` 변경 메서드 판정을 타입 기반으로 — 이름 기준 오탐 제거 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-071](./TASK-071-val-typed-mutation.md) |
| TASK-072 | 에디터에 타입 기반 `val` 진단 노출 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-072](./TASK-072-val-typed-diagnostics-in-editor.md) |
| TASK-073 | TypeScript 네이티브 백엔드 — 단일 프로젝트 그래프 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-073](./TASK-073-typescript-native-backend.md) |
| TASK-074 | 에디터를 네이티브 백엔드로 — 사이드카 규약 통일 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-074](./TASK-074-editor-native-backend.md) |
| TASK-075 | `types_host.mjs` 제거 — 타입 경로 단일화 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-075](./TASK-075-retire-types-host.md) |
| TASK-076 | 증분 검사 — 서버를 살려 두고 스냅샷만 갱신 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-076](./TASK-076-incremental-native-check.md) |
| TASK-077 | 언어 서비스를 네이티브 백엔드로 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-077](./TASK-077-language-service-native.md) |
| TASK-078 | `result` 바인딩 선언 위치 emit-map | 완료 | 2026-08-19 | 2026-08-19 | [TASK-078](./TASK-078-result-binding-emit-map.md) |
| TASK-079 | tsgo native backend 전환 설계 문서 편입 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-079](./TASK-079-tsgo-backend-design-record.md) |
| TASK-080 | 구문이 도입하는 이름의 emit-map — `try`·패턴 바인딩 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-080](./TASK-080-binding-emit-map.md) |
| TASK-081 | let-else 발산 판정 — 객체 리터럴 반환 오탐 수정 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-081](./TASK-081-let-else-divergence-object-literal.md) |
| TASK-082 | TS7 semantic unification 제안서 검토와 개선 계획 확정 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-082](./TASK-082-ts7-semantic-unification-review.md) |
| TASK-083 | host 질의 batch화와 mutator 정책의 판정 시점 이동 — 동작 불변 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-083](./TASK-083-host-batch-and-mutator-policy.md) |
| TASK-084 | 큰 프로젝트에서의 host 스케일 — 응답 파이프 데드락 수정과 공유 타입 메모이제이션 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-084](./TASK-084-host-large-project-scale.md) |
| TASK-085 | call-capability 검사의 callee를 symbol identity로 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-085](./TASK-085-callee-symbol-pairing.md) |
| TASK-086 | Project/Snapshot 기반 Language Engine 아키텍처 재구성 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-086](./TASK-086-engine-architecture.md) |
| TASK-087 | LSP 재설계 — 엔진 어댑터화와 TsgoProject 제거 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-087](./TASK-087-lsp-engine-adapter.md) |
| TASK-088 | CI native 테스트 guard 개수 갱신 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-088](./TASK-088-ci-native-guard-count.md) |
| TASK-089 | 에디터 TS 진단의 glue 위치 보정 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-089](./TASK-089-editor-diagnostic-glue-fallback.md) |
| TASK-090 | 로컬 개발 setup — `scripts/setup`과 toolchain 자동 연결 | 완료 | 2026-08-19 | 2026-08-19 | [TASK-090](./TASK-090-local-dev-setup.md) |
| TASK-091 | 에디터 typed 진단 지연 단축 — debounce 축소와 예약 위치 이동 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-091](./TASK-091-editor-typed-check-latency.md) |
| TASK-092 | VS Code 하이라이팅 재구축 — TS 문법 전체 확장 생성 파이프라인 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-092](./TASK-092-vscode-grammar-full-extension.md) |
| TASK-093 | 엔진 semantic tokens — 파서 소유 분류를 LSP 표준으로 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-093](./TASK-093-semantic-tokens.md) |
| TASK-094 | 마크다운 코드 펜스 rl 하이라이팅 — injection 문법 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-094](./TASK-094-markdown-fence-highlighting.md) |
| TASK-095 | 펜스 injection을 MDX로 확장 — Svelte 확장과 동등하게 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-095](./TASK-095-mdx-fence-injection.md) |
| TASK-096 | match 구조 개선 — typed pattern analysis (MatchAnalysis) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-096](./TASK-096-match-typed-pattern-analysis.md) |
| TASK-097 | sema 소진성을 MatchAnalysis 위로 — coverage 단일 원천 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-097](./TASK-097-exhaustiveness-on-match-analysis.md) |
| TASK-098 | 폴백 타입의 한계 실측 — nested·tuple·제네릭 인스턴스화 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-098](./TASK-098-fallback-type-instantiation-survey.md) |
| TASK-099 | 기록 위생 — INDEX 상태 정합성과 미등록 후속 등록 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-099](./TASK-099-records-hygiene.md) |
| TASK-100 | TS enum을 scrutinee로 쓴 `match`를 rl 진단으로 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-100](./TASK-100-ts-enum-match-diagnostic.md) |
| TASK-101 | rl 구문의 Rust 수준 분석 격차 검토와 개선 계획 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-101](./TASK-101-rust-parity-review.md) |
| TASK-102 | 패턴 사이트 일반화와 이름 해석 진단 (P1+P2) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-102](./TASK-102-pattern-sites-and-resolution.md) |
| TASK-103 | 소진성을 usefulness 알고리즘으로 (P5) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-103](./TASK-103-usefulness-exhaustiveness.md) |
| TASK-104 | 진단 앵커와 진단 번역 (P4 계층 1·3) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-104](./TASK-104-diagnostic-anchors-and-translation.md) |
| TASK-105 | rl 이름의 semantic 표면을 엔진으로 (P3 1/2) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-105](./TASK-105-rl-name-surface.md) |
| TASK-106 | 패턴 자리 자동완성 (P3 2/2) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-106](./TASK-106-pattern-completions.md) |
| TASK-107 | 에디터가 엔진의 rl 표면을 쓴다 (P3 3/3) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-107](./TASK-107-editor-adopts-engine-surface.md) |
| TASK-108 | typed 소진성도 usefulness 위로 (P4 계층 2, 1/2) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-108](./TASK-108-typed-coverage-on-usefulness.md) |
| TASK-109 | 중첩 열의 알파벳을 체커에게 묻는다 (P4 계층 2, 2/2) | 완료 | 2026-08-20 | 2026-08-20 | [TASK-109](./TASK-109-payload-alphabet-query.md) |
| TASK-110 | witness를 붙여 넣을 수 있는 패턴으로 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-110](./TASK-110-pasteable-witnesses.md) |
| TASK-111 | 튜플 match의 typed 소진성 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-111](./TASK-111-typed-tuple-coverage.md) |
| TASK-112 | 키워드를 빠뜨린 `result` 바인딩 진단 | 완료 | 2026-08-20 | 2026-08-20 | [TASK-112](./TASK-112-result-missing-keyword.md) |
| TASK-113 | 도달 불가 arm을 에디터 힌트로 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-113](./TASK-113-unreachable-arm-hints.md) |
| TASK-114 | GAP-6 기록 정정 — 중첩 내부 소진성 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-114](./TASK-114-parity-gap6-correction.md) |
| TASK-115 | CI 복구 — 새 clippy 린트와 테스트 개수 guard | 완료 | 2026-08-21 | 2026-08-21 | [TASK-115](./TASK-115-ci-green.md) |
| TASK-116 | 진단을 정확한 구문 범위로 — 스팬 있는 에러 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-116](./TASK-116-diagnostic-spans.md) |
| TASK-117 | 한 파일의 rl 에러를 여러 개 보고한다 (문제 기록) | 완료 | — | 2026-08-21 | [TASK-117](./TASK-117-multiple-rl-diagnostics.md) |
| TASK-118 | 타입 에러 문안에서 구조적 타입을 선언 이름으로 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-118](./TASK-118-named-error-types-in-messages.md) |
| TASK-119 | 컴파일러 중심부 전환 설계 (umbrella) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-119](./TASK-119-compiler-core-design.md) |
| TASK-120 | 구조화 다중 진단 — Phase 0 (TASK-117 흡수) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-120](./TASK-120-structured-diagnostics.md) |
| TASK-121 | HIR 기반 — Phase 1 (ID 체계, arena, lowering, source map) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-121](./TASK-121-hir-foundation.md) |
| TASK-122 | 선언 수집과 이름 해석 — Phase 2 (resolve) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-122](./TASK-122-name-resolution.md) |
| TASK-123 | 이름 해석의 단일화 — analysis가 resolver를 소비 (Phase 3 1/2) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-123](./TASK-123-resolver-owns-names.md) |
| TASK-124 | typed facts의 경계 확정 — 백엔드 실패 강등 (Phase 4) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-124](./TASK-124-typed-facts-degradation.md) |
| TASK-125 | flow IR — let-else 발산을 제어 흐름으로 (Phase 5 1/n) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-125](./TASK-125-flow-ir.md) |
| TASK-126 | cross-snapshot semantic cache와 의존성 무효화 (Phase 6 1/n) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-126](./TASK-126-semantic-cache.md) |
| TASK-127 | 서버 declarations 표면 — 에디터 shadow 대체 재료 (D6 1/2) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-127](./TASK-127-declarations-surface.md) |
| TASK-128 | 에디터가 declarations를 소비, shadow 삭제 (D6 2/2) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-128](./TASK-128-editor-consumes-declarations.md) |
| TASK-129 | Table 구축을 resolver 위로 (Phase 3 2/2, D5 종결) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-129](./TASK-129-table-from-resolver.md) |
| TASK-130 | 에디터 semantic API가 semantic cache를 소비 (Phase 6 2/n) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-130](./TASK-130-editor-consumes-semantic-cache.md) |
| TASK-131 | try·let-else 배치를 flow 사실로 (Phase 5 2/n) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-131](./TASK-131-try-placement-on-flow.md) |
| TASK-132 | result 바인딩의 early-return 범위 확정 (Phase 5 3/n) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-132](./TASK-132-result-binding-scope.md) |
| TASK-133 | let-else·`if let`의 or-패턴 (GAP-6 마지막 항목) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-133](./TASK-133-or-patterns-in-let-else-if-let.md) |
| TASK-134 | `if let` 배치도 flow 사실로 (TASK-131 잔여) | 완료 | 2026-08-21 | 2026-08-21 | [TASK-134](./TASK-134-if-let-placement-on-flow.md) |
| TASK-135 | 인라인 문맥의 배치 상속 (Place) + verify 원인-결과 억제 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-135](./TASK-135-inline-context-inheritance.md) |
| TASK-136 | codegen 회복 출력 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-136](./TASK-136-codegen-recovery-output.md) |
| TASK-137 | 에디터 타입 진단 투영 가드 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-137](./TASK-137-editor-diagnostic-projection-guard.md) |
| TASK-138 | Snapshot 부분 실패 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-138](./TASK-138-snapshot-partial-failure.md) |
| TASK-139 | 파서 Claim 커밋 모델 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-139](./TASK-139-parser-claim-model.md) |
| TASK-140 | 진단 병합과 원본 이름 보존 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-140](./TASK-140-diagnostic-merge.md) |
| TASK-141 | codex 브랜치 정밀 진단 이식 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-141](./TASK-141-port-codex-diagnostics.md) |
| TASK-142 | 오류 구문 단위 typed projection 복구 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-142](./TASK-142-local-projection-recovery.md) |
| TASK-143 | 에디터가 typed recovery projection을 공유 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-143](./TASK-143-editor-recovery-projection.md) |
| TASK-144 | 구조화된 타입 진단과 공통 렌더링 | 완료 | 2026-08-21 | 2026-08-21 | [TASK-144](./TASK-144-structured-type-diagnostics.md) |
| TASK-145 | 타입 진단 범위의 구문 anchor 폴백 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-145](./TASK-145-diagnostic-range-fallback.md) |
| TASK-146 | 에디터 잠정 진단의 원인 소유권 정리 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-146](./TASK-146-editor-provisional-diagnostic-ownership.md) |
| TASK-147 | semantic 패턴 진단의 완전한 primary span | 완료 | 2026-08-22 | 2026-08-22 | [TASK-147](./TASK-147-semantic-pattern-primary-spans.md) |
| TASK-148 | README 전면 개편과 언어 스펙 문서 정리 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-148](./TASK-148-readme-rework.md) |
| TASK-149 | 공식 GitHub Pages 홈페이지 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-149](./TASK-149-official-github-pages.md) |
| TASK-150 | 전체 rl 구문의 Lowered IR 아키텍처 전환 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-150](./TASK-150-lowered-ir-architecture.md) |
| TASK-151 | Core IR backend 의미 판단 제거 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-151](./TASK-151-core-ir-backend-boundary.md) |
| TASK-152 | TanStack Start 기반 공식 홈페이지와 rl 하이라이팅 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-152](./TASK-152-tanstack-start-website.md) |
| TASK-153 | 홈페이지 오버뷰 조합 예제 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-153](./TASK-153-overview-composed-example.md) |
| TASK-154 | SWC 전체 프로그램 lowering 아키텍처 설계 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-154](./TASK-154-swc-program-lowering-architecture.md) |
| TASK-155 | SWC ProgramSyntax shadow 계층 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-155](./TASK-155-swc-program-syntax-shadow.md) |
| TASK-156 | SWC 평가 문맥 프로토콜 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-156](./TASK-156-swc-evaluation-context.md) |
| TASK-157 | shadow Evaluation IR과 CFG validator | 완료 | 2026-08-22 | 2026-08-22 | [TASK-157](./TASK-157-shadow-evaluation-ir.md) |
| TASK-158 | source-preserving target과 printer | 완료 | 2026-08-22 | 2026-08-22 | [TASK-158](./TASK-158-source-preserving-target.md) |
| TASK-159 | 값 continuation과 direct-return lowering (Phase 4 1/n) | 완료 | 2026-08-22 | 2026-08-22 | [TASK-159](./TASK-159-value-continuation-direct-return.md) |
| TASK-160 | SWC whole-owner 기반 RL→TS 최적 lowering | 완료 | 2026-08-22 | 2026-08-24 | [TASK-160](./TASK-160-whole-owner-ast-lowering.md) |
| TASK-161 | SWC와 TypeScript 7 책임 경계 주석 | 완료 | 2026-08-22 | 2026-08-22 | [TASK-161](./TASK-161-swc-ts7-responsibility-comments.md) |
| TASK-162 | 사용된 표준 라이브러리 멤버만 방출 | 완료 | 2026-08-22 | 2026-08-23 | [TASK-162](./TASK-162-stdlib-member-pruning.md) |
| TASK-163 | `.rlx` — TSX 위의 rl 문법과 React 도구 체인 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-163](./TASK-163-rlx-tsx-support.md) |
| TASK-164 | 웹사이트 `.rlx` React 가이드 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-164](./TASK-164-website-rlx-guide.md) |
| TASK-165 | `.rlx` 에디터 기능 parity | 완료 | 2026-08-23 | 2026-08-23 | [TASK-165](./TASK-165-rlx-editor-parity.md) |
| TASK-166 | 에디터 파일 아이콘을 웹사이트 로고로 통일 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-166](./TASK-166-editor-file-icon-refresh.md) |
| TASK-167 | 프로젝트 설치 CLI와 통합 가이드 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-167](./TASK-167-project-installer-and-guides.md) |
| TASK-168 | README와 웹사이트 개발 단계 안내 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-168](./TASK-168-development-status-notice.md) |
| TASK-169 | rl을 tt로 전면 개명 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-169](./TASK-169-rename-rl-to-tt.md) |
| TASK-170 | 로컬 저장소와 Git 원격을 tt로 전환 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-170](./TASK-170-local-repository-rename.md) |
| TASK-171 | TT 로고 자산 교정 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-171](./TASK-171-correct-tt-logo-assets.md) |
| TASK-172 | flow CFG 완성 — 모든 TypeScript 문 형태의 정확한 발산 판정 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-172](./TASK-172-complete-flow-cfg.md) |
| TASK-173 | tt 구문의 발산 판정 — flow의 마지막 근사 제거 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-173](./TASK-173-tt-construct-divergence.md) |
| TASK-174 | 웹사이트 문구 정확성 전면 점검 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-174](./TASK-174-website-copy-accuracy.md) |
| TASK-175 | npm·VS Code 개발 버전 자동 배포 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-175](./TASK-175-dev-registry-publishing.md) |
| TASK-176 | VS Code 개발 확장을 GitHub Release로 배포 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-176](./TASK-176-github-vsix-dev-releases.md) |
| TASK-177 | 버전 채널 기반 자동 배포 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-177](./TASK-177-version-routed-releases.md) |
| TASK-178 | Windows npm 플랫폼 패키지 이름 변경 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-178](./TASK-178-rename-windows-npm-package.md) |
| TASK-179 | release 0.3.0-dev.1 | 취소 | 2026-08-23 | 2026-08-23 | [TASK-179](./TASK-179-release-0.3.0-dev.1.md) |
| TASK-180 | malformed projection의 codegen 진입 차단 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-180](./TASK-180-lsp-test-notification-race.md) |
| TASK-181 | release 0.3.0-dev.2 | 취소 | 2026-08-23 | 2026-08-23 | [TASK-181](./TASK-181-release-0.3.0-dev.2.md) |
| TASK-182 | Windows npm 패키지의 고유 이름 적용 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-182-unique-windows-npm-package.md](./TASK-182-unique-windows-npm-package.md) |
| TASK-183 | release 0.3.0-dev.3 | 취소 | 2026-08-23 | 2026-08-23 | [TASK-183-release-0.3.0-dev.3.md](./TASK-183-release-0.3.0-dev.3.md) |
| TASK-184 | npm publish 로컬 경로 명시 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-184-explicit-npm-publish-paths.md](./TASK-184-explicit-npm-publish-paths.md) |
| TASK-185 | release 0.3.0-dev.4 | 취소 | 2026-08-23 | 2026-08-23 | [TASK-185-release-0.3.0-dev.4.md](./TASK-185-release-0.3.0-dev.4.md) |
| TASK-186 | npm 패키지를 `@load28` 스코프로 통일 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-186-unique-user-package-names.md](./TASK-186-unique-user-package-names.md) |
| TASK-187 | release 0.3.0-dev.5 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-187-release-0.3.0-dev.5.md](./TASK-187-release-0.3.0-dev.5.md) |
| TASK-188 | 설치 문서 역할 분리와 typescript-go 소스 연동 안내 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-188-installation-document-boundaries.md](./TASK-188-installation-document-boundaries.md) |
| TASK-189 | GitHub VSIX 확장 설치 안내 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-189-github-vsix-installation-guide.md](./TASK-189-github-vsix-installation-guide.md) |
| TASK-190 | 수동 설치의 typescript-go 선행 조건 명시 | 완료 | 2026-08-23 | 2026-08-23 | [TASK-190-manual-install-tsgo-prerequisite.md](./TASK-190-manual-install-tsgo-prerequisite.md) |
| TASK-191 | 설치 페이지 셸 명령어 하이라이팅 | 완료 | 2026-08-23 | 2026-08-24 | [TASK-191-install-section-shell-highlighting.md](./TASK-191-install-section-shell-highlighting.md) |
| TASK-192 | 소비자 설치를 소스 빌드 tsgo로 단일화 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-192-source-only-tsgo-consumer-setup.md](./TASK-192-source-only-tsgo-consumer-setup.md) |
| TASK-193 | release 0.3.0-dev.6 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-193-release-0.3.0-dev.6.md](./TASK-193-release-0.3.0-dev.6.md) |
| TASK-194 | projection parse 실패의 원인 분류 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-194-projection-parse-source-cause.md](./TASK-194-projection-parse-source-cause.md) |
| TASK-195 | tt 제작 동기 글 (영문·한글) | 완료 | 2026-08-24 | 2026-08-24 | [TASK-195-motivation-essay.md](./TASK-195-motivation-essay.md) |
| TASK-196 | 웹사이트 배경 글 페이지 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-196-website-background-essay.md](./TASK-196-website-background-essay.md) |
| TASK-197 | tt 제작 동기 글 한글판 문장 개정 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-197-korean-essay-rewrite.md](./TASK-197-korean-essay-rewrite.md) |
| TASK-198 | 방출 코드 가독성 — 레이아웃 계층과 그룹핑 규칙 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-198-generated-code-layout.md](./TASK-198-generated-code-layout.md) |
| TASK-199 | block arm의 도달 불가능한 폴스루 제거 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-199-unreachable-arm-fallthrough.md](./TASK-199-unreachable-arm-fallthrough.md) |
| TASK-200 | 일반 compile 출력용 표준 source map | 완료 | 2026-08-24 | 2026-08-24 | [TASK-200-standard-source-map.md](./TASK-200-standard-source-map.md) |
| TASK-201 | 로컬 재설치와 tour 패키지 갱신 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-201-local-reinstall-and-tour-package-refresh.md](./TASK-201-local-reinstall-and-tour-package-refresh.md) |
| TASK-202 | 에디터 진단의 세대 단위 원자적 발행 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-202-editor-diagnostic-generations.md](./TASK-202-editor-diagnostic-generations.md) |
| TASK-203 | TASK-202 로컬 개발 환경 재설치 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-203-reinstall-after-diagnostic-fix.md](./TASK-203-reinstall-after-diagnostic-fix.md) |
| TASK-204 | VS Code 전체 도구 체인 테스트 실패 조사 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-204](./TASK-204-vscode-full-toolchain-test-failures.md) |
| TASK-205 | VS Code 전체 도구 체인 테스트 복구 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-205](./TASK-205-vscode-full-toolchain-test-fixes.md) |
| TASK-206 | setup의 Cargo 전체 정리와 tsgo 자식 환경 주입 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-206](./TASK-206-setup-clean-cargo-builds.md) |
| TASK-207 | TASK-205 로컬 개발 환경 재설치 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-207](./TASK-207-reinstall-after-task-205.md) |
| TASK-208 | 설치 작업을 태스크 관리에서 제외 | 완료 | 2026-08-24 | 2026-08-24 | [TASK-208](./TASK-208-exclude-installation-from-task-tracking.md) |
| TASK-209 | output verify의 문자열 기반 tt 구문 추정 제거 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-209](./TASK-209-structured-unclaimed-tt-candidates.md) |
| TASK-210 | 잔여 아키텍처·codegen 개선 묶음 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-210](./TASK-210-deferred-architecture-and-codegen-cleanup.md) |
| TASK-211 | AI 에이전트의 로컬 개발 환경 탐색 표준화 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-211](./TASK-211-agent-local-setup-discovery.md) |
| TASK-212 | 파이프 헬퍼의 전역 스크립트 충돌 제거 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-212](./TASK-212-pipeline-helper-global-collision.md) |
| TASK-213 | 진단 표현 계층 — 렌더러, 코드 노출, 구조화된 제안 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-213](./TASK-213-diagnostic-presentation.md) |
| TASK-214 | 패닉 안전망 — 컴파일러 버그를 버그로 보고 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-214](./TASK-214-panic-safety-net.md) |
| TASK-215 | 스냅샷 픽스처 — 방출과 진단 전체 고정 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-215](./TASK-215-snapshot-fixtures.md) |
| TASK-216 | exhaustiveness 수정을 컴파일러 저작 편집으로 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-216](./TASK-216-compiler-authored-arm-edits.md) |
| TASK-217 | VS Code 확장 completion 테스트 불안정성 조사 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-217](./TASK-217-completion-test-instability.md) |
| TASK-218 | 남은 규칙의 수정 조언을 Suggestion으로 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-218](./TASK-218-suggestions-for-remaining-rules.md) |
| TASK-219 | 방출 코드 가독성 — 블록 암 들여쓰기와 런타임 import 위치 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-219](./TASK-219-generated-code-readability.md) |
| TASK-220 | 진단 렌더러의 ANSI 색상 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-220](./TASK-220-diagnostic-colour.md) |
| TASK-221 | unwrap/expect 감사 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-221](./TASK-221-panic-surface-audit.md) |
| TASK-222 | 부하 시 integration 스위트의 간헐적 실패 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-222](./TASK-222-integration-suite-flakiness.md) |
| TASK-223 | 실세계 코퍼스 차등 테스트와 퍼징 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-223](./TASK-223-corpus-and-fuzzing.md) |
| TASK-224 | 커버리지 측정과 게이트 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-224](./TASK-224-coverage-gate.md) |
| TASK-225 | 성능 벤치마크와 회귀 게이트 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-225](./TASK-225-performance-benchmarks.md) |
| TASK-226 | 로컬과 CI의 Rust 툴체인 격차 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-226](./TASK-226-local-ci-toolchain-parity.md) |
| TASK-227 | CI를 로컬 실행으로 옮기고 GitHub 실행은 수동으로 | 진행 중 | 2026-08-25 | — | [TASK-227](./TASK-227-local-only-ci.md) |
| TASK-228 | 부분 스냅샷에서 정상 파일의 tt 진단이 사라짐 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-228](./TASK-228-partial-snapshot-diagnostics.md) |
| TASK-229 | 바인딩 이름 `match`가 tt match로 오인된다 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-229](./TASK-229-match-claimed-as-a-binding-name.md) |
| TASK-230 | release 0.3.0-dev.7 | 완료 | 2026-08-25 | 2026-08-25 | [TASK-230](./TASK-230-release-0.3.0-dev.7.md) |
| TASK-231 | 수동 릴리스 브랜치 기반 Dev·Production 배포 | 완료 | 2026-08-25 | — | [TASK-231](./TASK-231-manual-release-branches.md) |
| TASK-232 | Dev 스탬프 이후 create-tt 채널 테스트 정합성 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-232](./TASK-232-create-tt-release-channel-test.md) |
| TASK-233 | 작업 브랜치 기반 릴리스 계약 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-233](./TASK-233-work-branch-release-contract.md) |
| TASK-234 | TypeScript 방식의 개발·릴리스 모델 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-234](./TASK-234-typescript-release-model.md) |
| TASK-235 | Beta 없는 RC 중심 릴리스 흐름 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-235](./TASK-235-rc-first-release-model.md) |
| TASK-236 | VS Code와 tsgo CI 통합 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-236](./TASK-236-unify-vscode-tsgo-ci.md) |
| TASK-237 | 릴리스 산출물 버전 핫픽스 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-237](./TASK-237-release-artifact-version-hotfix.md) |
| TASK-238 | 전용 GitHub App 기반 릴리스 push | 완료 | 2026-08-26 | 2026-08-26 | [TASK-238](./TASK-238-release-github-app-identity.md) |
| TASK-239 | 릴리스 게시 Environment 승인 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-239](./TASK-239-release-environment-approval.md) |
| TASK-240 | TypeScript 릴리스 명령과 Beta 단계 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-240](./TASK-240-typescript-release-commands.md) |
| TASK-241 | TypeScript 수준 릴리스 운영 가이드 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-241](./TASK-241-typescript-release-guide.md) |
| TASK-242 | 홈페이지 릴리스 가이드 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-242](./TASK-242-website-release-guide.md) |
| TASK-243 | 홈페이지 릴리스 가이드 전용 화면 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-243](./TASK-243-website-release-page.md) |
| TASK-244 | GitHub Discussion 숙의 봇 | 완료 | 2026-08-26 | 2026-08-26 | [TASK-244](./TASK-244-github-deliberation-bots.md) |
| TASK-245 | `variant` 기반 tt 태그드 유니언 선언 | 완료 | 2026-08-26 | 2026-08-27 | [TASK-245](./TASK-245-variant-declarations.md) |
| TASK-246 | GitHub Discussion·PR 숙의 봇 | 완료 | 2026-08-27 | 2026-08-27 | [TASK-246](./TASK-246-pr-review-deliberation.md) |
| TASK-247 | 공식 홈페이지 Google Analytics | 완료 | 2026-08-27 | 2026-08-27 | [TASK-247](./TASK-247-google-analytics.md) |
| TASK-248 | 공식 홈페이지 `variant` 전환 | 완료 | 2026-08-27 | 2026-08-27 | [TASK-248](./TASK-248-website-variant-content.md) |

## 다음 태스크 번호

**TASK-249**
