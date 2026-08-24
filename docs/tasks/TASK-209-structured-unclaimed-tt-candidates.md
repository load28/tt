# TASK-209: output verify의 문자열 기반 tt 구문 추정 제거

- **상태**: 완료
- **시작일**: 2026-08-25
- **완료일**: 2026-08-25
- **커밋**: 이 커밋

## 목적

출력 자가 검증 실패 시 오류 줄의 문자열을 검색해 tt 구문을 추정하는
`verify::tt_construct_at`을 제거한다. 파서가 보존한 구조화된 claim failure만
소비해 진단의 구문 종류와 위치를 결정한다.

## 범위

- 포함: 미완성 tt 후보의 종류·span 모델, parser→verify 전달, 휴리스틱 삭제, 회귀 테스트
- 제외: 기존 tt 구문의 판별 규칙과 오류 코드 변경, 일반 TypeScript 파싱 진단 확대

## 의사결정

1. **파서의 claim 실패를 이름 있는 상태로 보존한다.** 일반 오류 메시지만 쓰거나
   verify에서 주변 문자열을 다시 검색하는 대안은 tt 의도를 잃거나 문자열
   휴리스틱을 유지한다. `Claim::Unclaimed`에 구문 종류, 키워드 span, 후보 extent를
   담아 parser가 판별한 사실만 후속 단계로 전달한다.
2. **후보 자체로는 오류를 내지 않는다.** 유효한 TypeScript를 그대로 통과시키는
   계약을 지키기 위해 rollback 후보는 원문으로 남긴다. 출력 검증이 실제 TypeScript
   파싱 실패를 확인하고 매핑된 위치가 후보 extent 안에 있을 때만 기존 tt 안내를
   사용한다.
3. **희소한 메타데이터는 간접 side table로 둔다.** 모든 중첩 `Program`에 `Vec`을
   직접 넣으면 주요 AST enum의 크기가 커진다. 후보가 있을 때만
   `Option<Box<UnclaimedTtCandidates>>`를 할당해 일반 경로의 표현 크기를 유지한다.

## 작업 내역

1. `ast.rs`에 `UnclaimedTtCandidate`, `UnclaimedTtKind`, 희소 side table을 추가했다.
2. `parser::Claim`과 `tries` parser가 미완성 `try <expr>`의 구조화된 rollback
   정보를 남기도록 바꾸고, 중첩 `Program` 전체에서 후보를 수집하도록 구현했다.
3. `verify.rs`의 `tt_construct_at`과 식별자 문자열 검색을 삭제했다. source map으로
   복원한 실패 위치를 parser 후보의 반열린 extent와 대조하도록 바꿨다.
4. 유효한 TS `try` 형태가 후보가 되지 않는 parser 테스트, 누락된 세미콜론의 span
   테스트, 문자열·주석 속 키워드가 tt 진단을 만들지 않는 회귀 테스트를 추가했다.
5. 언어 구문과 판별 규칙은 바뀌지 않아 `docs/ai/tt.md` 갱신이 필요 없음을 확인했다.
6. 별도로 보류할 개선 후보를 `TASK-210` 하나로 등록했다.

## 이슈 및 해결

1. 처음에는 후보 `Vec`을 `Program`에 직접 넣었다. clippy가 `IfLetElse`와
   `ParsedMatch`에서 `large_enum_variant`를 보고했다. `Program`의 inline 크기 증가가
   원인이었고, 희소한 후보 목록만 이름 있는 박스 side table로 분리해 해결했다.
2. 여러 테스트 필터를 한 번의 `cargo test` 인자로 전달해 `unexpected argument`
   오류가 발생했다. 각 필터를 별도 명령으로 실행한 뒤 전체 테스트로 다시 검증했다.

## 검증

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_TSGO_ROOT=/Users/seominyong/Downloads/source/typescript-go cargo test`

## 결과

output verify는 더 이상 오류 줄의 `match`·`try`·`result`·`flow` 문자열로 tt 의도를
추측하지 않는다. 미완성 bare `try`의 기존 안내와 정확한 키워드 위치는 parser가
소유한 구조 정보로 유지되고, 문자열·주석의 같은 단어는 일반 TypeScript 파싱
오류로 남는다. 포맷, clippy, 전체 테스트가 모두 통과했다.
