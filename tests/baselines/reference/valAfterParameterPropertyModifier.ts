//// [valAfterParameterPropertyModifier.tt] ////
// `val` after a parameter property's accessibility or `readonly` modifier
// (`constructor(public val p: T)`) is the parameter's modifier: plain ttc
// erases it, and the typed check pairs the constructor's references with
// that parameter, so a write or a built-in mutator call through it is
// reported the same way as through `constructor(val p: T)`.
class Point {
  constructor(public val x: number, private val y: number, readonly val tags: string[]) {}
  sum() { return this.x + this.y; }
}
class Bag {
  constructor(protected val items: number[]) {
    items.push(1);
  }
}
class Slots {
  constructor(public readonly val slots: number[], val spare: number[]) {
    slots[0] = 1;
    spare.push(2);
  }
}
class Held {
  constructor(private val held: number[]) {}
  add(n: number) {
    this.held.push(n);
    return this.held.length;
  }
}

