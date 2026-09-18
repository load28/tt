declare namespace JSX { interface IntrinsicElements { p: {children?:unknown} } }
declare const items:{price:number;quantity:number}[];declare const ready:boolean;
let $tt_v0;
{
  const $tt_m = ready;
  switch ($tt_m) {
    case true: {
      $tt_v0 = (<p>{items.map(item=>{
        let $tt_subject_1;
        let $tt_subject_2;
        
        return <p>{Number(($tt_subject_1 = item.quantity, ($tt_subject_1 === 0) ? 0 : item.quantity))+Number(($tt_subject_2 = item.price, ($tt_subject_2 === 0) ? 0 : item.price))}</p>;
      })}</p>);
      break;
    }
    case false: {
      $tt_v0 = <p>Empty</p>;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + JSON.stringify($tt_m));
    }
  }
}
const view=$tt_v0;
