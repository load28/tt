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
declare const x: { kind: "A" } | { kind: "B" };
if (x) {
  let $tt_v0: number;
  {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v0 = 1;
        break;
      }
      case "B": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
}