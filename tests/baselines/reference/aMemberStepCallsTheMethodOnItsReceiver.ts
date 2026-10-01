//// [aMemberStepCallsTheMethodOnItsReceiver.tt] ////

const order: string[] = [];
const obj = { k: 10, add(n: number) { return n + this.k; } };
const key = "add" as const;
const traced = { k: 1, get m() { order.push("get"); return function (this: { k: number }, n: number) { return n + this.k; }; } };
class Base { k = 3; m(n: number) { return n + this.k; } }
class Derived extends Base {
    #p(n: number) { return n - this.k; }
    run() { return [2].map(x => x |> this.m |> this.#p); }
    parent() { return [4].map(x => x |> super.m); }
    composed() { return flow |> super.m |> this.#p; }
}
const gen = { id<T>(v: T): T { return v; } };
const inlined = 1 |> obj.add;
const nested = [1].map(x => x |> obj.add);
const chained = 1 |> obj.add |> obj.add;
const computed = [2].map(x => x |> obj[key]);
const generic: number = (1 + 1) |> gen.id;
const ordered = (order.push("head"), 1) |> traced.m;
const composed = flow |> ((n: number) => n * 2) |> obj.add |> String;
async function awaited() { return 3 |> (await Promise.resolve(obj)).add; }
awaited().then(q => {
    console.log(JSON.stringify([inlined, nested, chained, computed, generic, ordered, q]));
    console.log(JSON.stringify([new Derived().run(), new Derived().parent(), new Derived().composed()(5), composed(1), order]));
});

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [aMemberStepCallsTheMethodOnItsReceiver.ts]
import { $tt_fl } from "./tt/runtime.js";

const order: string[] = [];
const obj = { k: 10, add(n: number) { return n + this.k; } };
const key = "add" as const;
const traced = { k: 1, get m() { order.push("get"); return function (this: { k: number }, n: number) { return n + this.k; }; } };
class Base { k = 3; m(n: number) { return n + this.k; } }
class Derived extends Base {
    #p(n: number) { return n - this.k; }
    run() { return [2].map(x => (($tt_v, $tt_r) => $tt_r.#p($tt_v))((($tt_v, $tt_r) => $tt_r.m($tt_v))(x, (this)), (this))); }
    parent() { return [4].map(x => (($tt_v) => super.m($tt_v))(x)); }
    composed() { return $tt_fl((() => (super.m).bind(this))(), (($tt_r) => ($tt_r.#p).bind($tt_r))((this))); }
}
const gen = { id<T>(v: T): T { return v; } };
const inlined = obj.add(1);
const nested = [1].map(x => (($tt_v, $tt_r) => $tt_r.add($tt_v))(x, (obj)));
const chained = (($tt_v, $tt_r) => $tt_r.add($tt_v))(obj.add(1), (obj));
const computed = [2].map(x => (($tt_v, $tt_r) => $tt_r[key]($tt_v))(x, (obj)));
const generic: number = (($tt_v, $tt_r) => $tt_r.id($tt_v))((1 + 1), (gen));
const ordered = (($tt_v, $tt_r) => $tt_r.m($tt_v))((order.push("head"), 1), (traced));
const composed = $tt_fl($tt_fl(((n: number) => n * 2), (($tt_r) => ($tt_r.add).bind($tt_r))((obj))), String);
async function awaited() { return (await Promise.resolve(obj)).add(3); }
awaited().then(q => {
    console.log(JSON.stringify([inlined, nested, chained, computed, generic, ordered, q]));
    console.log(JSON.stringify([new Derived().run(), new Derived().parent(), new Derived().composed()(5), composed(1), order]));
});

export {};
