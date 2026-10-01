//// [statementPositionMatchIsASupportedOwner.tt] ////
variant R { Ok(value: number), Err(error: string) }
const f = (x: number) => { match (R.Ok(x)) {
Ok(value) => { console.log(value); },
Err(error) => { console.log(error); },
}; };


//// [statementPositionMatchIsASupportedOwner.ts]
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
type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
const f = (x: number) => { let $tt_v0: undefined;
{
  const $tt_m = R.Ok(x);
  switch ($tt_m.kind) {
    case "Ok": {
      const { value } = $tt_m;
      console.log(value);
        $tt_v0 = undefined;
        break;
    }
    case "Err": {
      const { error } = $tt_m;
      console.log(error);
        $tt_v0 = undefined;
        break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
; };
