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
      $tt_v0$view = <p>A</p>;
      break;
    }
    case "B": {
      $tt_v0$view = <p>B</p>;
      break;
    }
    case "C": {
      $tt_v0$view = <p>C</p>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const view=$tt_v0$view;
