//// [anOperatorLineAfterABlockBodiedArrowFunctionIsItsOwnStatement.tt] ////
const log: string[] = []
const f = () => {}
/x/g.exec("x") |> String |> log.push
const g = async () => {}
/y/.test("y") |> String |> log.push
const h = (): void => {}
-1 |> String |> log.push
const k = () => {}
(2) |> String |> log.push
f(); g(); h(); k()
console.log(log.join(","))

export {};

//// [tt/runtime.ts] support module @tt/std/runtime.ts

//// [anOperatorLineAfterABlockBodiedArrowFunctionIsItsOwnStatement.ts]
import { $tt_ap } from "./tt/runtime.js";
const log: string[] = []
const f = () => {}
;(($tt_v, $tt_r) => $tt_r.push($tt_v))($tt_ap(/x/g.exec("x"), String), (log))
const g = async () => {}
;(($tt_v, $tt_r) => $tt_r.push($tt_v))($tt_ap(/y/.test("y"), String), (log))
const h = (): void => {}
;(($tt_v, $tt_r) => $tt_r.push($tt_v))($tt_ap(-1, String), (log))
const k = () => {}
;(($tt_v, $tt_r) => $tt_r.push($tt_v))(String((2)), (log))
f(); g(); h(); k()
console.log(log.join(","))

export {};
