//// [aScriptDeclarationNamesItsGlobalStorageAfterItsBinding.tt] ////
declare const o: { kind: "A" } | { kind: "B" };
const total = match (o) { A => 1, B => 2 };
let { kind } = match (o) { A => o, B => o };
class Base extends (match (o) { A => Object, B => Object }) {}
function f(x: typeof o) { return match (x) { A => 1, B => 2 }; }


//// [aScriptDeclarationNamesItsGlobalStorageAfterItsBinding.ts]
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
let $tt_v0$total: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v0$total = 1;
      break;
    }
    case "B": {
      $tt_v0$total = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const total = $tt_v0$total;
let $tt_v1$kind: {
    kind: "A";
} | {
    kind: "B";
};
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v1$kind = o;
      break;
    }
    case "B": {
      $tt_v1$kind = o;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
let { kind } = $tt_v1$kind;
let $tt_v2$Base: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": $tt_v2$Base = 0; break;
    case "B": $tt_v2$Base = 1; break;
    default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  }
}
class Base extends (($tt_v2$Base === 0 ? Object : Object)) {}
function f(x: typeof o) { let $tt_v3: number;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v3 = 1;
      break;
    }
    case "B": {
      $tt_v3 = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v3; }
