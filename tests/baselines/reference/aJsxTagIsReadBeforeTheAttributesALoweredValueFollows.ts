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
variant O { A(n: number), B }
const log: string[] = [];
const React = {
  createElement(tag: { name: string }, _props: unknown, ..._children: unknown[]) {
    log.push(`create ${tag.name}`);
    return null;
  },
};
function Old(_p: { x: number; y?: number; children?: unknown }) { return null; }
function New(_p: { x: number; y?: number; children?: unknown }) { return null; }
const lib = { get inner() { log.push("get inner"); return { Old }; } };
function t<T>(label: string, value: T): T { log.push(label); return value; }
let Comp = Old;
function selfClosing(o: O) {
  return <lib.inner.Old y={t("y", 1)} x={match (o) { A(n) => (log.push("arm"), n), B => 0 }} />;
}
function withChildren(o: O) {
  return <lib.inner.Old y={t("y", 1)} x={match (o) { A(n) => (log.push("arm"), n), B => 0 }}>{t("child", 2)}</lib.inner.Old>;
}
function reassigned(o: O) {
  return <Comp x={match (o) { A(n) => (Comp = New, n), B => 0 }} />;
}
selfClosing(O.A(1));
withChildren(O.A(1));
reassigned(O.A(1));
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
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const log: string[] = [];
const React = {
  createElement(tag: { name: string }, _props: unknown, ..._children: unknown[]) {
    log.push(`create ${tag.name}`);
    return null;
  },
};
function Old(_p: { x: number; y?: number; children?: unknown }) { return null; }
function New(_p: { x: number; y?: number; children?: unknown }) { return null; }
const lib = { get inner() { log.push("get inner"); return { Old }; } };
function t<T>(label: string, value: T): T { log.push(label); return value; }
let Comp = Old;
function selfClosing(o: O) {
  let $tt_v0: number;
  const $tt_v1 = (lib.inner.Old);
  const $tt_v2 = (t("y", 1));
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = (log.push("arm"), n);
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return <$tt_v1 y={$tt_v2} x={$tt_v0} />;
}
function withChildren(o: O) {
  let $tt_v3: number;
  const $tt_v4 = (lib.inner.Old);
  const $tt_v5 = (t("y", 1));
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v3 = (log.push("arm"), n);
        break;
      }
      case "B": {
        $tt_v3 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return <$tt_v4 y={$tt_v5} x={$tt_v3}>{t("child", 2)}</$tt_v4>;
}
function reassigned(o: O) {
  let $tt_v6: number;
  const $tt_v7 = (Comp);
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v6 = (Comp = New, n);
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
  return <$tt_v7 x={$tt_v6} />;
}
selfClosing(O.A(1));
withChildren(O.A(1));
reassigned(O.A(1));
console.log(log.join(", "));
