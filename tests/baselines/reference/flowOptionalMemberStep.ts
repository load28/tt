//// [flowOptionalMemberStep.tt] ////
// An optional-chain step in a `flow` composition is the optional call
// `obj?.m(v)`, as it is in a pipeline: it calls the method on its receiver
// and gives `undefined` when the chain short-circuits. Its receiver is
// evaluated once, where the step is written, as a member step's is.
const inc = (n: number) => n + 1;
class Scale {
  constructor(readonly k: number) {}
  by(n: number) { return n * this.k; }
  inner = { k: 3, by(n: number) { return n * this.k; } };
}
const present = new Scale(10) as Scale | undefined;
const absent = undefined as Scale | undefined;
const key = "by" as const;
const seen: string[] = [];
function receiver(scale: Scale | undefined) {
  seen.push("receiver");
  return scale;
}
const steps = {
  member: flow |> inc |> present?.by,
  nested: flow |> inc |> present?.inner.by,
  computed: flow |> inc |> present?.[key],
  asserted: flow |> inc |> (present?.by as (n: number) => number),
  absentMember: flow |> inc |> absent?.by,
  absentNested: flow |> inc |> absent?.inner.by,
  absentComputed: flow |> inc |> absent?.[key],
  bound: flow |> inc |> present!.by,
  later: flow |> inc |> present?.by |> String,
  once: flow |> inc |> receiver(present)?.by,
};
for (const [name, composed] of Object.entries(steps)) {
  try {
    console.log(name, composed(1), composed(2));
  } catch (error) {
    console.log(name, "threw", (error as Error).message);
  }
}
console.log(seen.join(","));
console.log("pipeline", 1 |> inc |> present?.by, 1 |> inc |> present?.inner.by, 1 |> inc |> absent?.by);


//// [flowOptionalMemberStep.ts]
var $tt_fl: <A extends unknown[], B, C>(
  f: (...a: A) => B,
  g: (b: B) => C,
) => (...a: A) => C = function (f, g) {
  return (...a) => g(f(...a));
};
// An optional-chain step in a `flow` composition is the optional call
// `obj?.m(v)`, as it is in a pipeline: it calls the method on its receiver
// and gives `undefined` when the chain short-circuits. Its receiver is
// evaluated once, where the step is written, as a member step's is.
const inc = (n: number) => n + 1;
class Scale {
  constructor(readonly k: number) {}
  by(n: number) { return n * this.k; }
  inner = { k: 3, by(n: number) { return n * this.k; } };
}
const present = new Scale(10) as Scale | undefined;
const absent = undefined as Scale | undefined;
const key = "by" as const;
const seen: string[] = [];
function receiver(scale: Scale | undefined) {
  seen.push("receiver");
  return scale;
}
const steps = {
  member: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.by($tt_v)))(inc, (present)),
  nested: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.inner.by($tt_v)))(inc, (present)),
  computed: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.[key]($tt_v)))(inc, (present)),
  asserted: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => ($tt_r?.by as (n: number) => number)($tt_v)))(inc, (present)),
  absentMember: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.by($tt_v)))(inc, (absent)),
  absentNested: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.inner.by($tt_v)))(inc, (absent)),
  absentComputed: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.[key]($tt_v)))(inc, (absent)),
  bound: $tt_fl(inc, (($tt_r) => ($tt_r.by).bind($tt_r))((present!))),
  later: (($tt_g, $tt_f) => $tt_fl($tt_g, ($tt_v) => $tt_f($tt_v)))((($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.by($tt_v)))(inc, (present)), String),
  once: (($tt_f, $tt_r) => $tt_fl($tt_f, ($tt_v) => $tt_r?.by($tt_v)))(inc, (receiver(present))),
};
for (const [name, composed] of Object.entries(steps)) {
  try {
    console.log(name, composed(1), composed(2));
  } catch (error) {
    console.log(name, "threw", (error as Error).message);
  }
}
console.log(seen.join(","));
console.log("pipeline", (($tt_v, $tt_r) => $tt_r?.by($tt_v))(inc(1), (present)), (($tt_v, $tt_r) => $tt_r?.inner.by($tt_v))(inc(1), (present)), (($tt_v, $tt_r) => $tt_r?.by($tt_v))(inc(1), (absent)));
