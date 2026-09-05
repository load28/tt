type State =
  | { kind: "Ready"; value: number }
  | { kind: "Empty" };
const State = {
  Ready: (value: number): State => ({ kind: "Ready", value }),
  Empty: { kind: "Empty" } as const,
};
declare const state: State;
declare function consume(item: {run: (x: number) => number}): void;
const $tt_v1 = (consume);
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "Ready": {
      const { value } = $tt_m;
      const amount = value + 1; $tt_v1({run: x => x + amount}); break;
    }
    case "Empty": {
      $tt_v1(({run: x => x}));
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}

