//// [resultRegionComposesWithNestedMatchOnce.tt] ////
variant R { Ok(value: number), Err(error: string) }
declare const g: () => R;
const f = (): R => result {
const n = try g();
const doubled = match (n) { 0 => 0, _ => n * 2 };
return doubled;
};


//// [resultRegionComposesWithNestedMatchOnce.ts]
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
    const $tt_t0 = g();
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const n = $tt_t0.value;
let $tt_v1: number;
{
  const $tt_m = n;
  switch ($tt_m) {
    case 0: {
      $tt_v1 = 0;
      break;
    }
    default: {
      $tt_v1 = n * 2;
      break;
    }
  }
}
const doubled = $tt_v1;
{
  $tt_v0 = { kind: "Ok" as const, value: doubled };
  break $tt_v0;
}
  }
  return $tt_v0;
};
