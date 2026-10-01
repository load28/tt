//// [anOptionalMemberStepIsTheOptionalCall.tt] ////

class C { k = 3; m(x: number) { return x * this.k; } }
const pick = <T,>(value: T): T => [value][0];
const o: C | undefined = pick<C | undefined>(new C());
const none: C | undefined = pick<C | undefined>(undefined);
const nested: { c?: C } | undefined = pick<{ c?: C } | undefined>({ c: new C() });
const order: string[] = [];
const head = () => { order.push("head"); return 2; };
console.log(JSON.stringify([head() |> o?.m, head() |> none?.m, head() |> nested?.c?.m, order]));

export {};


//// [anOptionalMemberStepIsTheOptionalCall.ts]

class C { k = 3; m(x: number) { return x * this.k; } }
const pick = <T,>(value: T): T => [value][0];
const o: C | undefined = pick<C | undefined>(new C());
const none: C | undefined = pick<C | undefined>(undefined);
const nested: { c?: C } | undefined = pick<{ c?: C } | undefined>({ c: new C() });
const order: string[] = [];
const head = () => { order.push("head"); return 2; };
console.log(JSON.stringify([(($tt_v, $tt_r) => $tt_r?.m($tt_v))(head(), (o)), (($tt_v, $tt_r) => $tt_r?.m($tt_v))(head(), (none)), (($tt_v, $tt_r) => $tt_r?.c?.m($tt_v))(head(), (nested)), order]));

export {};
