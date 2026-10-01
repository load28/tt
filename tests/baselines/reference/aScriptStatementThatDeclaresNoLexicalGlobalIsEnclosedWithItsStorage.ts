//// [aScriptStatementThatDeclaresNoLexicalGlobalIsEnclosedWithItsStorage.tt] ////
declare const o: { kind: "A" } | { kind: "B" };
var v = match (o) { A => 1, B => 2 };
console.log(match (o) { A => 1, B => 2 });
for (const x of match (o) { A => [1], B => [2] }) {}
const {} = match (o) { A => o, B => o };


//// [aScriptStatementThatDeclaresNoLexicalGlobalIsEnclosedWithItsStorage.ts]
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
declare const o: { kind: "A" } | { kind: "B" };
{
  let $tt_v0: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v0 = 1;
        break;
      }
      case "B": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  var v = $tt_v0;
}
{
  let $tt_v1: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": $tt_v1 = 0; break;
      case "B": $tt_v1 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  console.log(($tt_v1 === 0 ? 1 : 2));
}
{
  let $tt_v3: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": $tt_v3 = 0; break;
      case "B": $tt_v3 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  for (const x of ($tt_v3 === 0 ? [1] : [2])) {}
}
{
  let $tt_v4: {
    kind: "A";
} | {
    kind: "B";
};
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v4 = o;
        break;
      }
      case "B": {
        $tt_v4 = o;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const {} = $tt_v4;
}
