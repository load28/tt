//// [everyEcmaLineTerminatorEndsAStatement5.tt] ////
declare const e: Error;
export function f(v: Option<number>): number {
  const Some(value) = v else {
    // a comment ends at a CR    throw e
  };
  return value;
}


//// [everyEcmaLineTerminatorEndsAStatement5.ts]
declare const e: Error;
export function f(v: Option<number>): number {
  const $tt_t0 = v;
  if ($tt_t0.kind !== "Some") {
    // a comment ends at a CR    throw e
  }
  const { value } = $tt_t0;
  return value;
}
