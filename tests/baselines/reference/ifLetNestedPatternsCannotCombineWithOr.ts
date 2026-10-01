//// [ifLetNestedPatternsCannotCombineWithOr.tt] ////
function g(o: X): number {
  if let Some(value: Ok(v)) | None() = o {
    return 1;
  }
  return 0;
}

