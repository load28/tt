//// [matchGuardsKeepTheirTypeNarrowingScope.tt] ////

declare const input: unknown;
declare const flag: boolean;
declare function consume(value: number): void;
consume(match (flag) {
  true if typeof input === "string" => input.length,
  _ => 0,
});

export {};


//// [matchGuardsKeepTheirTypeNarrowingScope.ts]

declare const input: unknown;
declare const flag: boolean;
declare function consume(value: number): void;
const $tt_v1: typeof consume = (consume);
const $tt_m = flag;

$tt_v1((($tt_m === true && typeof input === "string") ? input.length : 0));

export {};
