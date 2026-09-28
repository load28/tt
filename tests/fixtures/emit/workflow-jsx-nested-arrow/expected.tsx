declare namespace JSX { interface IntrinsicElements { p: {children?:unknown} } }
declare const items:{price:number;quantity:number}[];declare const ready:boolean;
let $tt_v0;
{
  const $tt_m = ready;
  switch ($tt_m) {
    case true: {
      $tt_v0 = (<p>{items.map(item=><p>{Number(item.quantity)+Number(item.price)}</p>)}</p>);
      break;
    }
    case false: {
      $tt_v0 = <p>Empty</p>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
const view=$tt_v0;
function $tt_show(value: unknown): string {
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
}
