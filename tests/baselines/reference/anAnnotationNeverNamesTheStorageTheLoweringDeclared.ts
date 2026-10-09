//// [anAnnotationNeverNamesTheStorageTheLoweringDeclared.tt] ////

function pick(n: number) {
  const C = match (n) { 1 => class { q = 1 }, _ => class { q = 2 } };
  const K = match (n) { 1 => { const Local = class { r = 3 }; return Local; }, _ => class { r = 4 } };
  return [new C().q, new K().r];
}
console.log(JSON.stringify([pick(1), pick(2)]));

export {};


//// [anAnnotationNeverNamesTheStorageTheLoweringDeclared.ts]

function pick(n: number) {
  let $tt_v0;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = (void 0, class { q = 1 });
        break;
      }
      default: {
        $tt_v0 = (void 0, class { q = 2 });
        break;
      }
    }
  }
  const C = $tt_v0;
  let $tt_v1;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        const Local = class { r = 3 }; $tt_v1 = Local; break;
      }
      default: {
        $tt_v1 = (void 0, class { r = 4 });
        break;
      }
    }
  }
  const K = $tt_v1;
  return [new C().q, new K().r];
}
console.log(JSON.stringify([pick(1), pick(2)]));

export {};
