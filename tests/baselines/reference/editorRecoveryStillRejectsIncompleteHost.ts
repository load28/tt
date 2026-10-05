//// [missingInitializer.tt] ////
declare const flag: boolean;
const broken = ;
const later = match (flag) { true => 1, false => 2 };

//// [missingDelimiter.tt] ////
declare const flag: boolean;
const broken = f(
const later = match (flag) { true => 1, false => 2 };

