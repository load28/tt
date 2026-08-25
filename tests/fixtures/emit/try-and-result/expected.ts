import type { TResult } from "@tt/std";
import * as Result from "@tt/std/result";

declare function getUser(id: string): TResult<{ name: string; companyId: string }, string>;
declare function getCompany(id: string): TResult<{ title: string }, string>;

export function load(id: string): TResult<{ name: string; title: string }, string> {
  let $tt_v0;
  do {
    const $tt_r0 = getUser(id); if ($tt_r0.kind !== "Ok") { $tt_v0 = $tt_r0; break; } const user = $tt_r0.value;
    const name = user.name;
    const $tt_r1 = getCompany(user.companyId); if ($tt_r1.kind !== "Ok") { $tt_v0 = $tt_r1; break; } const company = $tt_r1.value;
    $tt_v0 = { kind: "Ok" as const, value: { name, title: company.title } }; break;
  } while (false);
  const both = $tt_v0;
  const $tt_t2 = both; if ($tt_t2.kind !== "Ok") return $tt_t2; const value = $tt_t2.value;
  return Result.Ok(value);
}
