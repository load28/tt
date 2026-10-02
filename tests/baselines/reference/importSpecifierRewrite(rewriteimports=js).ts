//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "allowImportingTsExtensions": true
  }
}

//// [token.tt] ////
export variant Token { Num(value: number), Eof }

//// [main.tt] ////
import { Token } from "./token.tt";
export const value = (t: Token): number => match (t) { Num(value) => value, Eof => 0 };


//// [main.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}
import { Token } from "./token.js";
export const value = (t: Token): number => {
  let $tt_v0: number;
  {
    const $tt_m = t;
    switch ($tt_m.kind) {
      case "Num": {
        const { value } = $tt_m;
        $tt_v0 = value;
        break;
      }
      case "Eof": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};

//// [token.ts]
export type Token =
  | { kind: "Num"; value: number }
  | { kind: "Eof" };
export const Token = {
  Num: (value: number): Token => ({ kind: "Num", value }),
  Eof: { kind: "Eof" } as const,
};
