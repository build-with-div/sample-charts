// Records the DOM Recharts renders in Chromium: bundles the pinned react,
// react-dom and recharts with esbuild the way a page would, renders each
// chart case in headless Chrome and writes what is in the chart's container
// once it settles to crates/recharts/tests/fixtures/dom.json, which the
// Rust port's element tree is compared with (crates/recharts/tests/dom.rs).
//
//   npm ci --ignore-scripts
//   node dom_fixtures.mjs <path to chrome or chrome.exe>   # write the fixture
//   node dom_fixtures.mjs --check-inputs                    # no browser: fail if
//                                                           # the committed cases
//                                                           # are not these
//
// Chrome is not run in CI, so the fixture is written by hand with the
// Chrome for Testing build named in docs/design.md; it records the
// browser's user agent and time zone.
//
// Each case renders in a fresh iframe, so a fresh JavaScript realm: React's
// `useId` counter and Recharts' `uniqueId` counter and text measurement
// cache start over for every chart, as on a page that renders one chart.
// The cases are encoded as in axis_fixtures.mjs ("u", null, booleans,
// "n:…", "s:…", "t:…", arrays, {"o": …}); a case is the chart's props and
// its children, each a component name and its props.
//
// Every call of Recharts' `getStringSize` (util/DOMUtils.js, which
// measures text in a hidden <span>) is recorded with its result, as
// [text, style, width, height] per distinct call: the tick label filter
// and the axes measure text, and the port reads these measurements until it
// renders through Div's own text layout. So are the font size and letter
// spacing `getComputedStyle` gives for a tick label (`tickFonts`), which an
// axis measures its ticks with once its labels are in the document.
//
// Every <Bar>, <Line> and <Area> renders with isAnimationActive={false}: the
// fixture is the chart at rest. For bars that is also the last frame of the
// entrance animation; a line's last frame sets strokeDasharray from
// getTotalLength(), and an area's is drawn through a growing clip path.

import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { areaCases, barCases, lineCases } from "./axis_fixtures.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const fixturePath = join(root, "crates", "recharts", "tests", "fixtures", "dom.json");

// ---------------------------------------------------------------- encoding

function enc(v) {
  if (v === undefined) return "u";
  if (v === null) return null;
  if (typeof v === "boolean") return v;
  if (typeof v === "number") return "n:" + (Object.is(v, -0) ? "-0" : String(v));
  if (typeof v === "string") return "s:" + v;
  if (v instanceof Date) return "t:" + v.getTime();
  if (Array.isArray(v)) return Array.from(v, enc);
  if (typeof v === "object") {
    return { o: Object.fromEntries(Object.entries(v).map(([k, x]) => [k, enc(x)])) };
  }
  throw new Error("cannot encode a " + typeof v);
}

// The browser side decodes with the inverse.
function dec(v) {
  if (v === "u") return undefined;
  if (v === null || typeof v === "boolean") return v;
  if (Array.isArray(v)) return v.map(dec);
  if (typeof v === "object") return Object.fromEntries(Object.entries(v.o).map(([k, x]) => [k, dec(x)]));
  if (v.startsWith("n:")) return v === "n:-0" ? -0 : Number(v.slice(2));
  if (v.startsWith("s:")) return v.slice(2);
  if (v.startsWith("t:")) return new Date(Number(v.slice(2)));
  throw new Error("cannot decode " + v);
}

// ------------------------------------------------------------------ cases

let seed = 0xd0e1;
function rand() {
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  return seed / 2 ** 32;
}
const pick = (list) => list[Math.floor(rand() * list.length)];
const chance = (p) => rand() < p;

const element = (type, props) => ({ type, props });

