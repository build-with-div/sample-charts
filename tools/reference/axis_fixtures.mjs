// Generates the fixtures of the d3 port and of Recharts' axis selectors by
// running the pinned packages the way a page bundles them: Recharts' browser
// build (`recharts/es6`) and the npm d3 modules it imports through
// victory-vendor. See esm-loader.mjs.
//
//   npm ci --ignore-scripts
//   node axis_fixtures.mjs          # write the fixtures
//   node axis_fixtures.mjs --check  # fail if the committed fixtures differ
//
// Writes crates/js/tests/fixtures/math.json, crates/d3/tests/fixtures/d3.json
// and crates/recharts/tests/fixtures/axes.json.
//
// Values are written tagged, so the Rust side rebuilds them exactly:
// "u" undefined, null, true/false, "n:<String(n)>" numbers ("n:-0" for -0),
// "s:<text>" strings, [..] arrays, {"o": {..}} objects.
//
// Math.pow is not the same everywhere. Chromium 147's V8 calls the C
// library's pow (V8 12.4, in Node 22, used fdlibm's; V8's main branch now
// uses LLVM libc's), and C libraries disagree on some results: measured on
// Windows 11, Chrome for Testing 147.0.7727.50 gives 1.0000000000000001e23
// for Math.pow(10, 23), while the ucrtbase.dll pow Rust links also rounds
// 10^126 up. Every result below 10^±23 agreed. So the fixtures only use
// Math.pow(10, n) with integral |n| <= 22 (or n NaN or infinite, which the
// spec defines), which the generator enforces, and
// every call is recorded with its result (`powCalls`) for the Rust tests to
// check against the port's Math.pow. Node's result must be correctly rounded
// for every call (Node 22's fdlibm pow gives 9.999999999999999e-5 for
// 10^-4; Node 24.22.3 gave Chromium 147's result on every call measured).

import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";
import { register } from "node:module";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");

// ---------------------------------------------------------------- encoding

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

// ------------------------------------------------------- deterministic input

let seed = 0x5eed2;
function rand() {
  seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
  return seed / 2 ** 32;
}
const pick = (list) => list[Math.floor(rand() * list.length)];
const chance = (p) => rand() < p;
const int = (lo, hi) => lo + Math.floor(rand() * (hi - lo + 1));
// 10^n parsed, not computed: `**` calls the C library's pow, whose results
// differ between platforms and Node versions, and the inputs must not.
const pow10 = (n) => Number(`1e${n}`);

// A number a chart's data could hold.
function value() {
  // Magnitudes stay below 1e21 so the ticks' Math.pow(10, n) calls keep
  // |n| <= 22 (see the note at the top).
  const exponent = pick([-7, -3, -2, -1, 0, 0, 1, 1, 2, 2, 2, 3, 3, 4, 6, 9, 15, 18, 20]);
  const digits = pick([1, 1, 2, 2, 3, 4, 6, 17]);
  let v = (Math.round(rand() * pow10(digits)) / pow10(digits)) * pow10(exponent);
  if (chance(0.1)) v = v + 0.1 + 0.2;
  if (chance(0.3)) v = -v;
  return v;
}

// ------------------------------------------------------------ math fixture

