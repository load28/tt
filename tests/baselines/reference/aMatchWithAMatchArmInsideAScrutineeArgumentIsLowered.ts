//// [aMatchWithAMatchArmInsideAScrutineeArgumentIsLowered.tt] ////
variant S { A, B }
const s: S = S.A as S;
function p(x: number): S { return x === 1 ? S.A : S.B; }
export function t() {
  const a = match (p(match (s) { A => match (s) { A => 1, B => 2 }, B => 2 })) { A => "a", B => "b" };
  const b = (p(match (s) { A => match (s) { A => 1, B => 2 }, B => 2 })) |> String;
  const c = match (p(match (s) { A => match (s) { A => 1, B => 2 }, B => 2 }), s) { (A, A) => "aa", _ => "x" };
  let d = "none";
  if let A() = p(match (s) { A => match (s) { A => 1, B => 2 }, B => 2 }) { d = "if-let"; }
  let A() = p(match (s) { A => match (s) { A => 1, B => 2 }, B => 2 }) else { return "else"; };
  return JSON.stringify([a, b, c, d]);
}
console.log(t());


//// [aMatchWithAMatchArmInsideAScrutineeArgumentIsLowered.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const s: S = S.A as S;
function p(x: number): S { return x === 1 ? S.A : S.B; }
export function t() {
  let $tt_v0: string;
  {
    let $tt_m_1; let $tt_v1: number;
    const $tt_v2: typeof p = (p);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          {
            const $tt_m = s;
            switch ($tt_m.kind) {
              case "A": {
                $tt_v1 = 1;
                break;
              }
              case "B": {
                $tt_v1 = 2;
                break;
              }
              default: {
                throw new Error("tt match: unexpected case " + $tt_show($tt_m));
              }
            }
          }
          break;
        }
        case "B": {
          $tt_v1 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_m_1 = $tt_v2($tt_v1);
    switch ($tt_m_1.kind) {
      case "A": {
        $tt_v0 = "a";
        break;
      }
      case "B": {
        $tt_v0 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m_1));
      }
    }
  }
  const a = $tt_v0;
  let $tt_v3: string;
  do {
    let $tt_v18: S;
    let $tt_v4: number;
    const $tt_v5: typeof p = (p);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          {
            const $tt_m = s;
            switch ($tt_m.kind) {
              case "A": {
                $tt_v4 = 1;
                break;
              }
              case "B": {
                $tt_v4 = 2;
                break;
              }
              default: {
                throw new Error("tt match: unexpected case " + $tt_show($tt_m));
              }
            }
          }
          break;
        }
        case "B": {
          $tt_v4 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v18 = ($tt_v5($tt_v4));
    $tt_v3 = String($tt_v18);
    break;
  } while (false);
  const b = $tt_v3;
  let $tt_v6: string;
  {
    let $tt_m0_1; let $tt_v7: number;
    const $tt_v8: typeof p = (p);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          {
            const $tt_m = s;
            switch ($tt_m.kind) {
              case "A": {
                $tt_v7 = 1;
                break;
              }
              case "B": {
                $tt_v7 = 2;
                break;
              }
              default: {
                throw new Error("tt match: unexpected case " + $tt_show($tt_m));
              }
            }
          }
          break;
        }
        case "B": {
          $tt_v7 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_m0_1 = $tt_v8($tt_v7);
    const $tt_m1_1 = s;
    do {
      if ($tt_m0_1.kind === "A" && $tt_m1_1.kind === "A") {
        $tt_v6 = "aa";
        break;
      }
      $tt_v6 = "x";
      break;
    } while (false);
  }
  const c = $tt_v6;
  let d = "none";
  {
    let $tt_t0; let $tt_v9: number;
    const $tt_v10: typeof p = (p);
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          {
            const $tt_m = s;
            switch ($tt_m.kind) {
              case "A": {
                $tt_v9 = 1;
                break;
              }
              case "B": {
                $tt_v9 = 2;
                break;
              }
              default: {
                throw new Error("tt match: unexpected case " + $tt_show($tt_m));
              }
            }
          }
          break;
        }
        case "B": {
          $tt_v9 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_t0 = $tt_v10($tt_v9);
    if ($tt_t0.kind === "A") {
      d = "if-let";
    }
  }
  let $tt_t1; let $tt_v11: number;
  const $tt_v12: typeof p = (p);
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        {
          const $tt_m = s;
          switch ($tt_m.kind) {
            case "A": {
              $tt_v11 = 1;
              break;
            }
            case "B": {
              $tt_v11 = 2;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        break;
      }
      case "B": {
        $tt_v11 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_t1 = $tt_v12($tt_v11);
  if ($tt_t1.kind !== "A") {
    return "else";
  }
  
  return JSON.stringify([a, b, c, d]);
}
console.log(t());
