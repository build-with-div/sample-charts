// Generates the fixtures of the d3-shape port and of Recharts' Curve path by
// running the pinned packages as a page bundles them: d3-shape 3.2.1 and
// d3-path 3.1.1 through victory-vendor, and Recharts' browser build
// (`recharts/es6/shape/Curve.js`). See esm-loader.mjs.
//
//   npm ci --ignore-scripts
//   node shape_fixtures.mjs          # write the fixtures
//   node shape_fixtures.mjs --check  # fail if the committed fixtures differ
//
// Writes crates/d3/tests/fixtures/shape.json, crates/d3/tests/fixtures/stack.json,
// crates/recharts/tests/fixtures/curve.json and
// crates/recharts/tests/fixtures/shapes.json (<Rectangle>, <Symbols> and
// <Sector> rendered with react-dom/server). Values are tagged as in
// axis_fixtures.mjs: "u" undefined, null, booleans, "n:<String(n)>" numbers,
// "s:<text>" strings, [..] arrays, {"o": {..}} objects.
//
// The line and area generators and the curves Recharts uses only add,
// multiply and divide, and paths are rounded with Math.round, so the
// results do not depend on the platform's math library. The star symbol
// and <Sector> call Math.sin and Math.cos, which Node 24 computes
// differently from Chromium 147: chrome_trig.mjs substitutes Chrome's
// recorded results, before any package loads.

import { register } from "node:module";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { useChromeTrig } from "./chrome_trig.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
register(pathToFileURL(join(here, "esm-loader.mjs")));
const recordMissingTrig = useChromeTrig(root);

function enc(v) {
  if (v === undefined) return "u";
  if (v === null) return null;
  if (typeof v === "boolean") return v;
  if (typeof v === "number") return "n:" + (Object.is(v, -0) ? "-0" : String(v));
  if (typeof v === "string") return "s:" + v;
  if (Array.isArray(v)) return Array.from(v, enc);
  if (typeof v === "object") {
    return { o: Object.fromEntries(Object.entries(v).map(([k, x]) => [k, enc(x)])) };
  }
  throw new Error("cannot encode a " + typeof v);
}

let seed = 0x5ea9e;
function rand() {
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  return seed / 2 ** 32;
}
const pick = (list) => list[Math.floor(rand() * list.length)];
const chance = (p) => rand() < p;
const int = (lo, hi) => lo + Math.floor(rand() * (hi - lo + 1));
const pow10 = (n) => Number(`1e${n}`);

// A coordinate a chart could produce: mostly pixels with a fraction, at
// times large, tiny, negative, -0 or repeated.
function coordinate(previous) {
  const r = rand();
  if (previous !== undefined && r < 0.08) return previous;
  if (r < 0.1) return -0;
  if (r < 0.14) return pick([1e7, 123456789.123, 1e-7, 0.0005, 0.0015, 2.5e-4]) * (chance(0.5) ? -1 : 1);
  const v = int(-50, 800) + (chance(0.7) ? Math.round(rand() * pow10(int(1, 15))) / pow10(int(1, 15)) : 0);
  return chance(0.1) ? v + 0.1 + 0.2 : v;
}

function series(n) {
  const xs = [];
  let x = int(-20, 100);
  for (let i = 0; i < n; i++) {
    if (chance(0.85)) x += pick([0, int(1, 80), rand() * 50]);
    else x = coordinate(x);
    xs.push(x);
  }
  if (chance(0.3)) xs.reverse();
  return xs;
}

const CURVES = [
  "curveBasis", "curveBasisClosed", "curveBasisOpen", "curveBumpX", "curveBumpY", "curveLinear",
  "curveLinearClosed", "curveMonotoneX", "curveMonotoneY", "curveNatural", "curveStep",
  "curveStepAfter", "curveStepBefore",
];

