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
declare const items:{price:number;quantity:number}[];declare const ready:boolean;
let $tt_v0$view;
{
  const $tt_m = ready;
  switch ($tt_m) {
    case true: {
      const $tt_a0 = { value: <p>{items.map(item=><p>{Number(item.quantity)+Number(item.price)}</p>)}</p> }; $tt_v0$view = $tt_a0.value;
      break;
    }
    case false: {
      const $tt_a1 = { value: <p>Empty</p> }; $tt_v0$view = $tt_a1.value;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
const view=$tt_v0$view;
