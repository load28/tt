declare function next(): number;
declare function work(value: number): void;

while (true) {
  let $tt_v0: boolean;
  {
    const $tt_m = next();
    switch ($tt_m) {
      case 1: {
        const $tt_a0 = { value: true };
        $tt_v0 = $tt_a0.value;
        break;
      }
      default: {
        const $tt_a1 = { value: false };
        $tt_v0 = $tt_a1.value;
        break;
      }
    }
  }
  if (!($tt_v0)) break; {
  work(1);
}}

{
  let $tt_v1: number;
  {
    const $tt_m = next();
    switch ($tt_m) {
      case 1: {
        const $tt_a2 = { value: 1 };
        $tt_v1 = $tt_a2.value;
        break;
      }
      default: {
        const $tt_a3 = { value: 0 };
        $tt_v1 = $tt_a3.value;
        break;
      }
    }
  }
  for (let value = $tt_v1;
     ; value++) {
       let $tt_v2: boolean;
       {
         const $tt_m = next();
         switch ($tt_m) {
           case 2: {
             const $tt_a4 = { value: true };
             $tt_v2 = $tt_a4.value;
             break;
           }
           default: {
             const $tt_a5 = { value: false };
             $tt_v2 = $tt_a5.value;
             break;
           }
         }
       }
       if (!($tt_v2)) break; {
  work(value);
}
}}
