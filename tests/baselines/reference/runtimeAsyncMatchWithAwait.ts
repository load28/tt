//// [runtimeAsyncMatchWithAwait.tt] ////

variant Job {
  Fetch(n: number),
  Idle,
}

async function double(n: number): Promise<number> {
  return n * 2;
}

async function runJob(j: Job): Promise<number> {
  return match (j) {
    Fetch(n) => await double(n),
    Idle => 0,
  };
}

runJob(Job.Fetch(21)).then((a) => {
  console.log(a);
  return runJob(Job.Idle);
}).then((b) => {
  console.log(b);
});

export {};


//// [runtimeAsyncMatchWithAwait.ts]
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

type Job =
  | { kind: "Fetch"; n: number }
  | { kind: "Idle" };
const Job = {
  Fetch: (n: number): Job => ({ kind: "Fetch", n }),
  Idle: { kind: "Idle" } as const,
};

async function double(n: number): Promise<number> {
  return n * 2;
}

async function runJob(j: Job): Promise<number> {
  let $tt_v0: Awaited< Promise<number>>;
  {
    const $tt_m = j;
    switch ($tt_m.kind) {
      case "Fetch": {
        const { n } = $tt_m;
        $tt_v0 = await double(n);
        break;
      }
      case "Idle": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}

runJob(Job.Fetch(21)).then((a) => {
  console.log(a);
  return runJob(Job.Idle);
}).then((b) => {
  console.log(b);
});

export {};