function mathFixture() {
  const log10 = [];
  const log = [];
  for (let i = 0; i < 3000; i++) {
    const x = Math.abs(value()) * (1 + rand());
    log10.push([enc(x), enc(Math.log10(x))]);
    log.push([enc(x), enc(Math.log(x))]);
  }
  for (let k = -30; k <= 30; k++) {
    for (const m of [1, 2, 2.5, 5, 7.5]) {
      const x = m * pow10(k);
      for (const y of [x, x * (1 - 2 ** -52), x * (1 + 2 ** -52)]) {
        log10.push([enc(y), enc(Math.log10(y))]);
      }
    }
  }
  for (const x of [0, -0, -1, Infinity, -Infinity, NaN, 5e-324, 2.2250738585072014e-308, 1.7976931348623157e308]) {
    log10.push([enc(x), enc(Math.log10(x))]);
    log.push([enc(x), enc(Math.log(x))]);
  }
  const strings = [
    "", " ", "12", " 12 ", " 1.5﻿", "-0", "+.5", "5.", "1e3", "1E-3", "1e", ".", "e5",
    "-Infinity", "Infinity", "+Infinity", "infinity", "inf", "NaN", "0x1F", "0X1f", "0b101", "0o17",
    "-0x1", "0x", "1_000", "12px", "\u00851", " 1 ", "0x20000000000001", "0x20000000000003",
    "0.1", "1e400", "-1e-400", "00012", "1.2.3", "Jan", "1,000", "  -3.25e+2  ",
  ];
  const toNumber = strings.map((s) => [s, enc(Number(s))]);
  const parseFloats = [...strings, "10%", "2.5e1x", "-Infinityx", "x1"].map((s) => [s, enc(parseFloat(s))]);
  return { log10, log, toNumber, parseFloat: parseFloats };
}

// -------------------------------------------------------------- d3 fixture

function d3Fixture(d3Array, d3Scale) {
  const counts = [0, 1, 2, 3, 5, 6, 10, 0.5, 1.5, 2.5, -1, NaN, Infinity];
  const ticks = [];
  for (let i = 0; i < 1500; i++) {
    let start = value();
    let stop = chance(0.1) ? start : value();
    if (chance(0.05)) stop = pick([NaN, Infinity, -Infinity]);
    const count = pick(counts);
    const run = (f) => {
      try {
        return enc(f());
      } catch (error) {
        return { error: String(error.message) };
      }
    };
    ticks.push({
      start: enc(start),
      stop: enc(stop),
      count: enc(count),
      ticks: run(() => d3Array.ticks(start, stop, count)),
      tickIncrement: run(() => d3Array.tickIncrement(Math.min(start, stop), Math.max(start, stop), count)),
      tickStep: run(() => d3Array.tickStep(start, stop, count)),
    });
  }

  const linear = [];
  for (let i = 0; i < 600; i++) {
    const n = chance(0.15) ? 3 : 2;
    const domain = Array.from({ length: n }, value).sort((a, b) => a - b);
    if (chance(0.3)) domain.reverse();
    if (chance(0.05)) domain[1] = domain[0];
    const range = Array.from({ length: chance(0.1) ? 3 : 2 }, () => int(-500, 900) + (chance(0.3) ? rand() : 0));
    const scale = d3Scale.scaleLinear().domain(domain);
    const round = chance(0.2);
    if (round) scale.rangeRound(range);
    else scale.range(range);
    const clamp = chance(0.2);
    scale.clamp(clamp);
    const nice = chance(0.4) ? pick([undefined, 3, 5, 10]) : null;
    if (nice !== null) scale.nice(nice);
    const d = scale.domain();
    const inputs = [...d, (d[0] + d[d.length - 1]) / 2, d[0] - (d[d.length - 1] - d[0]), value(), NaN];
    const outputs = range.concat([(range[0] + range[range.length - 1]) / 2, value()]);
    const count = pick([undefined, 0, 2, 5, 10]);
    linear.push({
      domain: enc(domain),
      range: enc(range),
      round,
      clamp,
      nice: nice === null ? null : enc(nice),
      niceDomain: enc(d),
      apply: inputs.map((x) => [enc(x), enc(scale(x))]),
      invert: outputs.map((y) => [enc(y), enc(scale.invert(y))]),
      count: enc(count),
      ticks: enc(scale.ticks(count)),
    });
  }

  const band = [];
  const words = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  for (let i = 0; i < 400; i++) {
    const point = chance(0.4);
    const n = int(0, 14);
    const domain = Array.from({ length: n }, () =>
      chance(0.2) ? pick([0, -0, 1, 2.5, NaN, 10]) : pick(words),
    );
    const range = [int(-100, 600) + (chance(0.3) ? rand() : 0), int(-100, 900)];
    const scale = point ? d3Scale.scalePoint() : d3Scale.scaleBand();
    scale.domain(domain);
    const roundRange = chance(0.2);
    if (roundRange) scale.rangeRound(range);
    else scale.range(range);
    const settings = {};
    if (chance(0.3)) settings.padding = pick([0, 0.1, 0.25, 0.5, 1, 1.5]);
    if (!point && chance(0.3)) settings.paddingInner = pick([0, 0.2, 0.7, 1, 2]);
    if (!point && chance(0.3)) settings.paddingOuter = pick([0, 0.2, 0.5, 2]);
    if (chance(0.3)) settings.align = pick([0, 0.25, 0.5, 1, 2, -1]);
    if (chance(0.2)) settings.round = pick([true, false]);
    for (const [k, v] of Object.entries(settings)) scale[k](v);
    const probes = [...domain, "missing", 99];
    band.push({
      point,
      domain: enc(domain),
      range: enc(range),
      roundRange,
      settings: enc(settings),
      scaleDomain: enc(scale.domain()),
      bandwidth: enc(scale.bandwidth()),
      step: enc(scale.step()),
      apply: probes.map((x) => [enc(x), enc(scale(x))]),
    });
  }
  return { ticks, linear, band };
}

