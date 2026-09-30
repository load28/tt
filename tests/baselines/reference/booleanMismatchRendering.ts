//// [flags.tt] ////
export const ready: boolean = true;
export type Dir = "north" | "south";
export declare function heading(): Dir;

//// [booleanMismatchRendering.tt] ////
import { ready, heading } from "./flags.tt";
declare function flag(): boolean;
export const x: number = flag();
export function f(): number { return flag(); }
export const o: { n: number } = { n: flag() };
export const imported: number = ready;
export const named: number = heading();
declare function either(): number | boolean;
export const partly: number = either();


//// [booleanMismatchRendering.ts]
import { ready, heading } from "./flags.js";
declare function flag(): boolean;
export const x: number = flag();
export function f(): number { return flag(); }
export const o: { n: number } = { n: flag() };
export const imported: number = ready;
export const named: number = heading();
declare function either(): number | boolean;
export const partly: number = either();

//// [flags.ts]
export const ready: boolean = true;
export type Dir = "north" | "south";
export declare function heading(): Dir;