// shadcn's bar chart examples (apps/v4/registry/new-york-v4/charts at
// shadcn@4.23.0) that use only the components ported so far, at a fixed
// size instead of ChartContainer's responsive one, without the tooltip and
// without tickFormatter functions (they only change axis labels).
function shadcnCases() {
  const data = [
    { month: "January", desktop: 186, mobile: 80 },
    { month: "February", desktop: 305, mobile: 200 },
    { month: "March", desktop: 237, mobile: 120 },
    { month: "April", desktop: 73, mobile: 190 },
    { month: "May", desktop: 209, mobile: 130 },
    { month: "June", desktop: 214, mobile: 140 },
  ];
  const monthAxis = { dataKey: "month", tickLine: false, tickMargin: 10, axisLine: false };
  const desktop = (props) => element("Bar", { dataKey: "desktop", fill: "var(--color-desktop)", isAnimationActive: false, ...props });
  const mobile = (props) => element("Bar", { dataKey: "mobile", fill: "var(--color-mobile)", isAnimationActive: false, ...props });
  const chart = (props) => ({ chartName: "BarChart", width: 500, height: 300, accessibilityLayer: true, data, ...props });
  return [
    {
      index: "shadcn-bar-default",
      chart: chart({}),
      children: [element("CartesianGrid", { vertical: false }), element("XAxis", monthAxis), desktop({ radius: 8 })],
    },
    {
      index: "shadcn-bar-multiple",
      chart: chart({}),
      children: [element("CartesianGrid", { vertical: false }), element("XAxis", monthAxis), desktop({ radius: 4 }), mobile({ radius: 4 })],
    },
    {
      index: "shadcn-bar-horizontal",
      chart: chart({ layout: "vertical", margin: { left: -20 } }),
      children: [
        element("XAxis", { type: "number", dataKey: "desktop", hide: true }),
        element("YAxis", { dataKey: "month", type: "category", tickLine: false, tickMargin: 10, axisLine: false }),
        desktop({ radius: 5 }),
      ],
    },
    {
      index: "shadcn-bar-stacked",
      chart: chart({}),
      children: [
        element("CartesianGrid", { vertical: false }),
        element("XAxis", monthAxis),
        desktop({ stackId: "a", radius: [0, 0, 4, 4] }),
        mobile({ stackId: "a", radius: [4, 4, 0, 0] }),
      ],
    },
    {
      index: "shadcn-bar-mixed",
      chart: chart({
        layout: "vertical",
        margin: { left: 0 },
        data: [
          { browser: "chrome", visitors: 275, fill: "var(--color-chrome)" },
          { browser: "safari", visitors: 200, fill: "var(--color-safari)" },
          { browser: "firefox", visitors: 187, fill: "var(--color-firefox)" },
          { browser: "edge", visitors: 173, fill: "var(--color-edge)" },
          { browser: "other", visitors: 90, fill: "var(--color-other)" },
        ],
      }),
      children: [
        element("YAxis", { dataKey: "browser", type: "category", tickLine: false, tickMargin: 10, axisLine: false }),
        element("XAxis", { dataKey: "visitors", type: "number", hide: true }),
        element("Bar", { dataKey: "visitors", layout: "vertical", radius: 5, isAnimationActive: false }),
      ],
    },
  ];
}