// ----------------------------------------------------------- chart fixture

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

function datum() {
  const r = rand();
  if (r < 0.04) return null;
  if (r < 0.07) return undefined;
  if (r < 0.12) return String(Math.round(value() * 100) / 100);
  if (r < 0.14) return "n/a";
  if (r < 0.17) return [Math.round(value()), Math.round(value())];
  if (r < 0.2) return NaN;
  const style = pick(["int", "int", "small", "big", "decimal"]);
  if (style === "int") return int(-50, 400);
  if (style === "small") return int(0, 9);
  if (style === "big") return int(0, 1e6) * pow10(int(0, 6));
  return Math.round(value() * 1000) / 1000;
}

function chartData(n) {
  const dupes = chance(0.15);
  return Array.from({ length: n }, (_, i) => {
    const row = {};
    if (!chance(0.03)) row.name = dupes && i > 0 && chance(0.3) ? months[(i - 1) % 12] : chance(0.05) ? i : months[i % 12];
    row.a = datum();
    if (row.a === undefined) delete row.a;
    row.b = datum();
    if (chance(0.3)) row.r = [int(0, 100), int(50, 300)];
    if (chance(0.3)) row.nested = { v: datum() };
    row.x = chance(0.9) ? int(-20, 120) + (chance(0.3) ? 0.5 : 0) : datum();
    return row;
  });
}

const valueKeys = ["a", "b", "r", "nested.v", "x", "missing", "a", "b"];

