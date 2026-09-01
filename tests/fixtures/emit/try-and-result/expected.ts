import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";

declare function getUser(id: string): TResult<{ name: string; companyId: string }, string>;
declare function getCompany(id: string): TResult<{ title: string }, string>;

export function load(id: string): TResult<{ name: string; title: string }, string> {
  let $tt_v0;
  $tt_v0: {
    const $tt_t0 = getUser(id);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const user = $tt_t0.value;
    const name = user.name;
    const $tt_t1 = getCompany(user.companyId);
    if (!("value" in $tt_t1)) {
      $tt_v0 = $tt_t1;
      break $tt_v0;
    }
    const company = $tt_t1.value;
    {
      $tt_v0 = { kind: "Ok" as const, value: { name, title: company.title } };
      break $tt_v0;
    }
  }
  const both = $tt_v0;
  const $tt_t2 = both;
  if (!("value" in $tt_t2)) {
    return $tt_t2;
  }
  const value = $tt_t2.value;
  return Result.Ok(value);
}
