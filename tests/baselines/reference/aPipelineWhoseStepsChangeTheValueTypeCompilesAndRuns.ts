//// [aPipelineWhoseStepsChangeTheValueTypeCompilesAndRuns.tt] ////

const flag = Math.random() >= 0;
const pick = (value: { kind: "a" } | { kind: "b" }): string => value.kind;
const lengths = match (1) { _ => [1] } |> (p => p.length);
const text: string = match (flag) { true => [1, 2], false => [3] } |> (p => p.length) |> String;
const count: number = match (flag) { true => "xy", false => "z" } |> .length |> (n => [n, n]) |> .length;
const kind = match (flag) { true => ({ kind: "a" }), false => ({ kind: "b" }) } |> pick;
const mapped: string[] = [match (flag) { true => 1, false => 2 }] |> .map(n => n + 1) |> .map(String);
console.log(lengths, text, count, kind, mapped.join(","));

export {};


//// [aPipelineWhoseStepsChangeTheValueTypeCompilesAndRuns.ts]
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

const flag = Math.random() >= 0;
const pick = (value: { kind: "a" } | { kind: "b" }): string => value.kind;
let $tt_v0: number;
do {
  let $tt_v10: number[];
  {
    const $tt_m = 1;
    switch ($tt_m) {
      default: {
        $tt_v10 = [1];
        break;
      }
    }
  }
  $tt_v0 = (p => p.length)($tt_v10);
  break;
} while (false);
const lengths = $tt_v0;
let $tt_v1: string;
do {
  let $tt_v11: number[];
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v11 = [1, 2];
        break;
      }
      case false: {
        $tt_v11 = [3];
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v12 = (p => p.length)($tt_v11);
  $tt_v1 = String($tt_v12);
  break;
} while (false);
const text: string = $tt_v1;
let $tt_v2: number;
do {
  let $tt_v13: string;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v13 = "xy";
        break;
      }
      case false: {
        $tt_v13 = "z";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const $tt_v14: number = $tt_v13.length;
  const $tt_v15 = (n => [n, n])($tt_v14);
  $tt_v2 = $tt_v15.length;
  break;
} while (false);
const count: number = $tt_v2;
let $tt_v3: string;
do {
  let $tt_v16: {
    kind: "a";
} | {
    kind: "b";
};
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v16 = ({ kind: "a" });
        break;
      }
      case false: {
        $tt_v16 = ({ kind: "b" });
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  $tt_v3 = pick($tt_v16);
  break;
} while (false);
const kind = $tt_v3;
let $tt_v4: string[];
do {
  let $tt_v17: number[];
  let $tt_v5: number;
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        $tt_v5 = 1;
        break;
      }
      case false: {
        $tt_v5 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  const $tt_a0 = { value: [$tt_v5] };
  $tt_v17 = $tt_a0.value;
  const $tt_v18 = $tt_v17.map(n => n + 1);
  $tt_v4 = $tt_v18.map(String);
  break;
} while (false);
const mapped: string[] = $tt_v4;
console.log(lengths, text, count, kind, mapped.join(","));

export {};
