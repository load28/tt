declare const flag: boolean;
declare function f(...xs: unknown[]): unknown;
const broken = f(
const a = (flag ? 1 : 2);
const b = (flag ? 3 : 4);
const later = "정상";
const wrong: number = "wrong";
later./*member*/toUpperCase();
/*later*/later;
