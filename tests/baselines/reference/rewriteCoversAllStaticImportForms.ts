//// [rewriteCoversAllStaticImportForms.tt] ////

import def from "./a.tt";
import def2, { named as alias } from "./b.tt";
import * as ns from "./c.tt";
import type { T } from "./d.tt";
import "./side.tt";
export { x, y as z } from "./e.tt";
export * from "./f.tt";
export * as g from "./g.tt";
export type { U } from "./h.tt";


//// [rewriteCoversAllStaticImportForms.ts]

import def from "./a.js";
import def2, { named as alias } from "./b.js";
import * as ns from "./c.js";
import type { T } from "./d.js";
import "./side.js";
export { x, y as z } from "./e.js";
export * from "./f.js";
export * as g from "./g.js";
export type { U } from "./h.js";
