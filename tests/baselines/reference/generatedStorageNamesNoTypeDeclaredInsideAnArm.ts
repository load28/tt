//// [generatedStorageNamesNoTypeDeclaredInsideAnArm.tt] ////

variant S { A, B }
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function read(): R { return { kind: "Ok", value: 1 }; }
class Loc { other = 1; }
function inner(s: S) {
  const v = match (s) { A => { class Loc { tag = "a"; } return new Loc(); }, B => ({ tag: "b" }) };
  return v.tag;
}
function inResult(s: S) {
  return result {
    const n = try read();
    const v = match (s) { A => { class Loc { tag = "a" + n; } return new Loc(); }, B => ({ tag: "b" }) };
    return v.tag;
  };
}
function unnamed(s: S) {
  const v = match (s) { A => { class Hidden { tag = "h"; } return new Hidden(); }, B => ({ tag: "b" }) };
  return v.tag;
}
console.log(inner(S.A), inner(S.B), JSON.stringify(inResult(S.A)), unnamed(S.A), new Loc().other);

export {};


//// [generatedStorageNamesNoTypeDeclaredInsideAnArm.ts]
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
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function read(): R { return { kind: "Ok", value: 1 }; }
class Loc { other = 1; }
function inner(s: S) {
  let $tt_v0;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        class Loc { tag = "a"; } $tt_v0 = new Loc(); break;
      }
      case "B": {
        $tt_v0 = ({ tag: "b" });
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const v = $tt_v0;
  return v.tag;
}
function inResult(s: S) {
  let $tt_v1: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: string;
});
  $tt_v1: {
    const $tt_t0 = read();
    if (!("value" in $tt_t0)) {
      $tt_v1 = $tt_t0;
      break $tt_v1;
    }
    const n = $tt_t0.value;
    let $tt_v2;
    {
      const $tt_m = s;
      switch ($tt_m.kind) {
        case "A": {
          class Loc { tag = "a" + n; } $tt_v2 = new Loc(); break;
        }
        case "B": {
          $tt_v2 = ({ tag: "b" });
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    const v = $tt_v2;
    {
      $tt_v1 = { kind: "Ok" as const, value: v.tag };
      break $tt_v1;
    }
  }
  return $tt_v1;
}
function unnamed(s: S) {
  let $tt_v3;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        class Hidden { tag = "h"; } $tt_v3 = new Hidden(); break;
      }
      case "B": {
        $tt_v3 = ({ tag: "b" });
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const v = $tt_v3;
  return v.tag;
}
console.log(inner(S.A), inner(S.B), JSON.stringify(inResult(S.A)), unnamed(S.A), new Loc().other);

export {};
