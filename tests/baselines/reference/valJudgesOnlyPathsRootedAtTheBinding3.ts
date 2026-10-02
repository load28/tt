//// [valJudgesOnlyPathsRootedAtTheBinding3.tt] ////
val const o = { x: 1 };
const q = { o: { x: 1 } };
q.o.x++;
q!.o.x = 2;
delete q.o.x;
[q.o.x] = [3];


//// [valJudgesOnlyPathsRootedAtTheBinding3.ts]
const o = { x: 1 };
const q = { o: { x: 1 } };
q.o.x++;
q!.o.x = 2;
delete q.o.x;
[q.o.x] = [3];
