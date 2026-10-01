//// [missingCasesAreAllListed.tt] ////
variant Dir { North, South, East, West(deg: number) }
const f = (d: Dir) => match (d) { North => 1 };

