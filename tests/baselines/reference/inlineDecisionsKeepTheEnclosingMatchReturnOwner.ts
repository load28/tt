//// [inlineDecisionsKeepTheEnclosingMatchReturnOwner.tt] ////

variant Opt { Some(value: number), None }
function pick(o: Opt): number {
  const v = match (o) {
    Some(value) => {
      for (let i = 0; i < 1; i++) {
        if let Some(value: v2) = o {
          const nested = () => { return v2 + 1; };
          try { return nested(); } finally { console.log("cleanup"); }
        }
      }
      return value;
    },
    None => 0,
  };
  return v + 100;
}
function fallback(o: Opt): number {
  const v = match (Boolean(o)) {
    true => {
      if let Some(value: n) = o { return n; } else { return 7; }
      return 9;
    },
    false => 0,
  };
  return v + 100;
}
console.log(pick(Opt.Some(5)), pick(Opt.None));
console.log(fallback(Opt.None), fallback(Opt.Some(3)));

export {};


//// [inlineDecisionsKeepTheEnclosingMatchReturnOwner.ts]
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

type Opt =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const Opt = {
  Some: (value: number): Opt => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
function pick(o: Opt): number {
  let $tt_v0: number;
  $tt_y_v0: {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "Some": {
        const { value } = $tt_m;
        for (let i = 0; i < 1; i++) {
        {
          const $tt_t0 = o;
          if ($tt_t0.kind === "Some") {
            const { value: v2 } = $tt_t0;
            const nested = () => { return v2 + 1; };
          try { $tt_v0 = nested(); break $tt_y_v0; } finally { console.log("cleanup"); }
          }
        }
      }
        $tt_v0 = value;
        break $tt_y_v0;
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
  const v = $tt_v0;
  return v + 100;
}
function fallback(o: Opt): number {
  let $tt_v1: number;
  {
    const $tt_m = Boolean(o);
    switch ($tt_m) {
      case true: {
        {
          const $tt_t1 = o;
          if ($tt_t1.kind === "Some") {
            const { value: n } = $tt_t1;
            $tt_v1 = n; break;
          } else {
            $tt_v1 = 7; break;
          }
        }
        $tt_v1 = 9;
        break;
      }
      case false: {
        $tt_v1 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const v = $tt_v1;
  return v + 100;
}
console.log(pick(Opt.Some(5)), pick(Opt.None));
console.log(fallback(Opt.None), fallback(Opt.Some(3)));

export {};
