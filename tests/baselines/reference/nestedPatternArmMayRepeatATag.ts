//// [nestedPatternArmMayRepeatATag.tt] ////

const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Ok(value) => 0,
  Err(error) => -1,
};


//// [nestedPatternArmMayRepeatATag.ts]
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
  const $tt_m = r;
  do {
    if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some") {
      const { value: v } = $tt_m.value;
      $tt_v0$n = v;
      break;
    }
    if ($tt_m.kind === "Ok") {
      const { value } = $tt_m;
      $tt_v0$n = 0;
      break;
    }
    if ($tt_m.kind === "Err") {
      const { error } = $tt_m;
      $tt_v0$n = -1;
      break;
    }
    throw new Error("tt match: unexpected case " + $tt_show($tt_m));
  } while (false);
}
const n = $tt_v0$n;
