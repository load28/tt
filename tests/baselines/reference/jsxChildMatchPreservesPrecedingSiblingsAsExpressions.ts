//// [jsxChildMatchPreservesPrecedingSiblingsAsExpressions.ttx] ////
variant Maybe { Some(value: string), None }
declare const value: Maybe;
const view = <main><h1>title</h1><form>form</form>{match (value) {
  Some(value) => <p>{value}</p>, None => null,
}}</main>;


//// [jsxChildMatchPreservesPrecedingSiblingsAsExpressions.tsx]
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
type Maybe =
  | { kind: "Some"; value: string }
  | { kind: "None" };
const Maybe = {
  Some: (value: string): Maybe => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
declare const value: Maybe;
let $tt_v0$view;
const $tt_v1$view = (<h1>title</h1>);
const $tt_v2$view = (<form>form</form>);
{
  const $tt_m = value;
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      $tt_v0$view = <p>{value}</p>;
      break;
    }
    case "None": {
      $tt_v0$view = null;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const view = <main>{$tt_v1$view}{$tt_v2$view}{$tt_v0$view}</main>;
