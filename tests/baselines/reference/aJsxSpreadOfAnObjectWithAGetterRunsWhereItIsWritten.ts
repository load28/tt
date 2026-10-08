//// [tsconfig.json] ////
{
  "compilerOptions": {
    "target": "es2022",
    "module": "preserve",
    "moduleResolution": "bundler",
    "jsx": "react",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true
  }
}

//// [main.ttx] ////
variant S { A, B }
const log: string[] = [];
const React = {
  createElement(tag: { name: string }, _props: unknown, ..._children: unknown[]) {
    log.push(`create ${tag.name}`);
    return null;
  },
};
function o(): S { log.push("subject"); return S.A; }
function Comp(_p: { k: number; z: number }) { return null; }
function main() {
  return <Comp {...{ get k() { log.push("k"); return 1; } }} z={match (o()) { A => 1, B => 2 }} />;
}
main();
console.log(log.join(", "));


//// [main.tsx]
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
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = [];
const React = {
  createElement(tag: { name: string }, _props: unknown, ..._children: unknown[]) {
    log.push(`create ${tag.name}`);
    return null;
  },
};
function o(): S { log.push("subject"); return S.A; }
function Comp(_p: { k: number; z: number }) { return null; }
function main() {
  let $tt_v0: number;
  const $tt_v1 = (Comp);
  const $tt_v2 = ({ ...{ get k() { log.push("k"); return 1; } } });
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return <$tt_v1 {...$tt_v2} z={($tt_v0 === 0 ? 1 : 2)} />;
}
main();
console.log(log.join(", "));
