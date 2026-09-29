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
        const $tt_a0 = { value: 1 }; $tt_v0 = $tt_a0.value;
        break;
      }
      case "B": {
        const $tt_a1 = { value: 2 }; $tt_v0 = $tt_a1.value;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
}