function axisProps(kind, layout, valueAxis, ids) {
  const p = {};
  const categoryAxis = (kind === "x") === (layout === "horizontal");
  if (ids.length > 1 || chance(0.2)) p[kind === "x" ? "xAxisId" : "yAxisId"] = ids[0];
  if (categoryAxis) {
    if (!chance(0.15)) p.dataKey = chance(0.85) ? "name" : pick(["x", "a", "missing"]);
    if (chance(0.2)) p.type = pick(["number", "category", "auto"]);
    else if (kind === "y") p.type = "category";
  } else {
    if (chance(0.15)) p.dataKey = pick(valueKeys);
    if (chance(0.15)) p.type = pick(["number", "category", "auto"]);
    else if (kind === "x") p.type = "number";
  }
  if (chance(0.35)) {
    p.domain = pick([
      ["auto", "auto"],
      [0, "auto"],
      ["dataMin", "dataMax"],
      ["dataMin - 10", "dataMax + 10"],
      ["dataMin - 0.5", "dataMax +  2.25"],
      [0, 100],
      [100, 0],
      [-50, 50],
      ["auto", 50],
      [0, "dataMax"],
      [NaN, 10],
      [0, Infinity],
      ["dataMin-1", "foo"],
      [0],
      ["Jan", "Mar", "Feb"],
    ]);
  }
  if (chance(0.15)) p.allowDataOverflow = true;
  if (chance(0.15)) p.allowDecimals = false;
  if (chance(0.15)) p.allowDuplicatedCategory = false;
  if (chance(0.1)) p.includeHidden = true;
  if (chance(0.1)) p.reversed = true;
  if (chance(0.1)) p.hide = true;
  if (chance(0.05)) p.mirror = true;
  if (chance(0.3)) p.tickCount = pick([0, 1, 2, 3, 5, 6, 8, 11]);
  if (chance(0.1)) p.ticks = pick([[0, 50, 100], ["Jan", "Feb", "Zzz"], [], [-10, 1e3, "7"]]);
  if (chance(0.25)) p.niceTicks = pick(["auto", "none", "snap125", "adaptive"]);
  if (chance(0.2)) {
    p.padding = pick([
      "gap",
      "no-gap",
      "wide",
      { left: 10, right: 20 },
      { top: 15, bottom: 5 },
      { left: 7 },
    ]);
  }
  if (chance(0.15)) p.scale = pick(["auto", "linear", "band", "point", "Linear", "foo", "log", "time", "implicit"]);
  if (chance(0.15)) p.orientation = kind === "x" ? pick(["top", "bottom"]) : pick(["left", "right"]);
  if (kind === "x" && chance(0.15)) p.height = pick([0, 20, 45.5]);
  if (kind === "y" && chance(0.15)) p.width = pick([0, 40, 80.25]);
  return p;
}

function chartCase(index) {
  const chartName = pick(["BarChart", "BarChart", "LineChart", "AreaChart", "ComposedChart"]);
  const layout = chance(0.25) ? "vertical" : "horizontal";
  const chart = {
    chartName,
    width: int(120, 900) + (chance(0.2) ? 0.5 : 0),
    height: int(100, 500),
  };
  if (chance(0.4)) chart.layout = layout;
  else if (layout === "vertical") chart.layout = "vertical";
  if (chance(0.3)) chart.margin = pick([{ top: 12, right: 12, left: 0, bottom: 0 }, { left: 12, right: 12 }, {}, { top: 3.5 }]);
  if (!chance(0.05)) chart.data = chartData(pick([0, 1, 2, 3, 5, 6, 7, 12, 12]));
  if (chance(0.15)) chart.barCategoryGap = pick(["20%", 4, "0%", "%", "", "5%%", 0]);
  if (chance(0.08)) chart.stackOffset = pick(["expand", "none", "wiggle"]);

  const xIds = chance(0.15) ? [0, "top"] : [0];
  const yIds = chance(0.2) ? [0, "right"] : [0];
  const xAxes = chance(0.08) ? [] : xIds.map((id) => axisProps("x", layout, layout === "vertical", [id]));
  const yAxes = chance(0.1) ? [] : yIds.map((id) => axisProps("y", layout, layout === "horizontal", [id]));
  if (yIds.length > 1 && yAxes.length > 1 && !("orientation" in yAxes[1])) yAxes[1].orientation = "right";

  const kinds = {
    BarChart: ["bar"],
    LineChart: ["line"],
    AreaChart: ["area"],
    ComposedChart: ["bar", "line", "area"],
  }[chartName];
  const items = Array.from({ length: int(0, 3) }, (_, i) => {
    const item = { kind: pick(kinds), id: `item-${i}` };
    if (!chance(0.05)) item.dataKey = pick(valueKeys);
    if (chance(0.1)) item.hide = true;
    if (yIds.length > 1 && chance(0.4)) item.yAxisId = "right";
    if (xIds.length > 1 && chance(0.4)) item.xAxisId = "top";
    if (chance(0.03)) item.yAxisId = "nowhere";
    if (item.kind !== "line" && chance(0.08)) item.stackId = pick(["a", 1]);
    if (item.kind !== "bar" && chance(0.1)) item.data = chartData(int(0, 6));
    return item;
  });
  return { index, chart, xAxes, yAxes, items };
}

