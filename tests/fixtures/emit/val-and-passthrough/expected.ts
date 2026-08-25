export interface Config {
  readonly name: string;
}

// Plain TypeScript passes through byte for byte; `val` is a tt-level
// promise the emission erases.
export function read(config: Config): string {
  const parts = config.name.split(",");
  return parts.map((p) => p.trim()).join("|");
}

const shared = { count: 0 };
export const count = shared.count;
