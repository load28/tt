type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
declare function h(s: string): R<number>;
declare const o: { name: string };
declare const s: "A" | "B";
export function f(): R<number> {
  const v = ((s) => {
    h(/*arg*/
  const w = o./*member*/
