//// [generatedValueSlotsPreserveAuthoredContextualTypes2.tt] ////
type Toggle = "on" | "off";
const flip = (value: Toggle): Toggle => match (value) {
"on" => "off", "off" => "on",
};


//// [generatedValueSlotsPreserveAuthoredContextualTypes2.ts]
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
type Toggle = "on" | "off";
const flip = (value: Toggle): Toggle => {
  let $tt_v0: Toggle;
  {
    const $tt_m = value;
    switch ($tt_m) {
      case "on": {
        $tt_v0 = "off";
        break;
      }
      case "off": {
        $tt_v0 = "on";
        break;
      }
      default: {
        throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
