//// [letElseBindingAliasAndKeyword.tt] ////
function f(): string {
  let Some(value: user) = find() else { throw new Error("none"); };
  return user;
}


//// [letElseBindingAliasAndKeyword.ts]
function f(): string {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    throw new Error("none");
  }
  let { value: user } = $tt_t0;
  return user;
}
