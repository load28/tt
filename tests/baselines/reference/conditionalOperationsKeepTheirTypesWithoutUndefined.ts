//// [conditionalOperationsKeepTheirTypesWithoutUndefined.tt] ////

declare const flag: boolean;
declare const maybe: number | undefined;
export const a: number | boolean = flag && match (1) { 1 => 1, _ => 0 };
export const b: number | boolean = flag || match (1) { 1 => 2, _ => 0 };
export const c: number = maybe ?? match (1) { 1 => 3, _ => 0 };
export const d: number = flag ? match (1) { 1 => 4, _ => 0 } : 9;
declare const f: ((v: number) => number) | undefined;
export const e: number | undefined = f?.(match (1) { 1 => 5, _ => 0 });
declare const host: { g?: (v: number) => number };
export const g: number | undefined = host.g?.(match (1) { 1 => 6, _ => 0 });

export {};


//// [conditionalOperationsKeepTheirTypesWithoutUndefined.ts]

declare const flag: boolean;
declare const maybe: number | undefined;
let $tt_v2: number | boolean;
let $tt_v1: boolean;
if ($tt_v1 = flag) {
  let $tt_v0: number | boolean;
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

export const a: number | boolean = $tt_v2;
let $tt_v5: number | boolean;
let $tt_v4: boolean;
if ($tt_v4 = flag) {
  $tt_v5 = $tt_v4;
} else {
  let $tt_v3: number | boolean;
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

export const b: number | boolean = $tt_v5;
let $tt_v8: number;
let $tt_v7: number | undefined;
if (($tt_v7 = maybe) == null) {
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

export const c: number = $tt_v8;
let $tt_v11: number;
if (flag) {
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v11 = 4;
        break;
      }
      default: {
        $tt_v11 = 0;
        break;
      }
    }
  }
} else {
  $tt_v11 = 9;
}

export const d: number = $tt_v11;
declare const f: ((v: number) => number) | undefined;
let $tt_v14: number | undefined;
const $tt_v13: typeof f = (f);
if ($tt_v13 != null) {
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v14 = $tt_v13(5);
        break;
      }
      default: {
        $tt_v14 = $tt_v13(0);
        break;
      }
    }
  }
} else {
  $tt_v14 = undefined;
}

export const e: number | undefined = $tt_v14;
declare const host: { g?: (v: number) => number };
let $tt_v17: number | undefined;
const $tt_v16 = (host.g);
if ($tt_v16 != null) {
  {
    const $tt_m = 1;
    switch ($tt_m) {
      case 1: {
        $tt_v17 = $tt_v16.call(host, 6);
        break;
      }
      default: {
        $tt_v17 = $tt_v16.call(host, 0);
        break;
      }
    }
  }
} else {
  $tt_v17 = undefined;
}

export const g: number | undefined = $tt_v17;

export {};
