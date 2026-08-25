import { $tt_ap, $tt_fl } from "@tt/runtime";
declare function half(n: number): number;
declare function twice(n: number): number;

export const once = $tt_ap(half(4), twice).toFixed(1);
export const composed = $tt_fl($tt_fl(half, twice), (($tt_v) => ($tt_v).toFixed(2)));
export const inline = $tt_ap((n => n + 1)(3), twice);
