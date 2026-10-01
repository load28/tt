//// [optionalPostfixKeepsNestedTtValuesInsideTheConditionalTail.tt] ////

variant E { A(value: number), B }
const order: string[] = [];
const subject = (): E => { order.push("subject"); return E.A(4); };
const live = { method(value: number): number { order.push("method"); return value; } };
const absent = (() => undefined as typeof live | undefined)();
const miss = absent |> ?.method(match (subject()) { A(value) => value, B => 0 });
const hit = live |> ?.method(match (subject()) { A(value) => value, B => 0 });
console.log(miss, hit, order.join(","));

export {};


//// [optionalPostfixKeepsNestedTtValuesInsideTheConditionalTail.ts]
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

type E =
  | { kind: "A"; value: number }
  | { kind: "B" };
const E = {
  A: (value: number): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
const order: string[] = [];
const subject = (): E => { order.push("subject"); return E.A(4); };
const live = { method(value: number): number { order.push("method"); return value; } };
const absent = (() => undefined as typeof live | undefined)();
let $tt_v0: number | undefined;
do {
  const $tt_v10 = absent;
  let $tt_v4: (number) | (undefined);
  const $tt_v3 = ($tt_v10);
  if ($tt_v3 != null) {
    {
      const $tt_m = subject();
      switch ($tt_m.kind) {
        case "A": {
          const { value } = $tt_m;
          $tt_v4 = $tt_v3?.method(value);
          break;
        }
        case "B": {
          $tt_v4 = $tt_v3?.method(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v4 = undefined;
  }
  $tt_v0 = $tt_v4;
  break;
} while (false);
const miss = $tt_v0;
let $tt_v5: number | undefined;
do {
  const $tt_v11 = live;
  let $tt_v9: (number) | (undefined);
  const $tt_v8 = ($tt_v11);
  if ($tt_v8 != null) {
    {
      const $tt_m = subject();
      switch ($tt_m.kind) {
        case "A": {
          const { value } = $tt_m;
          $tt_v9 = $tt_v8?.method(value);
          break;
        }
        case "B": {
          $tt_v9 = $tt_v8?.method(0);
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
  } else {
    $tt_v9 = undefined;
  }
  $tt_v5 = $tt_v9;
  break;
} while (false);
const hit = $tt_v5;
console.log(miss, hit, order.join(","));

export {};
