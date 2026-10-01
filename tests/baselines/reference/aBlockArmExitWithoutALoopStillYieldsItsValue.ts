//// [aBlockArmExitWithoutALoopStillYieldsItsValue.tt] ////

variant Pick { Some(v: number), None }
function choose(p: Pick): number {
  return match (p) {
    Some(v) => { const doubled = v * 2; return doubled; },
    None => 0,
  };
}
const guarded = (n: number): number => match (n) {
  0 if true => 1,
  _ => { return n + 100; },
};
console.log(choose(Pick.Some(21)), choose(Pick.None), guarded(0), guarded(5));

export {};


//// [aBlockArmExitWithoutALoopStillYieldsItsValue.ts]
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

type Pick =
  | { kind: "Some"; v: number }
  | { kind: "None" };
const Pick = {
  Some: (v: number): Pick => ({ kind: "Some", v }),
  None: { kind: "None" } as const,
};
function choose(p: Pick): number {
  let $tt_v0: number;
  {
    const $tt_m = p;
    switch ($tt_m.kind) {
      case "Some": {
        const { v } = $tt_m;
        const doubled = v * 2; $tt_v0 = doubled; break;
      }
      case "None": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
const guarded = (n: number): number => {
  let $tt_v1: number;
  {
    const $tt_m = n;
    do {
      if ($tt_m === 0) {
        if (true) {
          $tt_v1 = 1;
          break;
        }
      }
      { $tt_v1 = n + 100; break;
      }
    } while (false);
  }
  return $tt_v1;
};
console.log(choose(Pick.Some(21)), choose(Pick.None), guarded(0), guarded(5));

export {};
