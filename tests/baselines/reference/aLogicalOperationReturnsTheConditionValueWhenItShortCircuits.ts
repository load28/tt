//// [aLogicalOperationReturnsTheConditionValueWhenItShortCircuits.tt] ////

const zero = 0 as number;
const empty = "" as string;
const a = zero && match (1) { 1 => 1, _ => 0 };
const b = empty || match (1) { 1 => 2, _ => 0 };
const c = (zero as number | null) ?? match (1) { 1 => 3, _ => 0 };
console.log(a, JSON.stringify(b), c);

export {};


//// [aLogicalOperationReturnsTheConditionValueWhenItShortCircuits.ts]

const zero = 0 as number;
const empty = "" as string;
let $tt_v2: number;
let $tt_v1: number;
if ($tt_v1 = zero) {
  let $tt_v0: number;
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = 1;
        break;
      }
      default: {
        $tt_v0 = 0;
        break;
      }
    }
  }
  $tt_v2 = $tt_v1 && $tt_v0;
} else {
  $tt_v2 = $tt_v1;
}

const a = $tt_v2;
let $tt_v5: string | number;
let $tt_v4: string;
if ($tt_v4 = empty) {
  $tt_v5 = $tt_v4;
} else {
  let $tt_v3: number;
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v3 = 2;
        break;
      }
      default: {
        $tt_v3 = 0;
        break;
      }
    }
  }
  $tt_v5 = $tt_v4 || $tt_v3;
}

const b = $tt_v5;
let $tt_v8: number;
let $tt_v7: number | null;
if (($tt_v7 = zero as number | null) == null) {
  let $tt_v6: number;
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v6 = 3;
        break;
      }
      default: {
        $tt_v6 = 0;
        break;
      }
    }
  }
  $tt_v8 = $tt_v7 ?? $tt_v6;
} else {
  $tt_v8 = $tt_v7;
}

const c = $tt_v8;
console.log(a, JSON.stringify(b), c);

export {};
