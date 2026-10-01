//// [tryExpressionMayContainAMatch1.tt] ////
function f(): X {
  const x = try match (m) { Ok(value) => wrap(value), Err(error) => rewrap(error) };
  return x;
}


//// [tryExpressionMayContainAMatch1.ts]
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
function f(): X {
  let $tt_v0;
  {
    const $tt_m = m;
    switch ($tt_m.kind) {
      case "Ok": {
        const { value } = $tt_m;
        $tt_v0 = wrap(value);
        break;
      }
      case "Err": {
        const { error } = $tt_m;
        $tt_v0 = rewrap(error);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const $tt_t0 = $tt_v0;
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const x = $tt_t0.value;
  return x;
}
