import { variant } from "./variants.mjs";

const shape = variant("Shape", [
  ["Circle", [["r", "number"]]],
  ["Rect", [["w", "number"], ["h", "number"]]],
  ["Point", null],
]);

export default {
  construct: "matchComments",
  family: "match",
  kind: "value",
  jsx: true,
  forms: [
    {
      id: "betweenArms",
      title: "line and block comments written before, between, and after the arms",
      decls: [shape],
      In: "Shape",
      inputs: "[Shape.Circle(2), Shape.Rect(3, 4), Shape.Point]",
      tt: (x) =>
        `match (${x}) {\n// the radius, doubled\nCircle(r) => note("circle", r * 2), // an area next\nRect(w, h) => note("rect", w * h), /* no area */ Point => note("point", 0)\n// after the last arm\n}`,
      ts: (x, [t]) =>
        `(${t} = ${x}, ${t}.kind === "Circle" ? note("circle", ${t}.r * 2) : ${t}.kind === "Rect" ? note("rect", ${t}.w * ${t}.h) : note("point", 0))`,
    },
  ],
};
