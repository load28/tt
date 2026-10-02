//// [stdResultAndThenErrorUnionStaysCheckedAgainstAnAnnotation.tt] ////

import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type Exact<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

type User = { id: number };
type Company = { name: string };
type Profile = { title: string };
type ConfigError = { tag: "config" };
type TokenError = { tag: "token" };
type FetchError = { tag: "fetch" };
type ValidationError = { tag: "validation" };

declare function loadConfig(): TResult<string, ConfigError>;
declare function loadToken(config: string): TResult<User, TokenError>;
declare function getCompany(user: User): TResult<Company, FetchError>;
declare function fetchProfile(user: User): TResult<Profile, FetchError>;
declare function validateProfile(profile: Profile): TResult<Profile, ValidationError>;

declare const first: TResult<User, TokenError>;

function chain(): TResult<Profile, TokenError> {
  return Result.andThen(first, (user) => fetchProfile(user));
}

console.log(chain());

export {};


//// [stdResultAndThenErrorUnionStaysCheckedAgainstAnAnnotation.ts]

import type { TResult } from "./tt/index.js";
import * as Result from "./tt/result.js";

type Exact<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

type User = { id: number };
type Company = { name: string };
type Profile = { title: string };
type ConfigError = { tag: "config" };
type TokenError = { tag: "token" };
type FetchError = { tag: "fetch" };
type ValidationError = { tag: "validation" };

declare function loadConfig(): TResult<string, ConfigError>;
declare function loadToken(config: string): TResult<User, TokenError>;
declare function getCompany(user: User): TResult<Company, FetchError>;
declare function fetchProfile(user: User): TResult<Profile, FetchError>;
declare function validateProfile(profile: Profile): TResult<Profile, ValidationError>;

declare const first: TResult<User, TokenError>;

function chain(): TResult<Profile, TokenError> {
  return Result.andThen(first, (user) => fetchProfile(user));
}

console.log(chain());

export {};
