//// [aStatementPipelineEndingTheFileWithoutASemicolon.tt] ////
// A pipeline statement whose match head is planned as a block-owned prelude
// can end the file with neither a semicolon nor a line break. Nothing follows
// the value inside its owner, so the value itself is the last thing written
// before the owner's block closes.
variant G { E, F }
const choose = (): G => G.F;
const g = choose();
const show = (n: number) => console.log(n);
match (g) { E => 1, F => 2 } |> (show)


//// [aStatementPipelineEndingTheFileWithoutASemicolon.ts]
var $tt_show: (value: unknown) => string = function (value) {
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
};
// A pipeline statement whose match head is planned as a block-owned prelude
// can end the file with neither a semicolon nor a line break. Nothing follows
// the value inside its owner, so the value itself is the last thing written
// before the owner's block closes.
type G =
  | { kind: "E" }
  | { kind: "F" };
const G = {
  E: { kind: "E" } as const,
  F: { kind: "F" } as const,
};
const choose = (): G => G.F;
const g = choose();
const show = (n: number) => console.log(n);
{
  let $tt_v0: void;
  do {
    let $tt_v2: number;
    {
      const $tt_m = g;
      switch ($tt_m.kind) {
        case "E": {
          $tt_v2 = 1;
          break;
        }
        case "F": {
          $tt_v2 = 2;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    $tt_v0 = (show)($tt_v2);
    break;
  } while (false);
  $tt_v0
}
\ No newline at end of file
