//// [generatedValueSlotsPreserveAuthoredContextualTypes1.tt] ////
type TResult<T, E> = { kind: "Ok"; value: T } | { kind: "Err"; error: E };
declare const g: () => TResult<number, string>;
const f = (): TResult<readonly number[], string> => result {
const n = try g(); if (n === 0) return []; return [n];
};


//// [generatedValueSlotsPreserveAuthoredContextualTypes1.ts]
type TResult<T, E> = { kind: "Ok"; value: T } | { kind: "Err"; error: E };
declare const g: () => TResult<number, string>;
const f = (): TResult<readonly number[], string> => {
  let $tt_v0: TResult<readonly number[], string>;
  $tt_v0: {
    const $tt_t0 = g();
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const n = $tt_t0.value; if (n === 0) { $tt_v0 = { kind: "Ok" as const, value: [] }; break $tt_v0; } { $tt_v0 = { kind: "Ok" as const, value: [n] }; break $tt_v0; }
  }
  return $tt_v0;
};
