// @filename: api.ts
export type Shape = { kind: "Point" } | { kind: "Circle"; radius: number };
export function /*target*/work(n: number): number { return n; }
// @filename: contract.d.ts
export declare function work(n: number): number;
//# sourceMappingURL=contract.d.ts.map
// @filename: contract.d.ts.map
{"version":3,"file":"contract.d.ts","sourceRoot":"","sources":["api.ts"],"names":[],"mappings":"wBACgB,IAAI"}
// @filename: main.ts
import { work } from "./contract.js";
import { render } from "./ui-contract.js";
import { work as original } from "./api.ts";
/*mapped*/work(1);
/*direct*/original(2);
/*jsx*/render(3);
// @filename: ui-contract.d.ts
export declare function render(n: number): unknown;
//# sourceMappingURL=ui-contract.d.ts.map
// @filename: ui-contract.d.ts.map
{"version":3,"file":"ui-contract.d.ts","sourceRoot":"source files/","sources":["ui.tsx"],"names":[],"mappings":"wBACgB,MAAM"}
// @filename: source files/ui.tsx
export type View = { kind: "Empty" } | { kind: "Count"; value: number };
export function /*jsxTarget*/render(n: number) { return <span>{n}</span>; }
