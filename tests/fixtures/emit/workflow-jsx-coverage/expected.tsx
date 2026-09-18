declare namespace JSX { interface IntrinsicElements { p: {children?:unknown} } }
type State={kind:"A"}|{kind:"B"}|{kind:"C"};declare const state:State;
let $tt_v0;
{
  const $tt_m = state;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v0 = <p>A</p>;
      break;
    }
    case "B": {
      $tt_v0 = <p>B</p>;
      break;
    }
    case "C": {
      $tt_v0 = <p>C</p>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + JSON.stringify($tt_m));
    }
  }
}
const view=$tt_v0;
