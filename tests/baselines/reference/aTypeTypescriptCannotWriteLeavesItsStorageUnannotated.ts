//// [aTypeTypescriptCannotWriteLeavesItsStorageUnannotated.tt] ////

class Base {}
function Tagged<B extends new (...a: any[]) => {}>(Base: B) { return class extends Base { tag = "t"; }; }
function pick(n: number) {
  const M = match (n) { 1 => Tagged(Base), _ => Tagged(Base) };
  const o = match (n) { 1 => new (class { x = 1 })(), _ => new (class { x = 2 })() };
  const a = match (n) { 1 => [class {}], _ => [class {}, class {}] };
  return [new M().tag, o.x, a.length];
}
console.log(JSON.stringify([pick(1), pick(2)]));

export {};


//// [aTypeTypescriptCannotWriteLeavesItsStorageUnannotated.ts]

class Base {}
function Tagged<B extends new (...a: any[]) => {}>(Base: B) { return class extends Base { tag = "t"; }; }
function pick(n: number) {
  let $tt_v0;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        $tt_v0 = Tagged(Base);
        break;
      }
      default: {
        $tt_v0 = Tagged(Base);
        break;
      }
    }
  }
  const M = $tt_v0;
  let $tt_v1;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        $tt_v1 = new (class { x = 1 })();
        break;
      }
      default: {
        $tt_v1 = new (class { x = 2 })();
        break;
      }
    }
  }
  const o = $tt_v1;
  let $tt_v2;
  {
    const $tt_m = n;
    switch ($tt_m) {
      case 1: {
        const $tt_a0 = { value: [class {}] };
        $tt_v2 = $tt_a0.value;
        break;
      }
      default: {
        const $tt_a1 = { value: [class {}, class {}] };
        $tt_v2 = $tt_a1.value;
        break;
      }
    }
  }
  const a = $tt_v2;
  return [new M().tag, o.x, a.length];
}
console.log(JSON.stringify([pick(1), pick(2)]));

export {};
