//// [resultBlockUnionsTheErrorTypesOfItsBindings.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }
variant UserError { NoUser() }
variant CompanyError { NoCompany(id: number) }
type User = { id: number; name: string; companyId: number };
type Company = { id: number; name: string };
declare function getUser(id: number): Res<User, UserError>;
declare function getCompany(id: number): Res<Company, CompanyError>;

const view = (id: number): Res<string, UserError | CompanyError> => result {
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  return user.name + "@" + company.name;
};

export {};


//// [resultBlockUnionsTheErrorTypesOfItsBindings.ts]

type Res<T, E> =
  | { kind: "Ok"; value: T }
  | { kind: "Err"; error: E };
const Res = {
  Ok: <T, E>(value: T): Res<T, E> => ({ kind: "Ok", value }),
  Err: <T, E>(error: E): Res<T, E> => ({ kind: "Err", error }),
};
type UserError =
  { kind: "NoUser" };
const UserError = {
  NoUser: (): UserError => ({ kind: "NoUser" }),
};
type CompanyError =
  { kind: "NoCompany"; id: number };
const CompanyError = {
  NoCompany: (id: number): CompanyError => ({ kind: "NoCompany", id }),
};
type User = { id: number; name: string; companyId: number };
type Company = { id: number; name: string };
declare function getUser(id: number): Res<User, UserError>;
declare function getCompany(id: number): Res<Company, CompanyError>;

const view = (id: number): Res<string, UserError | CompanyError> => {
  let $tt_v0: Res<string, UserError | CompanyError>;
  $tt_v0: {
    const $tt_t0 = getUser(id);
    if (!("value" in $tt_t0)) {
      $tt_v0 = $tt_t0;
      break $tt_v0;
    }
    const user = $tt_t0.value;
  const $tt_t1 = getCompany(user.companyId);
  if (!("value" in $tt_t1)) {
    $tt_v0 = $tt_t1;
    break $tt_v0;
  }
  const company = $tt_t1.value;
  {
    $tt_v0 = { kind: "Ok" as const, value: user.name + "@" + company.name };
    break $tt_v0;
  }
  }
  return $tt_v0;
};

export {};
