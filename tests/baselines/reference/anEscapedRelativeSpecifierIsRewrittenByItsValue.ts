//// [f.tt] ////
export const f = 1;

//// [g.tt] ////
import { f } from "./f.\x74t";
export const h = import(`./\u{66}.tt`);
export { f };


//// [f.ts]
export const f = 1;
\ No newline at end of file

//// [g.ts]
import { f } from "./f.js";
export const h = import(`./\u{66}.js`);
export { f };