// The shadcn chart examples of docs/design.md, as they mount.
function shadcnCases() {
  const barData = [
    { month: "January", desktop: 186, mobile: 80 },
    { month: "February", desktop: 305, mobile: 200 },
    { month: "March", desktop: 237, mobile: 120 },
    { month: "April", desktop: 73, mobile: 190 },
    { month: "May", desktop: 209, mobile: 130 },
    { month: "June", desktop: 214, mobile: 140 },
  ];
  return [
    {
      index: "shadcn-bar-default",
      chart: { chartName: "BarChart", width: 500, height: 300, data: barData },
      xAxes: [{ dataKey: "month" }],
      yAxes: [],
      items: [{ kind: "bar", id: "desktop", dataKey: "desktop" }],
    },
    {
      index: "shadcn-bar-multiple",
      chart: { chartName: "BarChart", width: 500, height: 300, data: barData },
      xAxes: [{ dataKey: "month" }],
      yAxes: [],
      items: [
        { kind: "bar", id: "desktop", dataKey: "desktop" },
        { kind: "bar", id: "mobile", dataKey: "mobile" },
      ],
    },
    {
      index: "shadcn-line-default",
      chart: { chartName: "LineChart", width: 500, height: 300, data: barData, margin: { left: 12, right: 12 } },
      xAxes: [{ dataKey: "month" }],
      yAxes: [],
      items: [{ kind: "line", id: "desktop", dataKey: "desktop" }],
    },
    {
      index: "shadcn-bar-horizontal",
      chart: { chartName: "BarChart", width: 500, height: 300, data: barData, layout: "vertical", margin: { left: -20 } },
      xAxes: [{ type: "number", dataKey: "desktop", hide: true }],
      yAxes: [{ dataKey: "month", type: "category" }],
      items: [{ kind: "bar", id: "desktop", dataKey: "desktop" }],
    },
  ];
}

// ---------------------------------------------------------- worker: charts

