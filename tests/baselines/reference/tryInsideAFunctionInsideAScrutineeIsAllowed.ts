//// [tryInsideAFunctionInsideAScrutineeIsAllowed.tt] ////
const x = match (run(() => { try g(); return h(); })) {
  Ok(value) => value,
  Err(error) => 0,
};


//// [tryInsideAFunctionInsideAScrutineeIsAllowed.ts]
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
let $tt_v0$x;
{
  const $tt_m = (run(() => { const $tt_t0 = g();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  } return h(); }));
  switch ($tt_m.kind) {
    case "Ok": {
      const { value } = $tt_m;
      $tt_v0$x = value;
      break;
    }
    case "Err": {
      const { error } = $tt_m;
      $tt_v0$x = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const x = $tt_v0$x;
