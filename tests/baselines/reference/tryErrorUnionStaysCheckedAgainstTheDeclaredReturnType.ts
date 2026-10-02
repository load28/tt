//// [tryErrorUnionStaysCheckedAgainstTheDeclaredReturnType.tt] ////

import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

declare function getUser(): TResult<number, { tag: "user" }>;

function load(): TResult<number, string> {
  const user = try getUser();
  return Result.Ok(user);
}

console.log(load());

export {};


//// [tryErrorUnionStaysCheckedAgainstTheDeclaredReturnType.ts]

import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

declare function getUser(): TResult<number, { tag: "user" }>;

function load(): TResult<number, string> {
  const $tt_t0 = getUser();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const user = $tt_t0.value;
  return Result.Ok(user);
}

console.log(load());

export {};
