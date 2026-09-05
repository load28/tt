declare const flag: boolean;
declare function consume(item: { kind: "item"; run: (x: number) => number }): void;
let $tt_v0;
const $tt_v1 = (consume);
{
  const $tt_m = flag;
  switch ($tt_m) {
    case true: $tt_v0 = 0; break;
    case false: $tt_v0 = 1; break;
    default: throw new Error("tt match: unexpected literal " + JSON.stringify($tt_m));
  }
}
$tt_v1(($tt_v0 === 0 ? ({ kind: "item", run: x => x + 1 }) : ({ kind: "item", run: x => x - 1 })));
