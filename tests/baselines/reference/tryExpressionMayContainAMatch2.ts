//// [tryExpressionMayContainAMatch2.tt] ////
function f(): X {
  const x = try wrap(match (m) { Ok(value) => value, Err(_) => 0 });
  return x;
}


//// [tryExpressionMayContainAMatch2.ts]
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
  const $tt_v1 = (wrap);
  {
    const $tt_m = m;
    switch ($tt_m.kind) {
      case "Ok": {
        const { value } = $tt_m;
        $tt_v0 = $tt_v1(value);
        break;
      }
      case "Err": {
        const { _ } = $tt_m;
        $tt_v0 = $tt_v1(0);
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
