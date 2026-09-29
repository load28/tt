import * as path from "node:path";

export function isWithin(ancestor: string, dir: string): boolean {
  const relative = path.relative(ancestor, dir);
  return (
    relative === "" ||
    (relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative))
  );
}
