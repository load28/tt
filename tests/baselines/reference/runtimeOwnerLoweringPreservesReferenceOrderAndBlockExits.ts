//// [runtimeOwnerLoweringPreservesReferenceOrderAndBlockExits.tt] ////

variant E { A(value: number), B }
const events: string[] = [];
const receiver = {
  get method() {
    events.push("callee");
    return function (this: unknown, before: number, value: number, after: number) {
      events.push(`call:${this === receiver}:${before}:${value}:${after}`);
      return value;
    };
  },
};
function effect<T>(label: string, value: T): T {
  events.push(label);
  return value;
}
const value = receiver.method(
  effect("before", 1),
  match (effect("subject", E.A(2))) { A(value) => value, B => 0 },
  effect("after", 3),
);
const block = match (E.A(4)) {
  A(value) => {
    if (value > 0) return value * 2;
    return 0;
  },
  B => { return -1; },
};
const nested = match (E.A(5)) {
  A(value) => {
    const add = () => { return value + 1; };
    return add();
  },
  B => { return 0; },
};
console.log(events.join(","));
console.log(value, block, nested);

export {};


//// [runtimeOwnerLoweringPreservesReferenceOrderAndBlockExits.ts]
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
const events: string[] = [];
const receiver = {
  get method() {
    events.push("callee");
    return function (this: unknown, before: number, value: number, after: number) {
      events.push(`call:${this === receiver}:${before}:${value}:${after}`);
      return value;
    };
  },
};
function effect<T>(label: string, value: T): T {
  events.push(label);
  return value;
}
let $tt_v0: number;
const $tt_v2 = (effect("before", 1));
{
  const $tt_m = effect("subject", E.A(2));
  switch ($tt_m.kind) {
    case "A": {
      const { value } = $tt_m;
      $tt_v0 = value;
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
const value = receiver.method(
  $tt_v2,
  $tt_v0,
  effect("after", 3),
);
let $tt_v3: number;
{
  const $tt_m = E.A(4);
  switch ($tt_m.kind) {
    case "A": {
      const { value } = $tt_m;
      if (value > 0) { $tt_v3 = value * 2; break; }
      $tt_v3 = 0;
      break;
    }
    case "B": {
      $tt_v3 = -1; break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const block = $tt_v3;
let $tt_v4: number;
{
  const $tt_m = E.A(5);
  switch ($tt_m.kind) {
    case "A": {
      const { value } = $tt_m;
      const add = () => { return value + 1; };
      $tt_v4 = add();
      break;
    }
    case "B": {
      $tt_v4 = 0; break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const nested = $tt_v4;
console.log(events.join(","));
console.log(value, block, nested);

export {};
