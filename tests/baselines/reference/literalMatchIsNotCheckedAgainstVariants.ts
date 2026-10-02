//// [literalMatchIsNotCheckedAgainstVariants.tt] ////
const v = match (x) { "Some" => 1, "None" => 2 };


//// [literalMatchIsNotCheckedAgainstVariants.ts]
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
let $tt_v0$v: number;
{
  const $tt_m = x;
  switch ($tt_m) {
    case "Some": {
      $tt_v0$v = 1;
      break;
    }
    case "None": {
      $tt_v0$v = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
const v = $tt_v0$v;
\ No newline at end of file
