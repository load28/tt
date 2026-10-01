//// [boxes.tt] ////
export variant Box { Full(value: number), Half(value: number), Empty }
const box: Box = Box.Full(3);
export const Full(value: size) = box else { throw new Error("empty"); };
export let Full(value: counter) | Half(value: counter) = box else { throw new Error("empty"); };
export namespace Nested {
  export var Full(value: inner) = box else { throw new Error("empty"); };
}

//// [only.tt] ////
const found: { kind: "Some"; value: string } | { kind: "None" } = { kind: "Some", value: "only" };
export const Some(value: label) = found else { throw new Error("none"); };

//// [main.tt] ////
import { size, counter, Nested } from "./boxes.tt";
import { label } from "./only.tt";
console.log(`size=${size} counter=${counter} inner=${Nested.inner} label=${label}`);


//// [boxes.ts]
export type Box =
  | { kind: "Full"; value: number }
  | { kind: "Half"; value: number }
  | { kind: "Empty" };
export const Box = {
  Full: (value: number): Box => ({ kind: "Full", value }),
  Half: (value: number): Box => ({ kind: "Half", value }),
  Empty: { kind: "Empty" } as const,
};
const box: Box = Box.Full(3);
const $tt_t0 = box;
if ($tt_t0.kind !== "Full") {
  throw new Error("empty");
}
export const { value: size } = $tt_t0;
const $tt_t1 = box;
if ($tt_t1.kind !== "Full" && $tt_t1.kind !== "Half") {
  throw new Error("empty");
}
export let { value: counter } = $tt_t1;
export namespace Nested {
  const $tt_t2 = box;
  if ($tt_t2.kind !== "Full") {
    throw new Error("empty");
  }
  export var { value: inner } = $tt_t2;
}
\ No newline at end of file

//// [main.ts]
import { size, counter, Nested } from "./boxes.js";
import { label } from "./only.js";
console.log(`size=${size} counter=${counter} inner=${Nested.inner} label=${label}`);

//// [only.ts]
const found: { kind: "Some"; value: string } | { kind: "None" } = { kind: "Some", value: "only" };
const $tt_t0 = found;
if ($tt_t0.kind !== "Some") {
  throw new Error("none");
}
export const { value: label } = $tt_t0;
\ No newline at end of file
