export const out: string[] = [];
export function text(value: unknown): string {
  if (typeof value === "bigint") return `${value}n`;
  if (typeof value === "function") return "function";
  if (value === undefined) return "undefined";
  return JSON.stringify(value);
}
export function note<T>(label: string, value: T): T {
  out.push(`${label} ${text(value)}`);
  return value;
}
export function unexpected(value: unknown): never {
  throw new Error(`tt match: unexpected literal ${text(value)}`);
}
export function later<T>(value: T): Promise<T> {
  out.push("await");
  return Promise.resolve(value);
}
let flips = 0;
export function flip(): boolean {
  return note("flip", flips++ % 2 === 0);
}
export function pack(...items: unknown[]): unknown[] {
  return items;
}
export function callee(present: boolean): typeof pack | undefined {
  return note("callee", present) ? pack : undefined;
}
export function first<T>(...items: T[]): T {
  return items[0]!;
}
let risks = 0;
export function risky<T>(value: T): T {
  if (++risks % 3 === 0) throw new Error(`risky call ${risks}`);
  return note("risky", value);
}
let boxes = 0;
export function box<T>(value: T): { inner: { value: T } } | undefined {
  return boxes++ % 2 === 0 ? { inner: { value } } : undefined;
}
export const double = (n: number): number => note("double", n * 2);
export const adder = (a: number) => (b: number): number => note("add", a + b);
export const tools = {
  factor: 3,
  twice(n: number): number {
    return note("twice", n * this.factor);
  },
};
export function maybe(present: boolean): typeof tools | undefined {
  return note("maybe", present) ? tools : undefined;
}
export function lookup(present: boolean): ((n: number) => number) | undefined {
  return note("lookup", present) ? (n) => note("found", n - 1) : undefined;
}
export function positive(n: number): number | undefined {
  return note("positive", n > 0 ? n : undefined);
}
export function resource(name: string): Disposable {
  out.push(`open ${name}`);
  return { [Symbol.dispose]: () => void out.push(`dispose ${name}`) };
}
export type Success<T> = { kind: "Ok"; value: T };
export type Failure<E> = { kind: "Err"; error: E };
export const ok = <T>(value: T): Success<T> => ({ kind: "Ok", value });
export const err = <E>(error: E): Failure<E> => ({ kind: "Err", error });
export function read(input: number): Success<number> | Failure<string> {
  return note("read", input) >= 0 ? ok(input) : err(`negative ${input}`);
}
export class Propagate {
  constructor(readonly result: unknown) {}
}
export function unwrap(result: any): any {
  if ("value" in result) return result.value;
  throw new Propagate(result);
}
export function caught(error: unknown): any {
  if (error instanceof Propagate) return error.result;
  throw error;
}
export function report(title: string, input: unknown, shown: string): void {
  console.log(`${title}(${text(input)}) => ${shown} | ${out.join("; ")}`);
  out.length = 0;
}
export async function drive(title: string, probe: (input: any) => unknown, inputs: readonly unknown[]): Promise<void> {
  for (const input of inputs) {
    let shown: string;
    try {
      let value: any = probe(input);
      if (value !== null && typeof value === "object" && typeof value.next === "function" && Symbol.iterator in value) {
        let step = value.next();
        while (!step.done) {
          out.push(`yield ${text(step.value)}`);
          step = value.next(step.value);
        }
        value = step.value;
      }
      if (value instanceof Promise) value = await value;
      if (typeof value === "function") value = value(input);
      shown = text(value);
    } catch (error) {
      shown = `threw ${error instanceof Error ? error.message : text(error)}`;
    }
    report(title, input, shown);
  }
}
export function element(tag: string | ((props: any) => string), props: Record<string, unknown> | null, ...children: unknown[]): string {
  const quoted = (value: unknown) => text(value).replaceAll('"', "'");
  const content = children
    .map((child) => (child === null || child === undefined || typeof child === "boolean" ? "" : typeof child === "string" ? child : quoted(child)))
    .join("");
  if (typeof tag === "function") return tag({ ...props, children: content });
  const attributes = Object.entries(props ?? {})
    .map(([name, value]) => ` ${name}=${quoted(value)}`)
    .join("");
  return `<${tag}${attributes}>${content}</${tag}>`;
}
export declare namespace element {
  namespace JSX {
    type Element = string;
    interface IntrinsicElements {
      [name: string]: unknown;
    }
  }
}
export const Fragment = "";
export function Show(props: { value: unknown }): string {
  return `<Show ${text(props.value).replaceAll('"', "'")}>`;
}
