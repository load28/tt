//// [tryInClassCodeInsideResult.tt] ////
// A class body and a class static block are Result scope boundaries, as a
// function is: code there is not evaluated by the `result` block written
// around the class, and it can neither `return` nor `break` to a label
// outside (ECMA-262 §15.7.1). A `try` there does not leave the `result`
// block; its nearest scope is the class code, which has no failure edge,
// so it is `try-placement`. A `result` block written inside the static
// block is the nearest scope of its own `try` and stays legal.
import type { TResult } from "@tt/std";
declare function read(n: number): TResult<number, string>;
export const inStaticBlock = result {
  const z = try read(1);
  class K {
    static {
      const w = try read(2);
    }
  }
  return z;
};
export const inFieldInitializer = result {
  const z = try read(3);
  class F {
    x = try read(4);
  }
  return z;
};
export const nestedResult = result {
  const z = try read(5);
  class S {
    static v: unknown;
    static {
      S.v = result {
        const q = try read(6);
        return q;
      };
    }
  }
  return z;
};

