//// [aWrappedConciseArrowValueLowersToABlockBodyNotAnIife.tt] ////
variant V { A(n: number), B }
const g = async (p: Promise<V>) => match (await p) { A(n) => n, B => 0 } as number;
const h = (v: V) => match (v) { A(n) => n, B => 0 } satisfies number;
const i = (v: V) => (match (v) { A(n) => n, B => 0 }) as number;
const j = (v: V) => (match (v) { A(n) => n, B => 0 });


//// [aWrappedConciseArrowValueLowersToABlockBodyNotAnIife.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const g = async (p: Promise<V>) => {
  let $tt_v0: number;
  {
    const $tt_m = await p;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = n;
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0 as number;
};
const h = (v: V) => {
  let $tt_v1: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = n;
        break;
      }
      case "B": {
        $tt_v1 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1 satisfies number;
};
const i = (v: V) => {
  let $tt_v2: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v2 = n;
        break;
      }
      case "B": {
        $tt_v2 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return ($tt_v2) as number;
};
const j = (v: V) => {
  let $tt_v3: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = n;
        break;
      }
      case "B": {
        $tt_v3 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return ($tt_v3);
};
