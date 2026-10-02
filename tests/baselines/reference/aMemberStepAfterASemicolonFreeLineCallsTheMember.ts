//// [aMemberStepAfterASemicolonFreeLineCallsTheMember.tt] ////
const log: string[] = []
const o = { m(x: unknown) { log.push(String(x)) } }
const k = "m" as const
const c = false
function f() {}
class A {
m(x: unknown) { log.push("this " + String(x)) }
run() {
const w = 0
w |> this.m
}
}
new A().run()
const v = 1
v |> o.m
const a = f
2 |> o[k]
if (c) f
3 |> o?.m
type T = number
4 |> String |> o.m
let d: number
5 |> (o.m)
f // c
6 |> o.m
const g = () => {}
7 |> o.m
function* gen() {
yield
8 |> o.m
}
log.push(String([...gen()].length))
log.push(`${(() => { const w = 9
w |> o.m
return "t" })()}`)
console.log(log.join(","))

export {};


//// [aMemberStepAfterASemicolonFreeLineCallsTheMember.ts]
const log: string[] = []
const o = { m(x: unknown) { log.push(String(x)) } }
const k = "m" as const
const c = false
function f() {}
class A {
m(x: unknown) { log.push("this " + String(x)) }
run() {
const w = 0
;(($tt_v, $tt_r) => $tt_r.m($tt_v))(w, (this))
}
}
new A().run()
const v = 1
;(($tt_v, $tt_r) => $tt_r.m($tt_v))(v, (o))
const a = f
o[k](2)
if (c) f
o?.m(3)
type T = number
;(($tt_v, $tt_r) => $tt_r.m($tt_v))(String(4), (o))
let d: number
;(o.m)(5)
f // c
o.m(6)
const g = () => {}
o.m(7)
function* gen() {
yield
o.m(8)
}
log.push(String([...gen()].length))
log.push(`${(() => { const w = 9
;(($tt_v, $tt_r) => $tt_r.m($tt_v))(w, (o))
return "t" })()}`)
console.log(log.join(","))

export {};
