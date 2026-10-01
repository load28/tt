//// [runtimeNestedPatternFallsThroughOnInnerMismatch.tt] ////

variant Opt { Some(value: number), None }
variant Res { Ok(value: Opt), Err(error: string) }

function grade(r: Res): string {
  return match (r) {
    Ok(value: Some(value: v)) if v > 9000 => "over",
    Ok(value: Some(value: v)) => "num:" + v,
    Ok(value: None()) => "empty",
    Err(error) => "err:" + error,
    // v1 exhaustiveness: nested arms cover nothing, so `Ok` counts as
    // uncovered without a final wildcard (documented, like guards).
    _ => "unreachable",
  };
}

console.log(grade(Res.Ok(Opt.Some(9001))));
console.log(grade(Res.Ok(Opt.Some(3))));
console.log(grade(Res.Ok(Opt.None)));
console.log(grade(Res.Err("boom")));

export {};


//// [runtimeNestedPatternFallsThroughOnInnerMismatch.ts]

type Opt =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const Opt = {
  Some: (value: number): Opt => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
type Res =
  | { kind: "Ok"; value: Opt }
  | { kind: "Err"; error: string };
const Res = {
  Ok: (value: Opt): Res => ({ kind: "Ok", value }),
  Err: (error: string): Res => ({ kind: "Err", error }),
};

function grade(r: Res): string {
  let $tt_v0: string;
  {
    const $tt_m = r;
    do {
      if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some") {
        const { value: v } = $tt_m.value;
        if (v > 9000) {
          $tt_v0 = "over";
          break;
        }
      }
      if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some") {
        const { value: v } = $tt_m.value;
        $tt_v0 = "num:" + v;
        break;
      }
      if ($tt_m.kind === "Ok" && $tt_m.value.kind === "None") {
        $tt_v0 = "empty";
        break;
      }
      if ($tt_m.kind === "Err") {
        const { error } = $tt_m;
        $tt_v0 = "err:" + error;
        break;
      }
      // v1 exhaustiveness: nested arms cover nothing, so `Ok` counts as
      // uncovered without a final wildcard (documented, like guards).
      $tt_v0 = "unreachable";
      break;
    } while (false);
  }
  return $tt_v0;
}

console.log(grade(Res.Ok(Opt.Some(9001))));
console.log(grade(Res.Ok(Opt.Some(3))));
console.log(grade(Res.Ok(Opt.None)));
console.log(grade(Res.Err("boom")));

export {};