// The 400 bar charts of axis_fixtures.mjs (their geometry is in axes.json),
// with only their bars and with the props that change only what is drawn:
// colours, corner radii, class names, backgrounds, z-indexes, the chart's
// title, description, class, style and accessibility attributes.
function generatedCases() {
  return barCases().map((c) => {
    const chart = { ...c.chart };
    if (chance(0.1)) chart.accessibilityLayer = false;
    if (chance(0.15)) chart.title = pick(["Sales", "", "Q1 <revenue> & costs"]);
    if (chance(0.15)) chart.desc = pick(["Monthly totals", 42]);
    if (chance(0.1)) chart.className = pick(["my-chart", "a  b"]);
    if (chance(0.08)) chart.style = pick([{ opacity: 0.5 }, { marginTop: 3 }, { backgroundColor: "red", width: 300 }]);
    if (chance(0.05)) chart.id = "chart-1";
    if (chance(0.05)) chart.tabIndex = pick([-1, 2]);
    if (chance(0.05)) chart.role = "img";
    const children = [];
    if (chance(0.6)) {
      children.push(element("CartesianGrid", pick([{}, { vertical: false }, { horizontal: false }, { strokeDasharray: "3 3" }, { fill: "#fafafa" }])));
    }
    for (const props of c.xAxes) children.push(element("XAxis", props));
    for (const props of c.yAxes) children.push(element("YAxis", props));
    for (const item of c.items) {
      if (item.kind !== "bar") continue;
      const { kind, ...props } = item;
      if (chance(0.5)) delete props.id;
      if (chance(0.7)) props.fill = pick(["#2563eb", "var(--color-desktop)", "red", "none"]);
      if (chance(0.2)) props.stroke = pick(["#000", "white"]);
      if (chance(0.15)) props.strokeWidth = pick([1, 2.5, "3"]);
      if (chance(0.4)) props.radius = pick([4, 8, 0, [4, 4, 0, 0], [0, 0, 4, 4], [2], -3, 100, "4"]);
      if (chance(0.2)) props.name = pick(["Desktop", 7, ""]);
      if (chance(0.15)) props.className = pick(["bar-a", "x  y"]);
      if (chance(0.1)) props.opacity = 0.8;
      if (chance(0.1)) props.fillOpacity = 0.6;
      if (chance(0.05)) props.strokeDasharray = "3 3";
      if (chance(0.05)) props["data-testid"] = "bar";
      if (chance(0.05)) props["aria-label"] = "bars";
      if (chance(0.2)) {
        props.background = pick([true, { fill: "#ddd" }, { fill: "#eee", radius: 4 }, { zIndex: -10 }, { fill: "#f00", zIndex: 350 }]);
      }
      if (chance(0.12)) props.zIndex = pick([0, 50, -5, 300, 1000]);
      props.isAnimationActive = false;
      children.push(element("Bar", props));
    }
    return { index: c.index, chart, children };
  });
}

// shadcn's line chart examples (same source and changes as the bar ones)
// whose dots are not functions: chart-line-default, -linear, -step,
// -multiple and -dots.
function shadcnLineCases() {
  const data = [
    { month: "January", desktop: 186, mobile: 80 },
    { month: "February", desktop: 305, mobile: 200 },
    { month: "March", desktop: 237, mobile: 120 },
    { month: "April", desktop: 73, mobile: 190 },
    { month: "May", desktop: 209, mobile: 130 },
    { month: "June", desktop: 214, mobile: 140 },
  ];
  const monthAxis = { dataKey: "month", tickLine: false, axisLine: false, tickMargin: 8 };
  const chart = { chartName: "LineChart", width: 500, height: 300, accessibilityLayer: true, data, margin: { left: 12, right: 12 } };
  const line = (key, props) => element("Line", { dataKey: key, stroke: `var(--color-${key})`, strokeWidth: 2, dot: false, isAnimationActive: false, ...props });
  const cases = [
    ["default", [line("desktop", { type: "natural" })]],
    ["linear", [line("desktop", { type: "linear" })]],
    ["step", [line("desktop", { type: "step" })]],
    ["multiple", [line("desktop", { type: "monotone" }), line("mobile", { type: "monotone" })]],
    ["dots", [line("desktop", { type: "natural", dot: { fill: "var(--color-desktop)" }, activeDot: { r: 6 } })]],
  ];
  return cases.map(([name, lines]) => ({
    index: `shadcn-line-${name}`,
    chart,
    children: [element("CartesianGrid", { vertical: false }), element("XAxis", monthAxis), ...lines],
  }));
}

