declare function next(): number;
declare function work(value: number): void;

while (true) {
  let $tt_v0;
  {
    const $tt_m = next();
    switch ($tt_m) {
      case 1: {
        $tt_v0 = true;
        break;
      }
      default: {
        $tt_v0 = false;
        break;
      }
    }
  }
  if (!($tt_v0)) break; {
  work(1);
}}

let $tt_v1;
{
  const $tt_m = next();
  switch ($tt_m) {
    case 1: {
      $tt_v1 = 1;
      break;
    }
    default: {
      $tt_v1 = 0;
      break;
    }
  }
}
for (let value = $tt_v1;
     ; value++) {
       let $tt_v2;
       {
         const $tt_m = next();
         switch ($tt_m) {
           case 2: {
             $tt_v2 = true;
             break;
           }
           default: {
             $tt_v2 = false;
             break;
           }
         }
       }
       if (!($tt_v2)) break; {
  work(value);
}}
