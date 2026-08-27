import { $tt_ap } from "@tt/runtime";
declare const user: { profile?: { name: string } } | undefined;
declare const key: "profile";

export const name = user?.[key]?.name;
export const upper = $tt_ap(name?.toUpperCase(), (value => value ?? "unknown"));
