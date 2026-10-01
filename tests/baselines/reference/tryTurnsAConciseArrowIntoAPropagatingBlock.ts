//// [tryTurnsAConciseArrowIntoAPropagatingBlock.tt] ////
const f = (): TResult<number, string> => Result.Ok(try read());


//// [tryTurnsAConciseArrowIntoAPropagatingBlock.ts]
const f = (): TResult<number, string> => {
  let $tt_v0;
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  return Result.Ok($tt_v0);
};
