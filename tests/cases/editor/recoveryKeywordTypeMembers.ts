type T = { return; const
let: number; var?(): void; throw: number; export: number; else: number };
interface I { return; const
let: number; var?(): void; throw: number; export: number; else: number }
declare const t: T;
declare const i: I;
declare const flag: boolean;
const broken = ;
export const good = flag ? t.let : i.let;
const wrong: number = "wrong";
const later = "정상";
later./*member*/toUpperCase();
/*later*/later;
