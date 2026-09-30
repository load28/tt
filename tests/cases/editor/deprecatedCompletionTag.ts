// @filename: util.ts
/** @deprecated use addAll */
export function oldAdd(a: number): number { return a; }
export const api = {
  /** @deprecated use next */
  prev: 1,
  next: 2,
};
// @filename: main.ts
import { api } from "./util";
type R = { kind: "Ok"; v: number } | { kind: "Err"; e: string };
declare const R: unknown;
export function run(r: R): number {
  if (r.kind === "Ok") {
    const { v } = r;
    return oldAd/*deprecated*/ + api./*member*/;
  }
  return 0;
}
