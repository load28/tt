//// [aTupleMatchOverMatchesNamesEachNestedSubject.tt] ////
variant S { A(v: number), B(w: number), C }
declare const s: S;
declare const t: S;
const mm = match (match (s) { A(v) => S.B(v), _ => S.C }, match (t) { A(v) => S.B(v), _ => S.C }) { (B(w), B) => "b" + w, _ => "x" };


//// [aTupleMatchOverMatchesNamesEachNestedSubject.ts]
type S =
  | { kind: "A"; v: number }
  | { kind: "B"; w: number }
  | { kind: "C" };
const S = {
  A: (v: number): S => ({ kind: "A", v }),
  B: (w: number): S => ({ kind: "B", w }),
  C: { kind: "C" } as const,
};
declare const s: S;
declare const t: S;
let $tt_v0$mm: string;
{
  let $tt_m0_1; {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_m0_1 = S.B(v);
        break;
      }
      default: {
        $tt_m0_1 = S.C;
        break;
      }
    }
  }
  let $tt_m1_1; {
    const $tt_m = t;
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_m1_1 = S.B(v);
        break;
      }
      default: {
        $tt_m1_1 = S.C;
        break;
      }
    }
  }
  do {
    if ($tt_m0_1.kind === "B" && $tt_m1_1.kind === "B") {
      const { w } = $tt_m0_1;
      $tt_v0$mm = "b" + w;
      break;
    }
    $tt_v0$mm = "x";
    break;
  } while (false);
}
const mm = $tt_v0$mm;
