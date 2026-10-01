//// [valNeverCallsAMethodAMutationFromItsName2.tt] ////
class Collection {
  add(v: number): Collection {
    return new Collection();
  }
}
val const collection = new Collection();
collection.add(1);


//// [valNeverCallsAMethodAMutationFromItsName2.ts]
class Collection {
  add(v: number): Collection {
    return new Collection();
  }
}
const collection = new Collection();
collection.add(1);
