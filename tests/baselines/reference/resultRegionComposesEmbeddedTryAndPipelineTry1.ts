//// [resultRegionComposesEmbeddedTryAndPipelineTry1.tt] ////
variant R { Ok(value: number), Err(error: string) }
declare const g: () => R;
const f = (): R => result { return Math.round(try g() * 1.1); };


//// [resultRegionComposesEmbeddedTryAndPipelineTry1.ts]
type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
declare const g: () => R;
const f = (): R => {
  let $tt_v0: R;
  $tt_v0: {
    let $tt_v1: number;
    const $tt_t0 = g();
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    $tt_v1 = $tt_t0.value;
    { $tt_v0 = { kind: "Ok" as const, value: Math.round($tt_v1 * 1.1) }; break $tt_v0; }
  }
  return $tt_v0;
};
