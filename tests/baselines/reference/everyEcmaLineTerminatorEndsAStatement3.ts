//// [everyEcmaLineTerminatorEndsAStatement3.tt] ////
type K = string;
declare const o: unknown;
declare const x: object;
declare const e: Error;
export function f(v: Option<number>): number {
  const Some(value) = v else {
    const a = 1     throw e
  };
  return value;
}


//// [everyEcmaLineTerminatorEndsAStatement3.ts]
type K = string;
declare const o: unknown;
declare const x: object;
declare const e: Error;
export function f(v: Option<number>): number {
  const $tt_t0 = v;
  if ($tt_t0.kind !== "Some") {
    const a = 1     throw e
  }
  const { value } = $tt_t0;
  return value;
}
