//// [tsconfig.json] ////
{"compilerOptions":{"strict":false,"target":"es2022","module":"esnext","moduleResolution":"bundler","noEmit":true},"include":["MixedCase"]}

//// [MixedCase/Source.tt] ////
declare const flag: boolean;
export const numbers = () => match(flag) { true => [1], false => [2] };
export const nested = match(flag) { true => [numbers()], false => [] };
export const bad = nested[0]![0]!.toUpperCase();
export const unknowns = () => match(flag) { true => [JSON.parse("1")], false => [JSON.parse("2")] };

//// [MixedCase/Consumer.tt] ////
import { numbers, unknowns } from "./Source.tt";
declare const flag: boolean;
export const nested = match(flag) { true => [numbers()], false => [] };
export const bad = nested[0]![0]!.toUpperCase();
export const unknownNested = match(flag) { true => [unknowns()], false => [] };
export const worse = unknownNested[0]!.foo;


//// [MixedCase/Consumer.ts]
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
import { numbers, unknowns } from "./Source.js";
declare const flag: boolean;
let $tt_v0: number[][];
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: {
      const $tt_a0 = { value: [numbers()] };
      $tt_v0 = $tt_a0.value;
      break;
    }
    case false: {
      const $tt_a1 = { value: [] };
      $tt_v0 = $tt_a1.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
export const nested = $tt_v0;
export const bad = nested[0]![0]!.toUpperCase();
let $tt_v1: any[][];
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: {
      const $tt_a2 = { value: [unknowns()] };
      $tt_v1 = $tt_a2.value;
      break;
    }
    case false: {
      const $tt_a3 = { value: [] };
      $tt_v1 = $tt_a3.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
export const unknownNested = $tt_v1;
export const worse = unknownNested[0]!.foo;

//// [MixedCase/Source.ts]
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
declare const flag: boolean;
export const numbers = () => {
  let $tt_v0: number[];
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        const $tt_a0 = { value: [1] };
        $tt_v0 = $tt_a0.value;
        break;
      }
      case false: {
        const $tt_a1 = { value: [2] };
        $tt_v0 = $tt_a1.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
let $tt_v1: number[][];
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: {
      const $tt_a2 = { value: [numbers()] };
      $tt_v1 = $tt_a2.value;
      break;
    }
    case false: {
      const $tt_a3 = { value: [] };
      $tt_v1 = $tt_a3.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
export const nested = $tt_v1;
export const bad = nested[0]![0]!.toUpperCase();
export const unknowns = () => {
  let $tt_v2: any[];
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: {
        const $tt_a4 = { value: [JSON.parse("1")] };
        $tt_v2 = $tt_a4.value;
        break;
      }
      case false: {
        const $tt_a5 = { value: [JSON.parse("2")] };
        $tt_v2 = $tt_a5.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v2;
};
\ No newline at end of file
