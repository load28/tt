//// [aMatchWhoseSubjectIsAMatchKeepsBothSubjectsApart.tt] ////
variant S { A(v: number), B(w: number), C }
const mm = match (match (S.A(1)) { A(v) => S.B(v), _ => S.C }) { B(w) => "b" + w, _ => "x" };


//// [aMatchWhoseSubjectIsAMatchKeepsBothSubjectsApart.ts]
type S =
  | { kind: "A"; v: number }
  | { kind: "B"; w: number }
  | { kind: "C" };
const S = {
  A: (v: number): S => ({ kind: "A", v }),
  B: (w: number): S => ({ kind: "B", w }),
  C: { kind: "C" } as const,
};
let $tt_v0$mm: string;
{
  let $tt_m_1; {
    const $tt_m = S.A(1);
    switch ($tt_m.kind) {
      case "A": {
        const { v } = $tt_m;
        $tt_m_1 = S.B(v);
        break;
      }
      default: {
        $tt_m_1 = S.C;
        break;
      }
    }
  }
  switch ($tt_m_1.kind) {
    case "B": {
      const { w } = $tt_m_1;
      $tt_v0$mm = "b" + w;
      break;
    }
    default: {
      $tt_v0$mm = "x";
      break;
    }
  }
}
const mm = $tt_v0$mm;
