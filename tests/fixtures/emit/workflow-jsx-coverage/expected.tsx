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
declare namespace JSX { interface IntrinsicElements { p: {children?:unknown} } }
type State={kind:"A"}|{kind:"B"}|{kind:"C"};declare const state:State;
let $tt_v0$view;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "A": {
      const $tt_a0 = { value: <p>A</p> };
      $tt_v0$view = $tt_a0.value;
      break;
    }
    case "B": {
      const $tt_a1 = { value: <p>B</p> };
      $tt_v0$view = $tt_a1.value;
      break;
    }
    case "C": {
      const $tt_a2 = { value: <p>C</p> };
      $tt_v0$view = $tt_a2.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const view=$tt_v0$view;
