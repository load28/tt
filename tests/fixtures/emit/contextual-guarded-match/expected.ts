declare const input: unknown;
declare const flag: boolean;
declare function consume(item: { kind: "item"; run: (x: number) => number }): void;
const $tt_v1 = (consume);
const $tt_m = flag;

$tt_v1((($tt_m === true && typeof input === "string") ? ({ kind: "item", run: x => x + input.length }) : ({ kind: "item", run: x => x })));