// The 300 line charts of axis_fixtures.mjs (lineCases), with the props that
// change only what is drawn: curve types, dots, connectNulls, colours,
// dashes, class names and z-indexes. Their own seed keeps the cases above as
// they were.
function generatedLineCases() {
  const saved = seed;
  seed = 0x11ed;
  const cases = lineCases().map((c) => {
    const children = [];
    if (chance(0.6)) children.push(element("CartesianGrid", pick([{}, { vertical: false }, { strokeDasharray: "3 3" }])));
    for (const props of c.xAxes) children.push(element("XAxis", props));
    for (const props of c.yAxes) children.push(element("YAxis", props));
    for (const item of c.items) {
      const { kind, ...props } = item;
      if (chance(0.5)) delete props.id;
      if (kind === "line") {
        if (chance(0.7)) {
          props.type = pick(["linear", "monotone", "natural", "step", "stepBefore", "stepAfter", "basis", "basisClosed", "basisOpen", "bump", "bumpX", "bumpY", "linearClosed", "monotoneX", "monotoneY", "cardinal"]);
        }
        if (chance(0.5)) {
          props.dot = pick([false, true, { r: 5 }, { fill: "red", strokeWidth: 3 }, { clipDot: false }, { clipDot: false, r: 6, className: "d" }, { r: "4" }]);
        }
        if (chance(0.25)) props.connectNulls = true;
        if (chance(0.5)) props.stroke = pick(["#2563eb", "var(--color-desktop)", "red", "none"]);
        if (chance(0.3)) props.strokeWidth = pick([1, 2, 2.5, "3"]);
        if (chance(0.1)) props.strokeDasharray = pick(["5 5", "3 3 1", [4, 2]]);
        if (chance(0.1)) props.fill = pick(["none", "#eee"]);
        if (chance(0.15)) props.name = pick(["Desktop", 7, ""]);
        if (chance(0.1)) props.className = pick(["line-a", "x  y"]);
        if (chance(0.05)) props.opacity = 0.8;
        if (chance(0.1)) props.zIndex = pick([0, 50, 450, 1000]);
      }
      props.isAnimationActive = false;
      children.push(element(kind === "line" ? "Line" : "Bar", props));
    }
    return { index: c.index, chart: c.chart, children };
  });
  seed = saved;
  return cases;
}

// shadcn's area chart examples (same source and changes as the bar ones)
// that need no legend or SVG children of their own: chart-area-default,
// -linear, -step, -stacked, -stacked-expand and -axes.
function shadcnAreaCases() {
  const data = [
    { month: "January", desktop: 186, mobile: 80 },
    { month: "February", desktop: 305, mobile: 200 },
    { month: "March", desktop: 237, mobile: 120 },
    { month: "April", desktop: 73, mobile: 190 },
    { month: "May", desktop: 209, mobile: 130 },
    { month: "June", desktop: 214, mobile: 140 },
  ];
  const expandData = data.map((d, i) => ({ ...d, other: [45, 100, 150, 50, 100, 160][i] }));
  const monthAxis = { dataKey: "month", tickLine: false, axisLine: false, tickMargin: 8 };
  const chart = { chartName: "AreaChart", width: 500, height: 300, accessibilityLayer: true, data, margin: { left: 12, right: 12 } };
  const area = (key, props) =>
    element("Area", { dataKey: key, type: "natural", fill: `var(--color-${key})`, fillOpacity: 0.4, stroke: `var(--color-${key})`, isAnimationActive: false, ...props });
  const stacked = [area("mobile", { stackId: "a" }), area("desktop", { stackId: "a" })];
  const single = { ...chart, data: data.map(({ month, desktop }) => ({ month, desktop })) };
  const cases = [
    ["default", single, [area("desktop")]],
    ["linear", single, [area("desktop", { type: "linear" })]],
    ["step", single, [area("desktop", { type: "step" })]],
    ["stacked", chart, stacked],
    [
      "stacked-expand",
      { ...chart, data: expandData, margin: { left: 12, right: 12, top: 12 }, stackOffset: "expand" },
      [area("other", { stackId: "a", fillOpacity: 0.1 }), ...stacked],
    ],
    [
      "axes",
      { ...chart, margin: { left: -20, right: 12 } },
      [element("YAxis", { tickLine: false, axisLine: false, tickMargin: 8, tickCount: 3 }), ...stacked],
    ],
  ];
  return cases.map(([name, chart, items]) => ({
    index: `shadcn-area-${name}`,
    chart,
    children: [element("CartesianGrid", { vertical: false }), element("XAxis", monthAxis), ...items],
  }));
}

