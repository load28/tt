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
export function box<T>(value: T): { inner: { value: T } } | undefined {
  return { inner: { value } };
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
