//// [valJudgesOnlyPathsRootedAtTheBinding2.tt] ////
val const config = { debug: false };
class K {
  config = { debug: false };
  m() { this.config.debug = true; }
}


//// [valJudgesOnlyPathsRootedAtTheBinding2.ts]
const config = { debug: false };
class K {
  config = { debug: false };
  m() { this.config.debug = true; }
}
