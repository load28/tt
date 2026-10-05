const before = String(1);
declare const value: string;
declare const flag: boolean;
const broken: ? `${value}` = 1;
export const later: number = flag ? 1 : 2;
const wrong: string = later;
/*later*/later;
