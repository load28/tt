import { record } from "./runtime-state.js";

export function produceTs(): string {
  return record("ts");
}
