export function unwrap<T>(result: Success<T> | Failure<unknown>): T {
  if (result.kind === "Ok") return result.value;
  throw new Propagate(result);
}