function d3Fixture(d3) {
  const cases = [];
  for (let i = 0; i < 2000; i++) {
    const curve = CURVES[i % CURVES.length];
    const n = pick([0, 1, 1, 2, 2, 3, 3, 4, 5, 6, 8, 12]);
    const xs = series(n);
    let previous;
    const points = xs.map((x) => {
      if (chance(0.12)) return null; // not defined
      const y = coordinate(previous);
      previous = y;
      return [x, y, coordinate(), coordinate()];
    });
    const kind = chance(0.5) ? "line" : "area";
    const digits = pick([3, 3, 3, null, 0, 1, 6, 15, 16]);
    const hasX1 = kind === "area" && chance(0.3);
    const hasY1 = kind === "area" && !hasX1 ? chance(0.8) : kind === "area" && chance(0.2);
    let path;
    const defined = (d) => d !== null;
    if (kind === "line") {
      path = d3.line((d) => d[0], (d) => d[1]).defined(defined).curve(d3[curve]).digits(digits)(points);
    } else {
      const a = d3.area().defined(defined).curve(d3[curve]).digits(digits);
      a.x0((d) => d[0]).y0((d) => d[2]);
      if (hasX1) a.x1((d) => d[3]);
      // d3.area()'s y1 defaults to d => d[1]; null removes it.
      a.y1(hasY1 ? (d) => d[1] : null);
      path = a(points);
    }
    cases.push({
      kind,
      curve,
      digits: enc(digits),
      hasX1,
      hasY1,
      points: enc(points),
      path: enc(path),
    });
  }
  return cases;
}

const TYPES = [
  undefined, "linear", "monotone", "monotoneX", "monotoneY", "bump", "bumpX", "bumpY", "natural", "step",
  "stepBefore", "stepAfter", "basis", "basisClosed", "basisOpen", "linearClosed", "unknown", "Linear", "",
];

function curveValue(previous) {
  const r = rand();
  if (r < 0.04) return null;
  if (r < 0.06) return undefined;
  if (r < 0.08) return NaN;
  if (r < 0.09) return pick([Infinity, -Infinity]);
  if (r < 0.1) return "12";
  return coordinate(previous);
}

function rechartsFixture(getPath) {
  const cases = [];
  for (let i = 0; i < 2000; i++) {
    const n = pick([0, 1, 2, 3, 4, 5, 6, 7, 12]);
    const xs = series(n);
    let previous;
    const points = xs.map((x) => {
      const y = curveValue(previous);
      previous = y;
      return { x: chance(0.05) ? curveValue() : x, y };
    });
    const r = rand();
    let baseLine;
    if (r < 0.35) baseLine = undefined;
    else if (r < 0.6) baseLine = pick([0, 300, 452.5, -12.25, NaN, Infinity, "0", null]);
    else
      baseLine = points
        .map((p) => (chance(0.08) ? null : { x: chance(0.5) ? p.x : curveValue(), y: chance(0.5) ? 300 : curveValue() }))
        .slice(0, chance(0.1) ? Math.max(0, n - 1) : n);
    const input = {
      type: pick(TYPES),
      points,
      baseLine,
      layout: pick([undefined, "horizontal", "horizontal", "vertical", "vertical", "centric", ""]),
      connectNulls: chance(0.3),
    };
    let path;
    try {
      path = enc(getPath(input));
    } catch (error) {
      path = { error: String(error.message) };
    }
    cases.push({ input: enc(input), path });
  }
  return cases;
}

const OFFSETS = ["stackOffsetNone", "stackOffsetExpand", "stackOffsetSilhouette", "stackOffsetWiggle"];

// d3.stack() with stackOrderNone and each offset Recharts maps, over values
// a chart could give it after Number(): mostly numbers, some 0, -0, NaN or
// infinite. Drawn from its own seed so the fixtures above stay as they were.
function stackFixture(d3) {
  seed = 0x57ac4;
  const value = () => {
    const r = rand();
    if (r < 0.06) return NaN;
    if (r < 0.1) return 0;
    if (r < 0.12) return -0;
    if (r < 0.14) return pick([Infinity, -Infinity]);
    if (r < 0.2) return pick([1e7, 123456789.123, 1e-7, 0.1, 0.2, 0.3]) * (chance(0.5) ? -1 : 1);
    if (r < 0.45) return int(-300, 300);
    return int(0, 500) + (chance(0.5) ? Math.round(rand() * 1000) / 1000 : 0);
  };
  const cases = [];
  for (let i = 0; i < 1600; i++) {
    const offset = OFFSETS[i % OFFSETS.length];
    const n = pick([0, 1, 2, 2, 3, 3, 4, 5, 6]);
    const m = pick([0, 1, 2, 3, 4, 5, 6, 8, 12]);
    const keys = Array.from({ length: n }, (_, k) => `k${k}`);
    const values = keys.map(() => Array.from({ length: m }, value));
    const data = Array.from({ length: m }, (_, j) => Object.fromEntries(keys.map((k, s) => [k, values[s][j]])));
    const series = d3.stack().keys(keys).order(d3.stackOrderNone).offset(d3[offset])(data);
    cases.push({
      offset,
      values: enc(values),
      series: series.map((s) => s.map((point) => [enc(point[0]), enc(point[1])])),
    });
  }
  return cases;
}