async function runCharts(cases) {
  register(pathToFileURL(join(here, "esm-loader.mjs")));
  const powCalls = new Map();
  const log10Calls = new Map();
  const pow = Math.pow;
  const log10 = Math.log10;
  Math.pow = function (x, y) {
    const r = pow(x, y);
    powCalls.set(`${enc(x)} ${enc(y)}`, [enc(x), enc(y), enc(r)]);
    return r;
  };
  Math.log10 = function (x) {
    const r = log10(x);
    log10Calls.set(enc(x), [enc(x), enc(r)]);
    return r;
  };
  const es6 = (path) => import(pathToFileURL(join(here, "node_modules", "recharts", "es6", path)).href);
  const { createRechartsStore } = await es6("state/store.js");
  const ax = await es6("state/selectors/axisSelectors.js");
  const { addXAxis, addYAxis } = await es6("state/cartesianAxisSlice.js");
  const { setChartData } = await es6("state/chartDataSlice.js");
  const { setChartSize, setLayout, setMargin } = await es6("state/layoutSlice.js");
  const { updateOptions } = await es6("state/rootPropsSlice.js");
  const { addCartesianGraphicalItem } = await es6("state/graphicalItemsSlice.js");
  const { xAxisDefaultProps } = await es6("cartesian/XAxis.js");
  const { yAxisDefaultProps } = await es6("cartesian/YAxis.js");
  const { defaultCartesianChartProps } = await es6("chart/CartesianChart.js");
  const { resolveDefaultProps } = await es6("util/resolveDefaultProps.js");
  const { getAxisTypeBasedOnLayout } = await es6("util/getAxisTypeBasedOnLayout.js");
  const { getNormalizedStackId } = await es6("util/ChartUtils.js");
  const d3Scale = await import("victory-vendor/d3-scale");
  const d3Array = await import("victory-vendor/d3-array");

  if (workerData.d3) {
    parentPort.postMessage({ d3: d3Fixture(d3Array, d3Scale), scaleNames: Object.keys(d3Scale).filter((k) => k.startsWith("scale")) });
  }

  // The settings SetXAxisSettings / SetYAxisSettings dispatch
  // (cartesian/XAxis.js, cartesian/YAxis.js).
  function axisSettings(kind, props, layout) {
    const p = resolveDefaultProps(props, kind === "x" ? xAxisDefaultProps : yAxisDefaultProps);
    const settings = {
      allowDataOverflow: p.allowDataOverflow,
      allowDecimals: p.allowDecimals,
      allowDuplicatedCategory: p.allowDuplicatedCategory,
      angle: p.angle,
      dataKey: p.dataKey,
      domain: p.domain,
      hide: p.hide,
      id: kind === "x" ? p.xAxisId : p.yAxisId,
      includeHidden: p.includeHidden,
      interval: p.interval,
      minTickGap: p.minTickGap,
      mirror: p.mirror,
      name: p.name,
      orientation: p.orientation,
      padding: p.padding,
      reversed: p.reversed,
      scale: p.scale,
      tick: p.tick,
      tickCount: p.tickCount,
      tickFormatter: p.tickFormatter,
      ticks: p.ticks,
      unit: p.unit,
      niceTicks: p.niceTicks,
    };
    if (kind === "x") settings.height = p.height;
    else settings.width = p.width;
    return { ...settings, type: getAxisTypeBasedOnLayout(layout, kind === "x" ? "xAxis" : "yAxis", p.type) };
  }

  // The settings Bar, Line and Area dispatch (SetCartesianGraphicalItem).
  function itemSettings(item) {
    const p = { xAxisId: 0, yAxisId: 0, hide: false, ...Object.fromEntries(Object.entries(item).filter(([, v]) => v !== undefined)) };
    const base = { type: item.kind, id: item.id, xAxisId: p.xAxisId, yAxisId: p.yAxisId, zAxisId: 0, dataKey: p.dataKey, hide: p.hide, isPanorama: false };
    if (item.kind === "bar") return { ...base, data: undefined, stackId: p.stackId == null ? undefined : getNormalizedStackId(p.stackId) };
    if (item.kind === "area") return { ...base, data: p.data, stackId: getNormalizedStackId(p.stackId) };
    return { ...base, data: p.data };
  }

  function axisResult(state, axisType, id) {
    const call = (selector) => selector(state, axisType, id, false);
    const applied = call(ax.selectAllAppliedValues).map((v) => v.value);
    const out = {
      appliedValues: enc(applied),
      realScaleType: enc(call(ax.selectRealScaleType)),
      domain: enc(call(ax.selectAxisDomain)),
      niceTicks: enc(call(ax.selectNiceTicks)),
      domainIncludingNiceTicks: enc(call(ax.selectAxisDomainIncludingNiceTicks)),
      checkedDomain: enc(call(ax.selectCheckedAxisDomain)),
      range: enc(call(ax.selectAxisRange)),
      rangeWithReverse: enc(call(ax.selectAxisRangeWithReverse)),
    };
    const scale = call(ax.selectAxisScale);
    out.scale =
      scale == null
        ? null
        : {
            domain: enc(scale.domain()),
            range: enc(scale.range()),
            bandwidth: enc(scale.bandwidth ? scale.bandwidth() : undefined),
            // map(v) and map(v, { position: "middle" }) of each applied value
            map: applied.map((v) => [enc(scale.map(v)), enc(scale.map(v, { position: "middle" }))]),
          };
    // Each tick as [index, coordinate, value, offset].
    const ticks = call(ax.selectTicksOfAxis);
    out.ticks =
      ticks == null
        ? enc(ticks)
        : ticks.map((t) => {
            const keys = Object.keys(t).sort().join();
            if (keys !== "coordinate,index,offset,value") throw new Error(`tick keys ${keys}`);
            return [enc(t.index), enc(t.coordinate), enc(t.value), enc(t.offset)];
          });
    return out;
  }

  for (const c of cases) {
    try {
      const store = createRechartsStore({
        options: { chartName: c.chart.chartName, defaultTooltipEventType: "axis", validateTooltipEventTypes: ["axis"], tooltipPayloadSearcher: () => undefined, eventEmitter: undefined },
      });
      const root = resolveDefaultProps(c.chart, defaultCartesianChartProps);
      store.dispatch(setChartData(c.chart.data));
      store.dispatch(setLayout(root.layout));
      store.dispatch(setMargin(root.margin));
      store.dispatch(setChartSize({ width: c.chart.width, height: c.chart.height }));
      store.dispatch(
        updateOptions({
          accessibilityLayer: root.accessibilityLayer,
          barCategoryGap: root.barCategoryGap,
          barGap: root.barGap,
          barSize: root.barSize,
          maxBarSize: root.maxBarSize,
          stackOffset: root.stackOffset,
          syncId: root.syncId,
          syncMethod: root.syncMethod,
          className: root.className,
          baseValue: root.baseValue,
          reverseStackOrder: root.reverseStackOrder,
        }),
      );
      for (const props of c.xAxes) store.dispatch(addXAxis(axisSettings("x", props, root.layout)));
      for (const props of c.yAxes) store.dispatch(addYAxis(axisSettings("y", props, root.layout)));
      for (const item of c.items) store.dispatch(addCartesianGraphicalItem(itemSettings(item)));
      const state = store.getState();
      const ids = (kind) => {
        const key = kind === "x" ? "xAxisId" : "yAxisId";
        const all = [0, ...c[kind === "x" ? "xAxes" : "yAxes"].map((p) => p[key] ?? 0), ...c.items.map((i) => i[key] ?? 0)];
        return [...new Set(all)];
      };
      const axes = [];
      for (const [kind, axisType] of [["x", "xAxis"], ["y", "yAxis"]]) {
        for (const id of ids(kind)) {
          let result;
          try {
            result = axisResult(state, axisType, id);
          } catch (error) {
            result = { error: String(error.message) };
            axes.push({ axisType, id: enc(id), ...result });
            parentPort.postMessage({ case: { ...encodeCase(c), axes: [...axes] }, failed: true });
            throw Object.assign(new Error("restart"), { handled: true });
          }
          axes.push({ axisType, id: enc(id), ...result });
        }
      }
      parentPort.postMessage({ case: { ...encodeCase(c), axes } });
    } catch (error) {
      if (error.handled) break;
      throw error;
    }
  }
  parentPort.postMessage({ done: true, powCalls: [...powCalls.values()], log10Calls: [...log10Calls.values()] });
}

