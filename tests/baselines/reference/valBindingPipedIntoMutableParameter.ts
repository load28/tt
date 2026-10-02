//// [valBindingPipedIntoMutableParameter.tt] ////
// `x |> f` is the call `f(x)`, so a val binding piped into a same-file
// function whose parameter is not `val` is the same `val-pass` error as
// `touch(cfg)`. A `flow` composition hands its argument to its first step,
// so applying one to the binding (`(flow |> touch)(cfg)`) or piping the
// binding into one (`cfg |> (flow |> touch)`) passes it to `touch` too.
// The pipeline forms were not judged at all: only the written call syntax
// was, while a postfix step (`items |> .push(1)`) is already judged as the
// member call it is. Each piped line below must report `val-pass` at
// `cfg`, as `touch(cfg)` / `arrowTouch(cfg)` do; the lines that pass it to
// `look`, whose parameter is `val`, or pipe a value read from it
// (`cfg.n |> String`) are fine.
function touch(o: { n: number }) {
  o.n = 1;
  return o;
}
const arrowTouch = (o: { n: number }) => {
  o.n = 2;
};
function look(val o: { n: number }) {
  return o.n;
}
export function f(val cfg: { n: number }) {
  cfg |> touch;
  cfg |> arrowTouch;
  cfg |> touch |> String;
  return cfg.n;
}
export function g(val cfg: { n: number }) {
  cfg |> (flow |> touch |> String);
  (flow |> touch)(cfg);
  (flow |> (flow |> touch) |> String)(cfg);
  const text = `${cfg |> touch}`;
  cfg.n |> String;
  cfg |> look;
  (flow |> look)(cfg);
  return text;
}

