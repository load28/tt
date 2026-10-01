//// [valParameterPositionsBeyondPlainIdentifiers4.tt] ////
class B {
  constructor(private val inner: I) {
    inner.a = 1;
  }
}

