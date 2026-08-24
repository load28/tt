# TASK-210: 잔여 아키텍처·codegen 개선 묶음

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: 이 커밋

## 목적

보류됐던 query 세분화와 사용자 동작을 바꾸지 않는 잔여 아키텍처·codegen 개선
후보를 한 태스크에서 구조적으로 정리한다.

## 범위

- 포함:
  - Phase 6 query 세분화(`pattern_analysis`/`flow_body` 단위)
  - 불필요한 receiver temporary 제거와 직접 호출 가능한 `$tt_ap` 최적화
  - 생성 코드의 `do { ... } while (false)`·레이블 블록 이중 중첩 단일화와 임시 이름 정리
  - template interpolation recovery를 두 번 방문하는 parser traversal 중복 제거
- 제외: TASK-209의 output verify 휴리스틱 제거, 새 언어 표면, 필요성이 확인되지 않은 IR 확장

## 의사결정

1. **query 경계는 소비 단위로 이름을 고정한다.** project의 file semantic cache를
   `pattern_analysis_cache`로 명시하고, flow는 완전한 body text를 key로 삼는
   `FlowBodyQueries`로 분리했다. byte offset이나 불안정한 AST ID를 key로 쓰는
   대안은 같은 body의 위치 이동을 재사용하지 못한다. flow 판정은 위치 이동에
   불변이므로 body text가 구조적 key다.
2. **temporary 제거는 `Effects` 증명만 소비한다.** receiver나 pipeline source를
   구문 모양으로 추측하는 대안은 getter, TDZ, mutable read의 평가 순서를 깨뜨린다.
   ProgramSyntax가 `Effects::NONE`으로 증명한 receiver만 capture slot을 생략한다.
   pipeline은 accumulator가 이미 compiler slot에 평가됐거나 inline input의 이동이
   `Effects::NONE`일 때만 직접 호출한다.
3. **conditional match region은 exit target 하나만 갖는다.** fall-through block arm이
   `$tt_b`를 요구하면 모든 expression arm도 `break $tt_b`를 사용한다. 그렇지 않은
   assignment region만 `do { ... } while (false)`를 사용한다. 두 target을 중첩하는
   대안은 같은 제어 의미를 두 구조에 나눠 표현한다.
4. **중첩 parser side table은 공용 walker가 수집한다.** recovery와 unclaimed
   candidate가 각자 AST 재귀를 복제하는 대신 `visit_programs`가 모든 nested
   `Program` 모양을 한 번 정의한다. template interpolation도 소비자별 한 번만
   방문한다.

## 작업 내역

1. `engine/project.rs`의 cache와 entry를 `pattern_analysis` query 이름으로 정리하고
   기존 cross-snapshot invalidation과 editor/typed-pass 공유 계약을 유지했다.
2. `flow/mod.rs`에 `FlowBodyQueries`를 추가하고 let-else와 match block body가 같은
   memoized divergence query를 사용하도록 parser에 연결했다.
3. `program_syntax.rs`가 member receiver의 효과를 evaluation protocol에 전달하도록
   확장했다. Evaluation IR에 `PlannedReceiver::{Captured, Stable}`을 추가하고 target이
   증명된 receiver만 inline으로 재사용하도록 구현했다.
4. pipeline의 직접 호출 가능성을 SWC expression effect로 판정했다. inline 입력
   재배치를 source relocation으로 등록하고, statement-form pipeline은 materialized
   accumulator를 직접 callee에 전달하도록 바꿨다. helper는 필요한 파일에만 남는다.
5. conditional match의 `$tt_b`와 `do-while` 선택을 상호 배타적으로 만들고 외부 exit
   label 이름을 `$tt_y_$tt_vN`에서 `$tt_y_vN`으로 정리했다.
6. parser의 recovery/unclaimed 재귀를 `visit_programs` 하나로 통합했다.
7. query hit, template recovery 단일 수집, receiver slot 생략, 직접 pipeline 호출,
   단일 exit target, 평가 순서와 contextual typing 회귀 테스트를 추가했다.
8. `compiler-core.md`와 `program-lowering.md`의 query·효과 최적화 계약을 갱신했다.
   언어 표면은 바뀌지 않아 `docs/ai/tt.md` 갱신은 필요하지 않았다.

## 이슈 및 해결

1. stable receiver를 source-mapped 조각으로 두 번 출력하자
   `SourceEmittedTwice` validator가 실패했다. 원본 callee 재구성만 source mapping을
   소유하고 `.call`/`.bind`의 두 번째 receiver는 generated literal로 출력했다.
2. `f(v)` 직접 호출은 출력에서 callee가 input보다 앞에 놓이므로
   `SourceReordered` validator가 실패했다. `Effects::NONE`으로 허용한 input span을
   `SourcePreservation::relocated`에 명시적으로 등록했다.
3. 기존 출력 테스트 두 건이 statement-form pipeline의 `$tt_ap`를 기대해 실패했다.
   runtime과 typecheck로 평가 순서·contextual typing 보존을 확인하고 새 직접 호출
   계약으로 기대값을 갱신했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_TSGO_ROOT=/Users/seominyong/Downloads/source/typescript-go cargo test`

## 결과

Phase 6의 남은 query 경계와 세 codegen/parser 정리를 완료했다. 임시값·helper·exit
target은 평가 횟수와 순서가 증명되는 경우에만 줄어든다. source preservation,
TypeScript 타입 검사, Node runtime을 포함한 전체 테스트가 통과했다.
