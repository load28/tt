//// [aMaterializedPipelineKeepsHeadBeforeCallee.tt] ////

variant E { A(value: number), B }
const order: string[] = [];
const head = (): E => { order.push("head"); return E.A(2); };
const step = () => { order.push("step"); return (value: number) => {
  order.push("call");
  return value + 1;
}; };
const value = match (head()) { A(value) => value, B => 0 } |> step();
console.log(order.join(","), value);

export {};


//// [aMaterializedPipelineKeepsHeadBeforeCallee.ts]
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
const head = (): E => { order.push("head"); return E.A(2); };
const step = () => { order.push("step"); return (value: number) => {
  order.push("call");
  return value + 1;
}; };
let $tt_v0: number;
do {
  let $tt_v2: number;
  {
    const $tt_m = head();
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v2 = value;
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
  $tt_v0 = step()($tt_v2);
  break;
} while (false);
const value = $tt_v0;
console.log(order.join(","), value);

export {};
