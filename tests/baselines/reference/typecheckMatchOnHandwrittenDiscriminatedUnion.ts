//// [typecheckMatchOnHandwrittenDiscriminatedUnion.tt] ////

type AppEvent =
  | { kind: "click"; x: number; y: number }
  | { kind: "key"; code: string };
const f = (e: AppEvent) => match (e) {
  click(x, y) => x + y,
  key(code) => code.length,
};

export {};


//// [typecheckMatchOnHandwrittenDiscriminatedUnion.ts]
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

type AppEvent =
  | { kind: "click"; x: number; y: number }
  | { kind: "key"; code: string };
const f = (e: AppEvent) => {
  let $tt_v0: number;
  {
    const $tt_m = e;
    switch ($tt_m.kind) {
      case "click": {
        const { x, y } = $tt_m;
        $tt_v0 = x + y;
        break;
      }
      case "key": {
        const { code } = $tt_m;
        $tt_v0 = code.length;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};

export {};