function encodeCase(c) {
  return {
    index: c.index,
    chart: enc(c.chart),
    xAxes: enc(c.xAxes),
    yAxes: enc(c.yAxes),
    items: enc(c.items),
  };
}

// ------------------------------------------------------------------- main

function runWorker(cases, d3) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(fileURLToPath(import.meta.url), { workerData: { cases, d3 } });
    const results = [];
    let failedAt = null;
    let extra = {};
    worker.on("message", (m) => {
      if (m.case) {
        results.push(m.case);
        if (m.failed) failedAt = results.length;
      }
      if (m.d3) extra = { ...extra, d3: m.d3, scaleNames: m.scaleNames };
      if (m.done) extra = { ...extra, powCalls: m.powCalls, log10Calls: m.log10Calls };
    });
    worker.on("error", reject);
    worker.on("exit", () => resolve({ results, failedAt, ...extra }));
  });
}

async function main() {
  const check = process.argv.includes("--check");
  const cases = [...shadcnCases(), ...Array.from({ length: 1200 }, (_, i) => chartCase(i))];
  const charts = [];
  const powCalls = new Map();
  const log10Calls = new Map();
  let d3 = null;
  let scaleNames = null;
  let next = 0;
  let restarts = 0;
  while (next < cases.length || d3 === null) {
    const run = await runWorker(cases.slice(next), d3 === null);
    if (run.d3) {
      d3 = run.d3;
      scaleNames = run.scaleNames;
    }
    for (const [x, y, r] of run.powCalls ?? []) powCalls.set(`${x} ${y}`, [x, y, r]);
    for (const [x, r] of run.log10Calls ?? []) log10Calls.set(x, [x, r]);
    charts.push(...run.results);
    next += run.results.length;
    if (run.failedAt !== null) restarts++;
    else if (next < cases.length) throw new Error(`worker stopped at case ${next} without an error`);
  }
  const sortCalls = (calls) => [...calls.values()].sort((a, b) => (a.join(" ") < b.join(" ") ? -1 : 1));
  const dec = (s) => (s === "n:-0" ? -0 : Number(s.slice(2)));
  const unsafe = [...powCalls.values()].filter(([x, y, r]) => {
    const n = dec(y);
    // NaN and infinite exponents are the spec's special cases, not the C
    // library's.
    if (!Number.isFinite(n)) return false;
    return !(dec(x) === 10 && Number.isInteger(n) && Math.abs(n) <= 22 && dec(r) === Number(`1e${n}`));
  });
  if (unsafe.length) {
    console.error("Math.pow calls whose result is not the same in every engine, or which Node rounds wrongly:");
    for (const call of unsafe) console.error("  ", call.join(" "));
    process.exit(1);
  }
  const pin = JSON.parse(readFileSync(join(here, "package-lock.json"), "utf8")).packages;
  const versions = Object.fromEntries(
    ["recharts", "d3-array", "d3-scale", "d3-interpolate", "victory-vendor", "decimal.js-light"].map((name) => [name, pin[`node_modules/${name}`].version]),
  );
  const generator = `tools/reference/axis_fixtures.mjs (Node ${process.versions.node}, V8 ${process.versions.v8})`;
  const files = {
    "crates/js/tests/fixtures/math.json": { generator, ...mathFixture() },
    "crates/d3/tests/fixtures/d3.json": { generator, versions, ...d3, powCalls: sortCalls(powCalls), log10Calls: sortCalls(log10Calls) },
    "crates/recharts/tests/fixtures/axes.json": { generator, versions, scaleNames, restarts, cases: charts, powCalls: sortCalls(powCalls) },
  };
  let stale = [];
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
      console.log(`wrote ${path} (${(text.length / 1024).toFixed(0)} KiB)`);
    }
  }
  console.log(`${charts.length} chart cases, ${restarts} worker restarts after exceptions, ${powCalls.size} Math.pow calls`);
  if (stale.length) {
    console.error(`fixtures differ from the packages' output: ${stale.join(", ")}; run node axis_fixtures.mjs`);
    process.exit(1);
  }
}

