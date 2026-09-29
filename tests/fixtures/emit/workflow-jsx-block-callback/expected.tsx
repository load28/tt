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
declare global { namespace JSX { interface IntrinsicElements { [name: string]: {children?: unknown; [prop: string]: unknown} } } }
type Load<T> =
  | { kind: "Loading" }
  | { kind: "Failed"; message: string }
  | { kind: "Loaded"; value: T };
const Load = {
  Loading: { kind: "Loading" } as const,
  Failed: <T,>(message: string): Load<T> => ({ kind: "Failed", message }),
  Loaded: <T,>(value: T): Load<T> => ({ kind: "Loaded", value }),
};
type Item = {id: string; title: string; quantity: number; price: number};
export function Cart({state,onSelect}:{state:Load<Item[]>;onSelect:(item:Item)=>void}) {
 const filter=''; const setFilter=(value: string) => value;
 let $tt_v0;
 let $tt_v1: number;
 const $tt_v2 = (<input value={filter} onChange={(event: {currentTarget: {value: string}})=>setFilter(event.currentTarget.value)}/>);
 {
   const $tt_m = state;
   switch ($tt_m.kind) {
     case "Loading": {
       const $tt_a0 = { value: <p>Loading</p> }; $tt_v0 = $tt_a0.value;
       break;
     }
     case "Failed": {
       const { message } = $tt_m;
       const $tt_a1 = { value: <p role="alert">{message}</p> }; $tt_v0 = $tt_a1.value;
       break;
     }
     case "Loaded": {
       const { value } = $tt_m;
       const items=value.filter(item=>item.title.includes(filter));
   const $tt_a2 = { value: <ul>{items.map(item=>{
     let $tt_v4: number;
     let $tt_v5: number;
     const $tt_v7 = (item.id);
     const $tt_v9 = (<button onClick={()=>onSelect(item)}>{item.title.trim()}</button>);
     const $tt_v8 = (Number);
     {
       const $tt_m = item.quantity;
       switch ($tt_m) {
         case 0: {
           const $tt_a3 = { value: $tt_v8(0) }; $tt_v4 = $tt_a3.value;
           break;
         }
         default: {
           const $tt_a4 = { value: $tt_v8(item.quantity) }; $tt_v4 = $tt_a4.value;
           break;
         }
       }
     }
     const $tt_v11 = ($tt_v4);
     const $tt_v10 = (Number);
     {
       const $tt_m = item.price;
       switch ($tt_m) {
         case 0: {
           const $tt_a5 = { value: $tt_v10(0) }; $tt_v5 = $tt_a5.value;
           break;
         }
         default: {
           const $tt_a6 = { value: $tt_v10(item.price) }; $tt_v5 = $tt_a6.value;
           break;
         }
       }
     }
     return <li key={$tt_v7}>{$tt_v9}<strong>{$tt_v11 + $tt_v5}</strong></li>;
   })}</ul> }; $tt_v0 = $tt_a2.value;
   break;
     }
     default: {
       throw new Error("tt match: unexpected case " + $tt_show($tt_m));
     }
   }
 }
 {
   const $tt_m = state;
   switch ($tt_m.kind) {
     case "Loaded": {
       const { value } = $tt_m;
       const $tt_a7 = { value: value.length }; $tt_v1 = $tt_a7.value;
       break;
     }
     default: {
       const $tt_a8 = { value: 0 }; $tt_v1 = $tt_a8.value;
       break;
     }
   }
 }
 return <section>{$tt_v2}{$tt_v0}<footer>{$tt_v1} items</footer></section>;
}
