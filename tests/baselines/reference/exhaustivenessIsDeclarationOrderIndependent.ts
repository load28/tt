//// [exhaustivenessIsDeclarationOrderIndependent.tt] ////
const f = (s: Shape) => match (s) {
  Circle(radius) => radius,
};
variant Shape { Circle(radius: number), Point }

