//// [resultBlockBindingsAreNarrowedSuccessValues.tt] ////

variant Res<T, E> { Ok(value: T), Err(error: E) }
variant UserError { NoUser() }
variant CompanyError { NoCompany(id: number) }
type User = { id: number; name: string; companyId: number };
type Company = { id: number; name: string };
declare function getUser(id: number): Res<User, UserError>;
declare function getCompany(id: number): Res<Company, CompanyError>;

const view = (id: number) => result {
  const user = try getUser(id);
  const company = try getCompany(user.companyId);
  const label: string = user.name.toUpperCase() + company.name;
  return { user, company, label };
};
const check = (id: number): string => match (view(id)) {
  Ok(value) => value.label,
  Err(error) => match (error) {
    NoUser => "no user",
    NoCompany(id: missing) => "no company " + missing,
  },
};

export {};


//// [resultBlockBindingsAreNarrowedSuccessValues.ts]
function $tt_show(value: unknown): string {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
}

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

const view = (id: number) => {
  let $tt_v0: ({
    kind: "Err";
    error: UserError;
}) | ({
    kind: "Err";
    error: CompanyError;
}) | ({
    kind: "Ok";
    value: {
        user: User;
        company: Company;
        label: string;
    };
});
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
  const label: string = user.name.toUpperCase() + company.name;
  {
    $tt_v0 = { kind: "Ok" as const, value: { user, company, label } };
    break $tt_v0;
  }
  }
  return $tt_v0;
};
const check = (id: number): string => {
  let $tt_v1: string;
  {
    const $tt_m = view(id);
    switch ($tt_m.kind) {
      case "Ok": {
        const { value } = $tt_m;
        $tt_v1 = value.label;
        break;
      }
      case "Err": {
        const { error } = $tt_m;
        {
          const $tt_m = error;
          switch ($tt_m.kind) {
            case "NoUser": {
              $tt_v1 = "no user";
              break;
            }
            case "NoCompany": {
              const { id: missing } = $tt_m;
              $tt_v1 = "no company " + missing;
              break;
            }
            default: {
              throw new Error("tt match: unexpected case " + $tt_show($tt_m));
            }
          }
        }
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v1;
};

export {};