// What a component renders, as server rendering writes it: null for
// nothing, else the attributes of its single <path>.
function renderPath(React, renderToStaticMarkup, component, props) {
  const html = renderToStaticMarkup(React.createElement(component, props));
  if (html === "") return null;
  const match = /^<path((?: [a-zA-Z-]+="[^"]*")*)(?:\/>|><\/path>)$/.exec(html);
  if (!match) throw new Error(`unexpected markup ${html}`);
  const unescape = (s) =>
    s.replace(/&quot;/g, '"').replace(/&#x27;/g, "'").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
  return Object.fromEntries([...match[1].matchAll(/ ([a-zA-Z-]+)="([^"]*)"/g)].map(([, k, v]) => [k, unescape(v)]));
}

// <Rectangle> and <Symbols> rendered by React's server renderer, as the
// browser build defines them. Drawn from their own seed.
function shapesFixture(React, renderToStaticMarkup, Rectangle, Symbols) {
  seed = 0x5a9e5;
  const position = () =>
    chance(0.06) ? pick([-0, NaN, 1e7, 0.00005, 0.12345678, Infinity]) : int(-50, 800) + (chance(0.5) ? Math.round(rand() * 1e4) / 1e4 : 0);
  const side = () =>
    chance(0.12)
      ? pick([0, -0, NaN, 0.00004, 0.00006, 1e-9, Infinity, 123456.789])
      : (chance(0.2) ? -1 : 1) * (int(0, 400) + (chance(0.5) ? Math.round(rand() * 1e6) / 1e6 : 0));
  const corner = () => pick([0, 4, 8.25, 100, null, undefined, -2, NaN, 3, 6]);
  const rectangles = [];
  for (let i = 0; i < 1500; i++) {
    const props = { x: position(), y: position(), width: side(), height: side() };
    const r = rand();
    if (r < 0.2) props.radius = undefined;
    else if (r < 0.5) props.radius = pick([0, 4, 8, 0.5, 100, -3, NaN, Infinity, 2.123456]);
    else if (r < 0.95) props.radius = Array.from({ length: chance(0.85) ? 4 : int(0, 5) }, corner);
    else props.radius = "5";
    if (props.radius === undefined) delete props.radius;
    const path = renderPath(React, renderToStaticMarkup, Rectangle, props);
    rectangles.push({ props: enc(props), d: path === null ? null : enc(path.d) });
  }
  const symbols = [];
  const types = ["circle", "cross", "diamond", "square", "star", "triangle", "wye", "Cross", "", "unknown", "SQUARE", 5, undefined];
  for (let i = 0; i < 800; i++) {
    const props = {};
    const type = pick(types);
    if (type !== undefined) props.type = type;
    const size = pick([undefined, undefined, 0, 1, 64, 100.5, 1e6, -4, NaN, 30, 12.25]);
    if (size !== undefined) props.size = size;
    const sizeType = pick([undefined, undefined, "area", "diameter", "foo"]);
    if (sizeType !== undefined) props.sizeType = sizeType;
    for (const k of ["cx", "cy"]) {
      const v = chance(0.05) ? pick([undefined, NaN]) : position();
      if (v !== undefined) props[k] = v;
    }
    const path = renderPath(React, renderToStaticMarkup, Symbols, props);
    symbols.push({
      props: enc(props),
      path: path === null ? null : { transform: enc(path.transform), d: enc(path.d) },
    });
  }
  return { rectangles, symbols };
}

// <Sector> rendered by React's server renderer: pie and radial-bar sectors,
// with and without rounded corners. Drawn after the rectangles and symbols.
function sectorsFixture(React, renderToStaticMarkup, Sector) {
  const fraction = () => (chance(0.6) ? Math.round(rand() * 1e4) / 1e4 : 0);
  const center = () => (chance(0.04) ? pick([-0, NaN, 0.5, 1e6]) : int(0, 600) + fraction());
  const radius = () => (chance(0.08) ? pick([0, -0, -5, NaN, 0.25, 1000.125]) : int(0, 300) + fraction());
  const angle = () => {
    const r = rand();
    if (r < 0.3) return pick([0, 90, 180, 270, 360, -90, -180, -270, -360, 450, 720, 0.5, -0]);
    if (r < 0.85) return int(-360, 360) + fraction() * (chance(0.5) ? 1 : 0) + (chance(0.3) ? rand() : 0);
    return int(-1000, 1000) + rand();
  };
  const sectors = [];
  for (let i = 0; i < 1500; i++) {
    const props = { cx: center(), cy: center() };
    const inner = radius();
    const outer = radius();
    if (chance(0.3)) props.innerRadius = 0;
    else props.innerRadius = chance(0.85) ? Math.min(inner, outer) : inner;
    props.outerRadius = chance(0.85) ? Math.max(inner, outer) : outer;
    props.startAngle = angle();
    const r = rand();
    if (r < 0.05) props.endAngle = props.startAngle;
    else if (r < 0.15) props.endAngle = props.startAngle + pick([360, -360, 359.999, 359.9995, 180, -180, 400]);
    else if (r < 0.6) props.endAngle = props.startAngle + (chance(0.5) ? 1 : -1) * (rand() * pick([10, 90, 200, 360]));
    else props.endAngle = angle();
    const c = rand();
    if (c < 0.35) {
      // no cornerRadius: 0 by default
    } else if (c < 0.8) props.cornerRadius = pick([0, 1, 2, 4, 5.5, 8, 10, 20, 50, 1000, -3, NaN]);
    else props.cornerRadius = pick(["10%", "50%", "25.5%", "200%", "5", "abc", "", "-10%"]);
    if (chance(0.3)) props.forceCornerRadius = chance(0.6);
    if (chance(0.3)) props.cornerIsExternal = chance(0.6);
    for (const k of ["innerRadius", "outerRadius", "startAngle", "endAngle"]) {
      if (chance(0.02)) delete props[k];
    }
    const path = renderPath(React, renderToStaticMarkup, Sector, props);
    sectors.push({ props: enc(props), path: path === null ? null : enc(path) });
  }
  return { sectors };
}

function serialize(content) {
  const lines = ["{"];
  const entries = Object.entries(content);
  entries.forEach(([key, v], i) => {
    const comma = i + 1 < entries.length ? "," : "";
    if (Array.isArray(v) && v.length && typeof v[0] === "object") {
      lines.push(`  ${JSON.stringify(key)}: [`);
      v.forEach((item, j) => lines.push(`    ${JSON.stringify(item)}${j + 1 < v.length ? "," : ""}`));
      lines.push(`  ]${comma}`);
    } else {
      lines.push(`  ${JSON.stringify(key)}: ${JSON.stringify(v)}${comma}`);
    }
  });
  lines.push("}");
  return lines.join("\n") + "\n";
}

const d3 = await import("victory-vendor/d3-shape");
const { getPath } = await import(pathToFileURL(join(here, "node_modules", "recharts", "es6", "shape", "Curve.js")).href);
const React = await import("react");
const { renderToStaticMarkup } = await import("react-dom/server");
const es6 = (path) => import(pathToFileURL(join(here, "node_modules", "recharts", "es6", path)).href);
const { Rectangle } = await es6("shape/Rectangle.js");
const { Symbols } = await es6("shape/Symbols.js");
const { Sector } = await es6("shape/Sector.js");
const pin = JSON.parse(readFileSync(join(here, "package-lock.json"), "utf8")).packages;
const versions = Object.fromEntries(
  ["recharts", "d3-shape", "d3-path", "victory-vendor", "react", "react-dom"].map((name) => [name, pin[`node_modules/${name}`].version]),
);
const generator = `tools/reference/shape_fixtures.mjs (Node ${process.versions.node}, V8 ${process.versions.v8})`;
const files = {
  "crates/d3/tests/fixtures/shape.json": { generator, versions, cases: d3Fixture(d3) },
  "crates/recharts/tests/fixtures/curve.json": { generator, versions, cases: rechartsFixture(getPath) },
  "crates/d3/tests/fixtures/stack.json": { generator, versions, cases: stackFixture(d3) },
  "crates/recharts/tests/fixtures/shapes.json": {
    generator,
    versions,
    ...shapesFixture(React, renderToStaticMarkup, Rectangle, Symbols),
    ...sectorsFixture(React, renderToStaticMarkup, Sector),
  },
};
if (recordMissingTrig()) process.exit(1);
const check = process.argv.includes("--check");
const stale = [];
for (const [path, content] of Object.entries(files)) {
  const text = serialize(content);
  const file = join(root, path);
  if (check) {
    let old = null;
    try {
      old = readFileSync(file, "utf8");
    } catch {
      // missing
    }
    if (old !== text) stale.push(path);
  } else {
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, text);
    console.log(`wrote ${path} (${(text.length / 1024).toFixed(0)} KiB, ${(content.cases ?? [...content.rectangles, ...content.symbols, ...content.sectors]).length} cases)`);
  }
}
if (stale.length) {
  console.error(`fixtures differ from the packages' output: ${stale.join(", ")}; run node shape_fixtures.mjs`);
  process.exit(1);
}
