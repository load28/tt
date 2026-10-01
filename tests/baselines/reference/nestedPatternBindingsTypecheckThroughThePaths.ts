//// [nestedPatternBindingsTypecheckThroughThePaths.tt] ////

variant Opt { Some(value: number), None }
variant Res { Ok(value: Opt), Err(error: string) }
function f(r: Res): number {
  return match (r) {
    Ok(value: Some(value: v)) => v + 1,
    _ => 0,
  };
}

export {};


//// [nestedPatternBindingsTypecheckThroughThePaths.ts]

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
function f(r: Res): number {
  let $tt_v0: number;
  {
    const $tt_m = r;
    do {
      if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some") {
        const { value: v } = $tt_m.value;
        $tt_v0 = v + 1;
        break;
      }
      $tt_v0 = 0;
      break;
    } while (false);
  }
  return $tt_v0;
}

export {};
