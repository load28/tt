//// [fullMatchOnBuiltinVariantsCompiles.tt] ////

const f = (o: Option<number>) => match (o) { Some(value) => value, None => 0 };
const g = (r: Result<number, string>) => match (r) { Ok(value) => value, Err(error) => error.length };


//// [fullMatchOnBuiltinVariantsCompiles.ts]
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

const f = (o: Option<number>) => {
  let $tt_v0;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "Some": {
        const { value } = $tt_m;
        $tt_v0 = value;
        break;
      }
      case "None": {
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
const g = (r: Result<number, string>) => {
  let $tt_v1;
  {
    const $tt_m = r;
    switch ($tt_m.kind) {
      case "Ok": {
        const { value } = $tt_m;
        $tt_v1 = value;
        break;
      }
      case "Err": {
        const { error } = $tt_m;
        $tt_v1 = error.length;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1;
};
