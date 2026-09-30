//// [asyncGeneratorTtConstructs.tt] ////
// `await` and `yield` inside tt constructs suspend the surrounding async
// function or generator, and interleave with other work as JavaScript does.
import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";
variant Job { Fetch(id: number), Skip }
const log: string[] = [];
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
async function later<T>(label: string, value: T): Promise<T> {
  log.push(`${label} start`);
  await null;
  log.push(`${label} end`);
  return value;
}
async function run(job: Job) {
  return match (await later("scrutinee", job)) {
    Fetch(id) if await later("guard", id > 1) => await later("big", id * 100),
    Fetch(id) => await later("small", id),
    Skip => "skipped",
  };
}
async function load(ok: boolean): Promise<TResult<number, string>> {
  const n = try await later("load", ok ? Result.Ok(1) : Result.Err("no"));
  log.push("after load");
  return Result.Ok(n + 1);
}
function* totals(jobs: Job[]): Generator<string, number, number> {
  let total = 0;
  for (const job of jobs) {
    total += match (job) {
      Fetch(id) => yield `fetch ${id}`,
      Skip => 0,
    };
  }
  return total;
}
function* scrutinees(): Generator<string, string[], Job> {
  const seen: string[] = [];
  for (let i = 0; i < 2; i++) {
    seen.push(match (yield `ask ${i}`) { Fetch(id) => `got ${id}`, Skip => "got skip" });
  }
  return seen;
}
async function* stream(jobs: Job[]) {
  for (const job of jobs) {
    yield match (job) { Fetch(id) => await later(`fetch ${id}`, id), Skip => -1 };
  }
}
flush("run Fetch(2)", await run(Job.Fetch(2)));
flush("run Fetch(1)", await run(Job.Fetch(1)));
flush("run Skip", await run(Job.Skip));
const tick = Promise.resolve().then(() => log.push("tick"));
flush("interleaved", await Promise.all([run(Job.Fetch(3)), tick, run(Job.Skip)]));
flush("load ok", await load(true));
flush("load err", await load(false));
const generator = totals([Job.Fetch(1), Job.Skip, Job.Fetch(2)]);
let next = generator.next(0);
while (!next.done) {
  log.push(next.value);
  next = generator.next(10);
}
flush("generator total", next.value);
const asker = scrutinees();
log.push(String(asker.next(Job.Skip).value));
log.push(String(asker.next(Job.Fetch(5)).value));
flush("yield in scrutinee", asker.next(Job.Skip).value);
const streamed: number[] = [];
for await (const n of stream([Job.Fetch(7), Job.Skip, Job.Fetch(8)])) {
  streamed.push(n);
}
flush("async generator", streamed);
export {};

//// [tt/index.ts] support module @tt/std/index.ts
//// [tt/option.ts] support module @tt/std/option.ts
//// [tt/result.ts] support module @tt/std/result.ts

//// [asyncGeneratorTtConstructs.ts]
function $tt_show(value: unknown): string {
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
}
// `await` and `yield` inside tt constructs suspend the surrounding async
// function or generator, and interleave with other work as JavaScript does.
import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";
type Job =
  | { kind: "Fetch"; id: number }
  | { kind: "Skip" };
const Job = {
  Fetch: (id: number): Job => ({ kind: "Fetch", id }),
  Skip: { kind: "Skip" } as const,
};
const log: string[] = [];
function flush(title: string, value: unknown) {
  console.log(`${title}: ${JSON.stringify(value)} after ${log.join(" ")}`);
  log.length = 0;
}
async function later<T>(label: string, value: T): Promise<T> {
  log.push(`${label} start`);
  await null;
  log.push(`${label} end`);
  return value;
}
async function run(job: Job) {
  let $tt_v0: (number) | (string);
  {
    const $tt_m = await later("scrutinee", job);
    do {
      if ($tt_m.kind === "Fetch") {
        const { id } = $tt_m;
        if (await later("guard", id > 1)) {
          $tt_v0 = await later("big", id * 100);
          break;
        }
      }
      if ($tt_m.kind === "Fetch") {
        const { id } = $tt_m;
        $tt_v0 = await later("small", id);
        break;
      }
      if ($tt_m.kind === "Skip") {
        $tt_v0 = "skipped";
        break;
      }
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    } while (false);
  }
  return $tt_v0;
}
async function load(ok: boolean): Promise<TResult<number, string>> {
  const $tt_t0 = await later("load", ok ? Result.Ok(1) : Result.Err("no"));
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  const n = $tt_t0.value;
  log.push("after load");
  return Result.Ok(n + 1);
}
function* totals(jobs: Job[]): Generator<string, number, number> {
  let total = 0;
  for (const job of jobs) {
    let $tt_v1: number;
    let $tt_v2 = (total);
    {
      const $tt_m = job;
      switch ($tt_m.kind) {
        case "Fetch": {
          const { id } = $tt_m;
          $tt_v1 = yield `fetch ${id}`;
          break;
        }
        case "Skip": {
          $tt_v1 = 0;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    total = $tt_v2 += $tt_v1;
  }
  return total;
}
function* scrutinees(): Generator<string, string[], Job> {
  const seen: string[] = [];
  for (let i = 0; i < 2; i++) {
    let $tt_v3: string;
    {
      const $tt_m = yield `ask ${i}`;
      switch ($tt_m.kind) {
        case "Fetch": {
          const { id } = $tt_m;
          $tt_v3 = `got ${id}`;
          break;
        }
        case "Skip": {
          $tt_v3 = "got skip";
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    seen.push($tt_v3);
  }
  return seen;
}
async function* stream(jobs: Job[]) {
  for (const job of jobs) {
    let $tt_v5: number;
    {
      const $tt_m = job;
      switch ($tt_m.kind) {
        case "Fetch": {
          const { id } = $tt_m;
          $tt_v5 = await later(`fetch ${id}`, id);
          break;
        }
        case "Skip": {
          $tt_v5 = -1;
          break;
        }
        default: {
          throw new Error("tt match: unexpected case " + $tt_show($tt_m));
        }
      }
    }
    yield $tt_v5;
  }
}
flush("run Fetch(2)", await run(Job.Fetch(2)));
flush("run Fetch(1)", await run(Job.Fetch(1)));
flush("run Skip", await run(Job.Skip));
const tick = Promise.resolve().then(() => log.push("tick"));
flush("interleaved", await Promise.all([run(Job.Fetch(3)), tick, run(Job.Skip)]));
flush("load ok", await load(true));
flush("load err", await load(false));
const generator = totals([Job.Fetch(1), Job.Skip, Job.Fetch(2)]);
let next = generator.next(0);
while (!next.done) {
  log.push(next.value);
  next = generator.next(10);
}
flush("generator total", next.value);
const asker = scrutinees();
log.push(String(asker.next(Job.Skip).value));
log.push(String(asker.next(Job.Fetch(5)).value));
flush("yield in scrutinee", asker.next(Job.Skip).value);
const streamed: number[] = [];
for await (const n of stream([Job.Fetch(7), Job.Skip, Job.Fetch(8)])) {
  streamed.push(n);
}
flush("async generator", streamed);
export {};
