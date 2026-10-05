const before = String(1);
declare const value: string;
declare const flag: boolean;
cnst broken = `${value}text${`nested${value}`}`;
export const later: number = flag ? 1 : 2;
const wrong: string = later;
/*later*/later;
