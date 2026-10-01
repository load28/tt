//// [aWildcardOnlyMatchReadsNothingFromItsSubject.tt] ////

const values: (number | null | undefined)[] = [1, null, undefined];
console.log(JSON.stringify(values.map(v => match (v) { _ => "any" })));

export {};


//// [aWildcardOnlyMatchReadsNothingFromItsSubject.ts]

const values: (number | null | undefined)[] = [1, null, undefined];
console.log(JSON.stringify(values.map(v => {
  let $tt_v0: string;
  {
    const $tt_m = v;
    switch ($tt_m) {
      default: {
        $tt_v0 = "any";
        break;
      }
    }
  }
  return $tt_v0;
})));

export {};
