//// [scopedContextualCallVariants.tt] ////

variant State { Ready(value: number), Empty }
declare const state: State;
declare const flag: boolean;
declare function consume(item: {kind: "item"; run: (x: number) => number}): void;
consume(match (state) {
  Ready(value) if value > 0 => ({kind: "item", run: x => x + value}),
  _ => ({kind: "item", run: x => x}),
});
consume(match (state) {
  Ready(value: amount) => ({kind: "item", run: x => x + amount}),
  Empty => ({kind: "item", run: x => x}),
});
consume(match (flag) {
  true => { const amount = 1; return {kind: "item", run: x => x + amount}; },
  false => { function amount() { return 2; } return {kind: "item", run: x => x + amount()}; },
});

export {};


//// [scopedContextualCallVariants.ts]
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

type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;
declare const flag: boolean;
declare function consume(item: {kind: "item"; run: (x: number) => number}): void;
const $tt_v1: typeof consume = (consume);
{
  const $tt_m = state;
  do {
    if ($tt_m.kind === "Ready") {
      const { value } = $tt_m;
      if (value > 0) {
        $tt_v1(({kind: "item", run: x => x + value}));
        break;
      }
    }
    $tt_v1(({kind: "item", run: x => x}));
    break;
  } while (false);
}

const $tt_v3: typeof consume = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value: amount } = $tt_m;
      $tt_v3(({kind: "item", run: x => x + amount}));
      break;
    }
    case "Empty": {
      $tt_v3(({kind: "item", run: x => x}));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}

const $tt_v5: typeof consume = (consume);
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: {
      const amount = 1; $tt_v5({kind: "item", run: x => x + amount}); break;
    }
    case false: {
      function amount() { return 2; } $tt_v5({kind: "item", run: x => x + amount()}); break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}


export {};