// The 300 area charts of axis_fixtures.mjs (areaCases), with the props that
// change only what is drawn. Their own seed keeps the cases above as they
// were.
function generatedAreaCases() {
  const saved = seed;
  seed = 0xa4eb;
  const cases = areaCases().map((c) => {
    const children = [];
    if (chance(0.6)) children.push(element("CartesianGrid", pick([{}, { vertical: false }, { strokeDasharray: "3 3" }])));
    for (const props of c.xAxes) children.push(element("XAxis", props));
    for (const props of c.yAxes) children.push(element("YAxis", props));
    for (const item of c.items) {
      const { kind, ...props } = item;
      if (chance(0.5)) delete props.id;
      if (kind === "area") {
        if (chance(0.7)) {
          props.type = pick(["linear", "monotone", "natural", "step", "stepBefore", "stepAfter", "basis", "basisClosed", "basisOpen", "bump", "bumpX", "bumpY", "linearClosed", "monotoneX", "monotoneY"]);
        }
        if (chance(0.25)) props.dot = pick([true, { r: 5 }, { fill: "red", strokeWidth: 3 }, { clipDot: false }, { clipDot: false, r: 6, className: "d" }]);
        if (chance(0.5)) props.stroke = pick(["#2563eb", "var(--color-desktop)", "red", "none"]);
        if (chance(0.4)) props.fill = pick(["var(--color-desktop)", "#eee", "none", "url(#fill)"]);
        if (chance(0.3)) props.fillOpacity = pick([0.4, 0.1, 1, "0.5"]);
        if (chance(0.2)) props.strokeWidth = pick([1, 2, "3"]);
        if (chance(0.1)) props.strokeDasharray = pick(["5 5", [4, 2]]);
        if (chance(0.15)) props.name = pick(["Desktop", 7, ""]);
        if (chance(0.1)) props.className = pick(["area-a", "x  y"]);
        if (chance(0.05)) props.opacity = 0.8;
        if (chance(0.1)) props.zIndex = pick([0, 50, 100, 450, 1000]);
      }
      props.isAnimationActive = false;
      children.push(element({ area: "Area", line: "Line", bar: "Bar" }[kind], props));
    }
    return { index: c.index, chart: c.chart, children };
  });
  seed = saved;
  return cases;
}

function allCases() {
  return [
    ...shadcnCases(),
    ...generatedCases(),
    ...shadcnLineCases(),
    ...generatedLineCases(),
    ...shadcnAreaCases(),
    ...generatedAreaCases(),
  ].map((c) => ({
    index: c.index,
    chart: enc(c.chart),
    children: c.children.map((child) => ({ type: child.type, props: enc(child.props) })),
  }));
}

// ------------------------------------------------------------ the browser

