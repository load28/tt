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
declare const flag: boolean;
declare function consume(item: { kind: "item"; run: (x: number) => number }): void;
{
  let $tt_v0: number;
  const $tt_v1 = (consume);
  {
    const $tt_m = flag;
    switch ($tt_m) {
      case true: const $tt_a0 = { value: 0 }; $tt_v0 = $tt_a0.value; break;
      case false: const $tt_a1 = { value: 1 }; $tt_v0 = $tt_a1.value; break;
      default: throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
  $tt_v1(($tt_v0 === 0 ? (// leading comment
     /* returned value */ { kind: "item", run: x => x + 1 }
    // trailing comment
  ) : { kind: "item", run: x => x - 1 }));
}
