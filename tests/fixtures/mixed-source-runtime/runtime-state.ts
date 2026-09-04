export const trace: string[] = [];

export function record(value: string): string {
  trace.push(value);
  return value;
}