// The page each case renders in. `getStringSize` is wrapped to record its
// calls (see measurePlugin).
const frameEntry = `
import React from "react";
import { createRoot } from "react-dom/client";
import * as Recharts from "recharts";
const enc = ${enc};
const dec = ${dec};
const measurements = [];
const seen = new Set();
window.__recordMeasure = (args, result) => {
  const call = [enc(args[0]), args.length > 1 ? enc(args[1]) : "u"];
  const key = JSON.stringify(call);
  if (seen.has(key)) return;
  seen.add(key);
  measurements.push([...call, enc(result.width), enc(result.height)]);
};
// CartesianAxis reads the font size and letter spacing of its first tick
// label with getComputedStyle and measures the ticks again with them; each
// distinct result is recorded.
const fonts = [];
const getComputedStyle = window.getComputedStyle;
window.getComputedStyle = function (element, pseudo) {
  const style = getComputedStyle.call(window, element, pseudo);
  if (element.classList.contains("recharts-cartesian-axis-tick-value")) {
    const font = [style.fontSize, style.letterSpacing];
    if (!fonts.some((f) => f[0] === font[0] && f[1] === font[1])) fonts.push(font);
  }
  return style;
};
function serialize(node) {
  if (node.nodeType === Node.TEXT_NODE) return node.data;
  if (node.nodeType !== Node.ELEMENT_NODE) return null;
  const out = { tag: node.localName };
  if (node.namespaceURI !== "http://www.w3.org/1999/xhtml") out.ns = node.namespaceURI;
  out.attributes = Array.from(node.attributes, (a) => [a.name, a.value]);
  if (node.style && node.style.length) {
    out.style = Array.from(node.style, (p) => [p, node.style.getPropertyValue(p), node.style.getPropertyPriority(p)]);
  }
  out.children = Array.from(node.childNodes, serialize).filter((n) => n !== null);
  return out;
}
async function settle(container) {
  let last = null;
  let same = 0;
  const start = performance.now();
  while (performance.now() - start < 15000) {
    await new Promise((resolve) => setTimeout(resolve, 25));
    const now = container.innerHTML;
    if (now === last) {
      if (++same >= 4) return true;
    } else {
      same = 0;
      last = now;
    }
  }
  return false;
}
const message = (e) => (e && typeof e.message === "string" ? e.message : String(e));
window.renderCase = async (spec) => {
  const errors = [];
  window.addEventListener("error", (event) => errors.push(message(event.error ?? event.message)));
  const { chartName, ...chartProps } = dec(spec.chart);
  const children = spec.children.map((child, i) => React.createElement(Recharts[child.type], { key: String(i), ...dec(child.props) }));
  const container = document.getElementById("root");
  const root = createRoot(container, {
    onUncaughtError: (e) => errors.push(message(e)),
    onCaughtError: (e) => errors.push(message(e)),
    onRecoverableError: (e) => errors.push(message(e)),
  });
  root.render(React.createElement(Recharts[chartName], chartProps, ...children));
  const settled = await settle(container);
  const span = document.getElementById("recharts_measurement_span");
  const font = span ? getComputedStyle(span).font : null;
  const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return { dom: Array.from(container.childNodes, serialize).filter((n) => n !== null), errors, measurements, tickFonts: fonts, font, settled, timeZone };
};
`;

const topPage = (concurrency) => `<!doctype html><html><body><script>
(async () => {
  const cases = await (await fetch("/cases")).json();
  const results = new Array(cases.length);
  let next = 0;
  async function run(spec) {
    const frame = document.createElement("iframe");
    frame.src = "/frame";
    const loaded = new Promise((resolve) => frame.addEventListener("load", resolve, { once: true }));
    document.body.append(frame);
    await loaded;
    try {
      return await frame.contentWindow.renderCase(spec);
    } finally {
      frame.remove();
    }
  }
  async function worker() {
    while (next < cases.length) {
      const i = next++;
      results[i] = await run(cases[i]);
    }
  }
  await Promise.all(Array.from({ length: ${concurrency} }, worker));
  const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  await fetch("/result", {
    method: "POST",
    body: JSON.stringify({ userAgent: navigator.userAgent, timeZone, offset: new Date(0).getTimezoneOffset(), results }),
  });
})().catch((e) => fetch("/result", { method: "POST", body: JSON.stringify({ failure: String(e && e.stack || e) }) }));
</script></body></html>`;

const framePage = `<!doctype html><html><head><meta charset="utf-8"></head><body><div id="root"></div><script src="/bundle.js"></script></body></html>`;

