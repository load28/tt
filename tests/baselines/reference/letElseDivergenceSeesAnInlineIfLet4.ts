//// [letElseDivergenceSeesAnInlineIfLet4.tt] ////
variant Res { Ok(value: number), Err(error: string) }
function f(r: Res): number {
  const Some(v) = find() else { if let Ok(value) = r { if let Ok(inner) = r { return inner; } else { return value; } } else { return 1; } };
  return v;
}


//// [letElseDivergenceSeesAnInlineIfLet4.ts]
type Res =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const Res = {
  Ok: (value: number): Res => ({ kind: "Ok", value }),
  Err: (error: string): Res => ({ kind: "Err", error }),
};
function f(r: Res): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    {
      const $tt_t1 = r;
      if ($tt_t1.kind === "Ok") {
        const { value } = $tt_t1;
        {
          const $tt_t2 = r;
          if ($tt_t2.kind === "Ok") {
            const { inner } = $tt_t2;
            return inner;
          } else {
            return value;
          }
        }
      } else {
        return 1;
      }
    }
  }
  const { v } = $tt_t0;
  return v;
}
