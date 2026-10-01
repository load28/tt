//// [optionalPostfixTypesAreCheckedAsPlainTypescript1.tt] ////
declare const value: { n: number } | undefined;
const maybe: number | undefined = value |> ?.n;
const project = flow |> ((v: { n: number } | undefined) => v) |> ?.n;
const also: number | undefined = project(value);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [optionalPostfixTypesAreCheckedAsPlainTypescript1.ts]
import { $tt_fl } from "./tt/runtime.js";
declare const value: { n: number } | undefined;
const maybe: number | undefined = value?.n;
const project = $tt_fl(((v: { n: number } | undefined) => v), (($tt_v) => ($tt_v)?.n));
const also: number | undefined = project(value);

export {};