// Wraps util/DOMUtils.js's getStringSize so each call is recorded.
function measurePlugin(state) {
  return {
    name: "record-getStringSize",
    setup(build) {
      build.onLoad({ filter: /[\\/]recharts[\\/]es6[\\/]util[\\/]DOMUtils\.js$/ }, (args) => {
        const source = readFileSync(args.path, "utf8");
        const head = "export var getStringSize = function getStringSize(text) {";
        if (!source.includes(head)) throw new Error("getStringSize changed in " + args.path);
        state.wrapped = true;
        return {
          loader: "js",
          contents:
            source.replace(head, "var measuredGetStringSize = function getStringSize(text) {") +
            "\nexport var getStringSize = function getStringSize() {\n" +
            "  var result = measuredGetStringSize.apply(this, arguments);\n" +
            "  window.__recordMeasure(arguments, result);\n" +
            "  return result;\n};\n",
        };
      });
    },
  };
}

async function bundle() {
  const esbuild = createRequire(join(here, "package.json"))("esbuild");
  const state = { wrapped: false };
  const out = await esbuild.build({
    stdin: { contents: frameEntry, resolveDir: here, loader: "js" },
    bundle: true,
    write: false,
    format: "iife",
    platform: "browser",
    define: { "process.env.NODE_ENV": '"production"' },
    plugins: [measurePlugin(state)],
    logLevel: "error",
  });
  if (!state.wrapped) throw new Error("recharts/es6/util/DOMUtils.js was not bundled");
  return { js: out.outputFiles[0].text, esbuildVersion: esbuild.version };
}

// Chrome takes its time zone from the operating system, not from TZ. Time
// scales tick in local time, and the port's local time is UTC
// (docs/framework-gaps.md, TZ1), so the page is opened through the DevTools
// protocol with the time zone overridden to UTC.
async function inUtc(profile, url) {
  let port = null;
  for (let i = 0; i < 200 && port === null; i++) {
    try {
      port = readFileSync(join(profile, "DevToolsActivePort"), "utf8").split("\n")[0];
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }
  if (port === null) throw new Error("Chrome did not open its DevTools port");
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = targets.find((t) => t.type === "page");
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let id = 0;
  const pending = new Map();
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    const done = pending.get(message.id);
    if (!done) return;
    pending.delete(message.id);
    if (message.error) done.reject(new Error(`${message.error.message}`));
    else done.resolve(message.result);
  });
  const send = (method, params) =>
    new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject });
      socket.send(JSON.stringify({ id, method, params }));
    });
  const { product } = await send("Browser.getVersion", {});
  await send("Emulation.setTimezoneOverride", { timezoneId: "UTC" });
  await send("Page.navigate", { url });
  return product;
}

function record(chrome, cases, concurrency) {
  return bundle().then(
    ({ js, esbuildVersion }) =>
      new Promise((resolve, reject) => {
        const profile = mkdtempSync(join(tmpdir(), "dom-fixtures-"));
        let browser;
        let version = null;
        const server = createServer((req, res) => {
          if (req.method === "POST") {
            let body = "";
            req.on("data", (chunk) => (body += chunk));
            req.on("end", () => {
              res.end("ok");
              browser.kill();
              server.close();
              const result = JSON.parse(body);
              if (result.failure) reject(new Error(result.failure));
              else resolve({ ...result, esbuildVersion, version });
            });
            return;
          }
          const send = (type, text) => {
            res.setHeader("content-type", type);
            res.end(text);
          };
          if (req.url === "/") send("text/html", topPage(concurrency));
          else if (req.url === "/frame") send("text/html", framePage);
          else if (req.url === "/bundle.js") send("text/javascript", js);
          else if (req.url === "/cases") send("application/json", JSON.stringify(cases));
          else {
            res.statusCode = 404;
            res.end();
          }
        });
        server.listen(0, "127.0.0.1", () => {
          const url = `http://127.0.0.1:${server.address().port}/`;
          browser = spawn(chrome, [
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-background-timer-throttling",
            "--disable-renderer-backgrounding",
            "--remote-debugging-port=0",
            `--user-data-dir=${profile}`,
            "about:blank",
          ]);
          browser.on("exit", () => {
            try {
              rmSync(profile, { recursive: true, force: true });
            } catch {
              // Chrome may still hold files in the profile briefly.
            }
          });
          inUtc(profile, url).then((product) => (version = product)).catch((error) => {
            browser.kill();
            server.close();
            reject(error);
          });
        });
      }),
  );
}

