//// [optionalPostfixTypesAreCheckedAsPlainTypescript2.tt] ////
declare const value: { n: number } | undefined;
const bad = value |> ?.n |> ((n: number) => n + 1);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [optionalPostfixTypesAreCheckedAsPlainTypescript2.ts]
import { $tt_ap } from "./tt/runtime.js";
declare const value: { n: number } | undefined;
const bad = $tt_ap(value?.n, ((n: number) => n + 1));

export {};
