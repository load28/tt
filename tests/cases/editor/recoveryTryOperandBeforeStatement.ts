import * as Result from "@tt/std/result";
import type { TResult } from "@tt/std";
declare function parseNum(x: string): TResult<number, string>;
export function rect(a: string, b: string): TResult<number, string> {
  const width = unwrap(parseNum(a));
  const height = unwrap(parseNum(
  if (width <= 0) {
    return Result.Err("width must be positive");
  }
  return Result.Ok(/*width*/width * height);
}
declare function unwrap<T>(r: TResult<T, string>): T;
