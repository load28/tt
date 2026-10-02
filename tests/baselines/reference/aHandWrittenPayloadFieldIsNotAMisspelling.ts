//// [aHandWrittenPayloadFieldIsNotAMisspelling.tt] ////
const n = match (o) { Some(v) => v, None => 0 };


//// [aHandWrittenPayloadFieldIsNotAMisspelling.ts]
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
let $tt_v0$n;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "Some": {
      const { v } = $tt_m;
      $tt_v0$n = v;
      break;
    }
    case "None": {
      $tt_v0$n = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const n = $tt_v0$n;
