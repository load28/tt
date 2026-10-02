//// [asyncConciseArrowClaimsAResultRegion.tt] ////
variant R { Ok(value: number), Err(error: string) }
declare const g: () => R;
const f = async (): Promise<R> => result { const x = try g(); return x + 1; };


//// [asyncConciseArrowClaimsAResultRegion.ts]
type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
declare const g: () => R;
const f = async (): Promise<R> => {
  let $tt_v0: Awaited< Promise<R>>;
  $tt_v0: {
    const $tt_t0 = g();
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const x = $tt_t0.value; { $tt_v0 = { kind: "Ok" as const, value: x + 1 }; break $tt_v0; }
  }
  return $tt_v0;
};
