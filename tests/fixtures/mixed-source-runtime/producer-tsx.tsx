import { record } from "./runtime-state.js";

export function produceTsx(): string {
  return record("tsx");
}
