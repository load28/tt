//// [runtimeReferenceProtocolPreservesOptionalAndTaggedCalls.tt] ////

variant E { A(value: number), B }
const events: string[] = [];
const receiver = {
  get method() {
    events.push("method");
    return function (this: unknown, value: number) {
      events.push(`call:${this === receiver}:${value}`);
      return value;
    };
  },
  get tag() {
    events.push("tag");
    return function (this: unknown, strings: TemplateStringsArray, value: number) {
      events.push(`tag-call:${this === receiver}:${value}`);
      return (strings[0] ?? "") + value;
    };
  },
};
const absent: { method: ((value: number) => number) | null } = {
  get method() { return null as ((value: number) => number) | null; },
};
function effect(value: E): E {
  events.push("subject");
  return value;
}
const present = receiver.method?.(
  match (effect(E.A(2))) { A(value) => value, B => 0 },
);
const missing = absent.method?.(
  match (effect(E.A(3))) { A(value) => value, B => 0 },
);
const tagged = receiver.tag`value:${match (effect(E.A(4))) {
  A(value) => value,
  B => 0,
}}`;
console.log(events.join(","));
console.log(present, missing, tagged);

export {};


//// [runtimeReferenceProtocolPreservesOptionalAndTaggedCalls.ts]
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

type E =
  | { kind: "A"; value: number }
  | { kind: "B" };
const E = {
  A: (value: number): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
const events: string[] = [];
const receiver = {
  get method() {
    events.push("method");
    return function (this: unknown, value: number) {
      events.push(`call:${this === receiver}:${value}`);
      return value;
    };
  },
  get tag() {
    events.push("tag");
    return function (this: unknown, strings: TemplateStringsArray, value: number) {
      events.push(`tag-call:${this === receiver}:${value}`);
      return (strings[0] ?? "") + value;
    };
  },
};
const absent: { method: ((value: number) => number) | null } = {
  get method() { return null as ((value: number) => number) | null; },
};
function effect(value: E): E {
  events.push("subject");
  return value;
}
let $tt_v2: (number) | (undefined);
const $tt_v1 = (receiver.method);
if ($tt_v1 != null) {
  {
    const $tt_m = effect(E.A(2));
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v2 = $tt_v1.call(receiver, value);
        break;
      }
      case "B": {
        $tt_v2 = $tt_v1.call(receiver, 0);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v2 = undefined;
}

const present = $tt_v2;
let $tt_v5: (number) | (undefined);
const $tt_v4 = (absent.method);
if ($tt_v4 != null) {
  {
    const $tt_m = effect(E.A(3));
    switch ($tt_m.kind) {
      case "A": {
        const { value } = $tt_m;
        $tt_v5 = $tt_v4.call(absent, value);
        break;
      }
      case "B": {
        $tt_v5 = $tt_v4.call(absent, 0);
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
} else {
  $tt_v5 = undefined;
}

const missing = $tt_v5;
let $tt_v6: number;
{
  const $tt_m = effect(E.A(4));
  switch ($tt_m.kind) {
    case "A": {
      const { value } = $tt_m;
      $tt_v6 = value;
      break;
    }
    case "B": {
      $tt_v6 = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const tagged = receiver.tag`value:${$tt_v6}`;
console.log(events.join(","));
console.log(present, missing, tagged);

export {};
