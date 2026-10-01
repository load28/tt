//// [resultRegionComposesEmbeddedTryAndPipelineTry2.tt] ////
variant R { Ok(value: number), Err(error: string) }
declare const g: () => R; declare const step: (x: R) => R;
const f = (): R => result { const v = try (g() |> step); return v; };


//// [resultRegionComposesEmbeddedTryAndPipelineTry2.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
declare const g: () => R; declare const step: (x: R) => R;
const f = (): R => {
  let $tt_v0: R;
  $tt_v0: {
    const $tt_t0 = ($tt_ap(g(), step));
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const v = $tt_t0.value; { $tt_v0 = { kind: "Ok" as const, value: v }; break $tt_v0; }
  }
  return $tt_v0;
};