// One JSON value per line for arrays of records, so diffs stay readable.
function serialize(content) {
  const lines = ["{"];
  const entries = Object.entries(content);
  entries.forEach(([key, v], i) => {
    const comma = i + 1 < entries.length ? "," : "";
    if (Array.isArray(v) && v.length && typeof v[0] === "object") {
      lines.push(`  ${JSON.stringify(key)}: [`);
      v.forEach((item, j) => lines.push(`    ${JSON.stringify(item)}${j + 1 < v.length ? "," : ""}`));
      lines.push(`  ]${comma}`);
    } else if (v && typeof v === "object" && !Array.isArray(v)) {
      lines.push(`  ${JSON.stringify(key)}: {`);
      const inner = Object.entries(v);
      inner.forEach(([k, x], j) => {
        if (Array.isArray(x) && x.length && typeof x[0] === "object") {
          lines.push(`    ${JSON.stringify(k)}: [`);
          x.forEach((item, n) => lines.push(`      ${JSON.stringify(item)}${n + 1 < x.length ? "," : ""}`));
          lines.push(`    ]${j + 1 < inner.length ? "," : ""}`);
        } else {
          lines.push(`    ${JSON.stringify(k)}: ${JSON.stringify(x)}${j + 1 < inner.length ? "," : ""}`);
        }
      });
      lines.push(`  }${comma}`);
    } else {
      lines.push(`  ${JSON.stringify(key)}: ${JSON.stringify(v)}${comma}`);
    }
  });
  lines.push("}");
  return lines.join("\n") + "\n";
}

if (isMainThread) {
  await main();
} else {
  await runCharts(workerData.cases);
}
