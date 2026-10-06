//// [nestedCallbackReturnsAndOverlappingCallCapturesPreserveEffects.tt] ////

variant State { Ready(values: number[]), Empty }
const events: string[] = [];
const convert = (n: number) => { events.push(`convert:${n}`); return n + 10; };
const answer = match(State.Ready([2, 5])) {
    Ready(values) => { return values.map(n => convert(match(n){0=>0,_=>n}) + convert(match(n){0=>0,_=>n})); },
    Empty => [],
};
const keyed = { [String(match(1){1=>1,_=>0})]: match(2){2=>2,_=>0} };
const truthy = (1 |> ((n: number) => n + 1)) && match(3){3=>3,_=>0};
const fromBody = (() => { if let Ready(values) = State.Ready([4]) { return values[0]; } return 0; })() && match(4){4=>4,_=>0};
const take = (...values: number[]) => values;
const callArgs = take((1 |> ((n: number) => n + 1)), match(3){3=>3,_=>0});
console.log(JSON.stringify([answer, keyed, truthy, fromBody, callArgs]));
console.log(events.join(","));

export {};


//// [nestedCallbackReturnsAndOverlappingCallCapturesPreserveEffects.ts]
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
  | { kind: "Ready"; values: number[] }
  | { kind: "Empty" };
const State = {
  Ready: (values: number[]): State => ({ kind: "Ready", values }),
  Empty: { kind: "Empty" } as const,
};
const events: string[] = [];
const convert = (n: number) => { events.push(`convert:${n}`); return n + 10; };
let $tt_v0: number[];
{
  const $tt_m = State.Ready([2, 5]);
  switch ($tt_m.kind) {
    case "Ready": {
      const { values } = $tt_m;
      $tt_v0 = values.map(n => {
        let $tt_subject_1;
        let $tt_subject_2;
        
        return convert(($tt_subject_1 = n, ($tt_subject_1 === 0) ? 0 : n)) + convert(($tt_subject_2 = n, ($tt_subject_2 === 0) ? 0 : n));
      }); break;
    }
    case "Empty": {
      const $tt_a0 = { value: [] };
      $tt_v0 = $tt_a0.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const answer = $tt_v0;
let $tt_subject_3;
let $tt_subject_4;

const keyed = { [String(($tt_subject_3 = 1, ($tt_subject_3 === 1) ? 1 : 0))]: ($tt_subject_4 = 2, ($tt_subject_4 === 2) ? 2 : 0) };
let $tt_v13: number;
let $tt_v12: number;
if ($tt_v12 = ((n: number) => n + 1)(1)) {
  let $tt_v11: number;
  {
    const $tt_m = 3;
    switch ($tt_m) {
      case 3: {
        $tt_v11 = 3;
        break;
      }
      default: {
        $tt_v11 = 0;
        break;
      }
    }
  }
  $tt_v13 = $tt_v12 && $tt_v11;
} else {
  $tt_v13 = $tt_v12;
}

const truthy = $tt_v13;
let $tt_v16: number;
let $tt_v15: number;
if ($tt_v15 = ((() => { {
  const $tt_t0 = State.Ready([4]);
  if ($tt_t0.kind === "Ready") {
    const { values } = $tt_t0;
    return values[0];
  }
} return 0; })())) {
  let $tt_v14: number;
  {
    const $tt_m = 4;
    switch ($tt_m) {
      case 4: {
        $tt_v14 = 4;
        break;
      }
      default: {
        $tt_v14 = 0;
        break;
      }
    }
  }
  $tt_v16 = $tt_v15 && $tt_v14;
} else {
  $tt_v16 = $tt_v15;
}

const fromBody = $tt_v16;
const take = (...values: number[]) => values;
let $tt_v18: number[];
const $tt_v19: typeof take = (take);
const $tt_v20: number = (((n: number) => n + 1)(1));
{
  const $tt_m = 3;
  switch ($tt_m) {
    case 3: {
      $tt_v18 = $tt_v19($tt_v20, 3);
      break;
    }
    default: {
      $tt_v18 = $tt_v19($tt_v20, 0);
      break;
    }
  }
}
const callArgs = $tt_v18;
console.log(JSON.stringify([answer, keyed, truthy, fromBody, callArgs]));
console.log(events.join(","));

export {};
