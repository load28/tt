//// [valNeverCallsAMethodAMutationFromItsName1.tt] ////
class Query {
  set(key: string): Query {
    return new Query();
  }
}
val const query = new Query();
query.set("name");


//// [valNeverCallsAMethodAMutationFromItsName1.ts]
class Query {
  set(key: string): Query {
    return new Query();
  }
}
const query = new Query();
query.set("name");
