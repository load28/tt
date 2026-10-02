//// [runtimeAConditionalOperationsConditionIsEvaluatedOnceWhereItIsTested.tt] ////

variant O { A(n: number), B }
const o = O.A(1) as O;
let reads = 0;
function read<T>(v: T): T { reads++; return v; }
const cfg = { get name(): string | undefined { reads++; return "abc"; } };
const a = cfg.name ? match (o) { A(n) => n + 1, B => 0 } : -1;
const b = cfg.name && match (o) { A(n) => n + 2, B => 0 };
const c = !cfg.name || match (o) { A(n) => n + 3, B => 0 };
const d = cfg.name ?? match (o) { A(n) => n + 4, B => 0 };
const e = read(0) && match (o) { A(n) => n, B => 0 };
const f = read("") || match (o) { A(n) => n + 5, B => 0 };
const g = read(null) ?? match (o) { A(n) => n + 6, B => 0 };
const h = (read(true) ? { k: 1 } : null) && match (o) { A(n) => n, B => 0 };
const i = (read(false) ? [] : null) || match (o) { A(n) => n, B => 0 };
console.log(a, b, c, d, e, f, g, h, i, reads);

export {};


//// [runtimeAConditionalOperationsConditionIsEvaluatedOnceWhereItIsTested.ts]
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

type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const o = O.A(1) as O;
let reads = 0;
function read<T>(v: T): T { reads++; return v; }
const cfg = { get name(): string | undefined { reads++; return "abc"; } };
let $tt_v2: number;
if (cfg.name) {
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v2 = n + 1;
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
} else {
  $tt_v2 = -1;
}

const a = $tt_v2;
let $tt_v5: (number | "") | (string | undefined);
let $tt_v4: string | undefined;
if ($tt_v4 = cfg.name) {
  let $tt_v3: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = n + 2;
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
  $tt_v5 = $tt_v4 && $tt_v3;
} else {
  $tt_v5 = $tt_v4;
}

const b = $tt_v5;
let $tt_v8: (true) | (number);
let $tt_v7: boolean;
if ($tt_v7 = !cfg.name) {
  $tt_v8 = $tt_v7;
} else {
  let $tt_v6: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v6 = n + 3;
        break;
      }
      case "B": {
        $tt_v6 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v8 = $tt_v7 || $tt_v6;
}

const c = $tt_v8;
let $tt_v11: (number) | (string);
let $tt_v10: string | undefined;
if (($tt_v10 = cfg.name) == null) {
  let $tt_v9: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v9 = n + 4;
        break;
      }
      case "B": {
        $tt_v9 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v11 = $tt_v10 ?? $tt_v9;
} else {
  $tt_v11 = $tt_v10;
}

const d = $tt_v11;
let $tt_v14: 0;
let $tt_v13: 0;
if ($tt_v13 = read(0)) {
  let $tt_v12: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v12 = n;
        break;
      }
      case "B": {
        $tt_v12 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v14 = $tt_v13 && $tt_v12;
} else {
  $tt_v14 = $tt_v13;
}

const e = $tt_v14;
let $tt_v17: number;
let $tt_v16: "";
if ($tt_v16 = read("")) {
  $tt_v17 = $tt_v16;
} else {
  let $tt_v15: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v15 = n + 5;
        break;
      }
      case "B": {
        $tt_v15 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v17 = $tt_v16 || $tt_v15;
}

const f = $tt_v17;
let $tt_v20: number;
let $tt_v19: null;
if (($tt_v19 = read(null)) == null) {
  let $tt_v18: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v18 = n + 6;
        break;
      }
      case "B": {
        $tt_v18 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v20 = $tt_v19 ?? $tt_v18;
} else {
  $tt_v20 = $tt_v19;
}

const g = $tt_v20;
let $tt_v23: (number) | (null);
let $tt_v22: {
    k: number;
} | null;
if ($tt_v22 = ({ value: read(true) ? { k: 1 } : null }).value) {
  let $tt_v21: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v21 = n;
        break;
      }
      case "B": {
        $tt_v21 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v23 = $tt_v22 && $tt_v21;
} else {
  $tt_v23 = $tt_v22;
}

const h = $tt_v23;
let $tt_v26: (never[]) | (number);
let $tt_v25: never[] | null;
if ($tt_v25 = ({ value: read(false) ? [] : null }).value) {
  $tt_v26 = $tt_v25;
} else {
  let $tt_v24: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v24 = n;
        break;
      }
      case "B": {
        $tt_v24 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  $tt_v26 = $tt_v25 || $tt_v24;
}

const i = $tt_v26;
console.log(a, b, c, d, e, f, g, h, i, reads);

export {};
