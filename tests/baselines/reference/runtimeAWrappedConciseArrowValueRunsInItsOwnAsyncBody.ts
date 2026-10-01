//// [runtimeAWrappedConciseArrowValueRunsInItsOwnAsyncBody.tt] ////

variant V { A(n: number), B }
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const load = async (n: number): Promise<R<number>> => n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
const g = async (p: Promise<V>) => match (await p) { A(n) => n, B => 0 } as number;
const h = (v: V) => match (v) { A(n) => n, B => 0 } satisfies number;
const i = async (p: Promise<V>) => (match (await p) { A(n) => n * 2, B => 0 }) as number;
const j = (v: V) => (match (v) { A(n) => n, B => -1 });
const r = async (n: number) => result { const v = try await load(n); return v + 1; } as R<number>;
console.log(await g(Promise.resolve(V.A(3))), h(V.B), await i(Promise.resolve(V.A(4))), j(V.B));
console.log(JSON.stringify(await r(1)), JSON.stringify(await r(-1)));

export {};


//// [runtimeAWrappedConciseArrowValueRunsInItsOwnAsyncBody.ts]
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

type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
const load = async (n: number): Promise<R<number>> => n > 0 ? { kind: "Ok", value: n } : { kind: "Err", error: "neg" };
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
const i = async (p: Promise<V>) => {
  let $tt_v2: number;
  {
    const $tt_m = await p;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v2 = n * 2;
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
        $tt_v3 = -1;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return ($tt_v3);
};
const r = async (n: number) => {
  let $tt_v4: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: number;
});
  $tt_v4: {
    const $tt_t0 = await load(n);
    if (!("value" in $tt_t0)) {
      $tt_v4 = $tt_t0;
      break $tt_v4;
    }
    const v = $tt_t0.value; { $tt_v4 = { kind: "Ok" as const, value: v + 1 }; break $tt_v4; }
  }
  return $tt_v4 as R<number>;
};
console.log(await g(Promise.resolve(V.A(3))), h(V.B), await i(Promise.resolve(V.A(4))), j(V.B));
console.log(JSON.stringify(await r(1)), JSON.stringify(await r(-1)));

export {};
