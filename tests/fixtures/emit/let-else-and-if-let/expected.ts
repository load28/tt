import type { TOption } from "@tt/std";

declare function find(id: string): TOption<string>;

export function greet(id: string): string {
  const $tt_t0 = find(id); if ($tt_t0.kind !== "Some") { return "who?"; } const { value: name } = $tt_t0;
  { const $tt_t1 = find(name); if ($tt_t1.kind === "Some") { const { value: other } = $tt_t1; return `${name} and ${other}`; } else { return name; } }
}