// ------------------------------------------------------------------- main

function versions() {
  const pin = JSON.parse(readFileSync(join(here, "package-lock.json"), "utf8")).packages;
  return Object.fromEntries(["recharts", "react", "react-dom", "esbuild"].map((name) => [name, pin[`node_modules/${name}`].version]));
}

function serialize(header, cases) {
  const lines = ["{"];
  for (const [key, value] of Object.entries(header)) lines.push(`  ${JSON.stringify(key)}: ${JSON.stringify(value)},`);
  lines.push('  "cases": [');
  cases.forEach((c, i) => lines.push(`    ${JSON.stringify(c)}${i + 1 < cases.length ? "," : ""}`));
  lines.push("  ]", "}");
  return lines.join("\n") + "\n";
}

async function main() {
  const cases = allCases();
  if (process.argv.includes("--check-inputs")) {
    const fixture = JSON.parse(readFileSync(fixturePath, "utf8"));
    const recorded = fixture.cases.map((c) => ({ index: c.index, chart: c.chart, children: c.children }));
    const same = JSON.stringify(recorded) === JSON.stringify(cases) && JSON.stringify(fixture.versions) === JSON.stringify(versions());
    console.log(`${cases.length} cases`);
    if (!same) {
      console.error("dom.json was recorded from other cases or package versions; run node dom_fixtures.mjs <chrome>");
      process.exit(1);
    }
    return;
  }
  const chrome = process.argv[2];
  if (!chrome) {
    console.error("usage: node dom_fixtures.mjs <chrome executable> | --check-inputs");
    process.exit(2);
  }
  const concurrency = 6;
  const run = await record(chrome, cases, concurrency);
  if (run.esbuildVersion !== versions().esbuild) throw new Error(`esbuild ${run.esbuildVersion} is not the pinned one`);
  const unsettled = cases.filter((c, i) => !run.results[i].settled).map((c) => c.index);
  if (unsettled.length) throw new Error(`cases that did not settle within 15 s: ${unsettled.join(", ")}`);
  const zones = new Set([run.timeZone, ...run.results.map((r) => r.timeZone)]);
  if (zones.size !== 1 || run.timeZone !== "UTC" || run.offset !== 0) throw new Error(`not in UTC: ${[...zones]}`);
  const header = {
    generator: `tools/reference/dom_fixtures.mjs (Node ${process.versions.node}, esbuild ${run.esbuildVersion})`,
    browser: run.version,
    userAgent: run.userAgent,
    timeZone: run.timeZone,
    timezoneOffset: run.offset,
    versions: versions(),
  };
  const out = cases.map((c, i) => {
    const r = run.results[i];
    return { ...c, dom: r.dom, errors: r.errors, measurements: r.measurements, tickFonts: r.tickFonts, font: r.font };
  });
  const text = serialize(header, out);
  writeFileSync(fixturePath, text);
  const errors = out.filter((c) => c.errors.length).length;
  const measured = out.reduce((n, c) => n + c.measurements.length, 0);
  console.log(`wrote dom.json (${(text.length / 1024).toFixed(0)} KiB): ${out.length} cases, ${errors} with errors, ${measured} text measurements, from ${run.version} in ${run.timeZone}`);
}

await main();
