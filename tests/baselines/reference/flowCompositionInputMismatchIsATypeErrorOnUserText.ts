//// [flowCompositionInputMismatchIsATypeErrorOnUserText.tt] ////
const parse = (s: string) => s.length;
const f = flow |> parse |> ((n: number) => n + 1);
const v = f(3);

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [flowCompositionInputMismatchIsATypeErrorOnUserText.ts]
import { $tt_fl } from "./tt/runtime.js";
const parse = (s: string) => s.length;
const f = $tt_fl(parse, ((n: number) => n + 1));
const v = f(3);

export {};
