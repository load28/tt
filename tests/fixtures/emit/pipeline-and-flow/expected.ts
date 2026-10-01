import { $tt_fl } from "@tt/runtime";
declare function half(n: number): number;
declare function twice(n: number): number;

export const once = (($tt_v, $tt_f) => $tt_f($tt_v))(half(4), twice).toFixed(1);
export const composed = $tt_fl($tt_fl(half, (($tt_f) => ($tt_v) => $tt_f($tt_v))(twice)), (($tt_v) => ($tt_v).toFixed(2)));
export const inline = (($tt_v, $tt_f) => $tt_f($tt_v))((n => n + 1)(3), twice);
