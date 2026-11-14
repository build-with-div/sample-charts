# Charts on Div: design

Status: phase 2 (porting) from 13 November 2026; see
[Phase 2 milestones](#phase-2-milestones). The phase 1 evidence is in
[probes/](probes/).

The app is pinned to Div `92ea25a0`. The phase 1 statements and probe
measurements below are for Div `565db46a` (build-with-div/div `main` then);
statements about Div were checked against that source or measured with the
probes; statements about npm packages were checked against their source at
the versions named.
Measurements: Windows 11, 24 cores, release build, headless `Document`, CPU
raster, one run each unless stated.

## Principle

The sample ports popular chart packages and asks Div to provide the web
platform those packages run on, faithfully: same behaviour, same pixels as
pinned Chromium 147. What the sample cannot build is reported as a missing
platform feature with an owner. A workaround in the sample is scaffolding to
keep building, labelled `// div-gap:` and logged against the platform
feature that replaces it; it is never the proposed solution.

## Packages → platform features

| Package | Used by | Renders with | Platform features it runs on |
| --- | --- | --- | --- |
| **Recharts 3.8.0** (with shadcn `chart.tsx`, `shadcn@4.23.0`) | shadcn/ui charts | Inline SVG + HTML | Inline SVG shapes, text, paint, `clipPath`, gradients; CSS cascade and custom properties on SVG; `ResizeObserver`; `getBoundingClientRect`, `offsetWidth`; `getBBox`, `getTotalLength`; hit testing on SVG shapes; focusable `<svg>` with key events; CSS transitions; `requestAnimationFrame`; HTML/CSS for tooltip and legend |
| **visx** + **d3-shape** / **d3-scale** | Custom React charts | Inline SVG | Same SVG core as Recharts plus `pattern`, `marker`, `radialGradient`; d3-shape and d3-scale are pure math (ported as Rust code, no platform need) |
| **Chart.js 4** | General dashboards | `<canvas>` 2D | Canvas 2D context (paths, arcs, Béziers, dashes, gradients, text and `measureText`, clip, transforms, `drawImage`); `devicePixelRatio`; `ResizeObserver`; `requestAnimationFrame`; mouse/pointer events with element-relative coordinates. Ships LTTB and min-max decimation. |
| **ECharts** (zrender) | Large and streaming data | Several stacked `<canvas>` layers (or SVG) | Canvas 2D as above plus shadows, composite operations, patterns, radial gradients; layered canvases; wheel and pointer events; `requestAnimationFrame` |

Recharts' use of each feature was found in its source at `v3.8.0`
(`recharts/recharts`): `ResizeObserver` in `component/ResponsiveContainer.tsx`
and `chart/RechartsWrapper.tsx`; text measured through a hidden `<span>` and
`getBoundingClientRect` (`util/DOMUtils.ts`); pointer coordinates from
`getBoundingClientRect` with `getBBox` or `offsetWidth`
(`util/getRelativeCoordinate.ts`); `getTotalLength` for the stroke-dash
entrance animation (`cartesian/Line.tsx`, `shape/Rectangle.tsx`); CSS
`transition` on the tooltip's transform (`component/TooltipBoundingBox.tsx`)
and on bars (`animation/CSSTransitionAnimate.tsx`); `requestAnimationFrame`
for JS animation; `tabIndex=0` and `role` on the root `<svg>`
(`container/RootSurface.tsx`), with `accessibilityLayer` on by default in
cartesian and polar charts. The Chart.js and ECharts rows are from their
documentation and are not yet measured; see the
[canvas measurement plan](#canvas-2d-for-chartjs-and-echarts).

## SVG features ranked by shadcn's chart examples

shadcn `shadcn@4.23.0` ships 70 chart examples
(`apps/v4/registry/new-york-v4/charts`): 19 bar, 14 radar, 11 pie, 10 area,
10 line, 6 radial. Each row counts the examples that need the feature,
from the components and props they use and what Recharts 3.8.0 renders for
them.

| Platform feature | Examples (of 70) | Why | Div today |
| --- | --- | --- | --- |
| `<svg>` root (width/height/viewBox), `<g>`, `<path d>` | 70 | Every chart: surface, layers, all series shapes | `<svg>` is an empty replaced box; it ignores `width`/`height` (lays out 300×0 for `width=200 height=100`). `g`, `path`: `UnknownTag` (probe 7) |
| CSS cascade on SVG: `<style>`, class, attribute and descendant selectors setting `fill`/`stroke`/`outline` | 70 | `ChartContainer`'s classes (`[&_.recharts-cartesian-grid_line[stroke='#ccc']]:stroke-border/50` …) and `ChartStyle`'s `<style>` | No stylesheet cascade or class selectors (in progress); no SVG properties |
| `ResizeObserver` | 70 | `ResponsiveContainer` in every `ChartContainer` | Missing |
| Focusable `<svg>` (`tabindex`, `role`), `keydown`, focus outline | 70 | `accessibilityLayer` defaults to true | Focus and keys work on elements (probe 4); not on SVG |
| CSS custom properties, `var()` in SVG `fill`/`stroke` | 69 | `fill="var(--color-desktop)"`; `--color-*` set by `ChartStyle` | Missing |
| Mouse events on the wrapper, `getBoundingClientRect`, `offsetWidth` | 67 | Tooltip tracking (`axis` trigger) | Works (`bounding_rect`, `width`) |
| CSS transition on `transform` (HTML tooltip) | 67 | `TooltipBoundingBox` | Transitions exist (`Element::transition`); to verify against Chromium |
| `<text>`, `<tspan>`: `x`/`y`, `dy` in `em`, `text-anchor`, font from CSS | 60 | Axis ticks, labels, label lists, donut centre text | `UnknownTag` |
| `<line>` | 44 | Grids, polar grid spokes, tick lines | `UnknownTag` |
| Elliptical arcs (`A`) in `path` | 34 | Rounded bars (`radius`), pie and radial sectors | No SVG paths at all |
| `<circle>` | 28 | Dots, active dot on hover (default on for line and area), circular polar grid | `UnknownTag` |
| `fill-opacity`, `stroke-opacity` | 25 | Area fills, radar fills | — |
| `transform` attribute (`rotate(a, x, y)`, `translate`) | ≥4 | Rotated tick labels, polar radius axis | — |
| Cubic Béziers (`C`) in `path` | 16 | `type="natural"`, `"monotone"` curves | — |
| `stroke-dasharray` / `stroke-dashoffset` | 11 | Line entrance animation (10), dashed grid (1) | — |
| `getTotalLength()` on `path` | 10 | Line entrance animation | — |
| `clipPath`, `clip-path="url(#…)"` | 10 | Area entrance animation clip | — |
| Hit testing on SVG geometry | 10 | Pie tooltips (`item` trigger: hover on a sector) | No SVG geometry to hit |
| `linearGradient`, `stop`, `fill="url(#…)"` | 2 | `chart-area-gradient`, `chart-area-interactive` | — |

Counts come from a scan of the example sources (rules in the
[appendix](#appendix-how-the-examples-were-counted)); they are a ranking aid,
not a coverage claim. Every row is needed by at least one example; none is
optional for porting shadcn charts.

## Canvas 2D for Chart.js and ECharts

| Feature group | Needed for | Div today |
| --- | --- | --- |
| `CanvasRenderingContext2D` paths: `moveTo`, `lineTo`, `bezierCurveTo`, `quadraticCurveTo`, `arc`, `arcTo`, `rect`, `roundRect`, `closePath`, `Path2D` | All series | `<canvas>` is an empty replaced box (lays out 300×0; HTML's default is 300×150); no context |
| Stroke and fill state: `lineWidth`, `lineCap`, `lineJoin`, `miterLimit`, `setLineDash`, `lineDashOffset`, colours, `createLinearGradient`/`createRadialGradient`, `createPattern`, `globalAlpha`, `globalCompositeOperation`, `shadow*` | Lines, areas, grids; ECharts effects | — |
| `save`/`restore`, transforms, `clip` | Plot-area clipping during zoom/pan; rotated labels | — |
| `fillText`, `strokeText`, `measureText` (`TextMetrics`), `font`, `textAlign`, `textBaseline` | Axis labels, data labels | — |
| `drawImage`, `getImageData`/`putImageData`, `toDataURL`/`toBlob` | Point images, export to PNG | — |
| `isPointInPath`, `isPointInStroke` | Hit testing on irregular shapes | — |
| Backing store from `width`/`height` attributes and CSS size, `devicePixelRatio` | Sharp rendering at scale 2 | — |
| Several stacked canvases | ECharts layers (hover layer over a 100k-point series) | — |

The engine owns Div's 2D context, as a new `div-canvas` crate (platform
decision 8); the requests sent to the canvas design are in the shared notes
(`charts-canvas-api-requests.md`) and name these `CanvasRenderingContext2D`
members.

Measurement plan for the canvas packages: record the 2D context calls that
Chart.js and ECharts make for their sample charts in Chromium (a recording
proxy around `CanvasRenderingContext2D`), giving both a ranked API list (as
above for SVG) and replayable call traces. Replaying a trace through Div's
canvas and through Chromium's, then diffing pixels, is the canvas fidelity
test for these packages. Div's canvas itself is tested with Rust tests
generated from WPT's canvas YAML files (1,157 tests) and with the same kind
of trace replay (platform decision 8).

## The Div work list

This is the main deliverable. Each item is a web platform feature to
implement with Chromium's behaviour and pixels; Rust-typed handles on top
are welcome, web semantics underneath. Owners: **engine** (div-dom,
div-style, div-layout, div-paint, div-text, div-engine), **framework**
(divui, div-platform, div-macros; Canvas 2D is the engine's, in
`div-canvas`), **components** (div-components). Sizes: S (days), M (one to two weeks),
L (more). Priority: P0 blocks porting shadcn charts at all; P1 blocks some
examples or behaviours; P2 later.

| # | Platform feature | Examples | Owner | Size | Pri | Measured by |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Inline SVG core:** `svg` sizing (`width`, `height`, `viewBox`, `preserveAspectRatio`), `g`, `path` (full `d` grammar incl. arcs and Béziers), `line`, `rect`, `circle`, `ellipse`, `polygon`, `polyline`; presentation attributes; `fill`, `stroke`, `stroke-width`, `-linecap`, `-linejoin`, `-miterlimit`, `-dasharray`, `-dashoffset`, `opacity`, `fill-opacity`, `stroke-opacity`, `transform`; painted with Chromium's SVG paint setup. Scheduled right after the engine's Tier 0 items; the stroked-icons item becomes its stage 1, not a second painter; `view!` uses SVG's real attribute names (`stroke-width`, `viewBox`, case-sensitive) | 70 | engine (painting), framework (`view!` names) | L | P0 | WPT `svg/shapes`, `svg/painting`, `svg/path`, `svg/coordinate-systems`; harness |
| 2 | **SVG text:** `text`, `tspan`, `x`/`y`/`dx`/`dy` (incl. `em`), `text-anchor`, `dominant-baseline`, fonts from CSS | 60 | engine | L | P0 | WPT `svg/text`; harness |
| 3 | **CSS on SVG:** stylesheets (`<style>`), class/attribute/descendant selectors, SVG properties as CSS (`fill`, `stroke`, …), `outline`; shadcn's `ChartStyle` sheet through the browser-compatible ingestion mode for third-party sheets (drops what Chromium drops and reports it), beside compile-time checking for Div-authored CSS | 70 | engine | L (cascade in progress) | P0 | WPT `css/selectors`, `svg/styling`; harness |
| 4 | **CSS custom properties** and `var()`, including in SVG paint, and runtime writes (`style.setProperty('--x', …)`) | 69 | engine (framework: runtime API) | M | P0 | WPT `css/css-variables` |
| 5 | **`ResizeObserver`**: one API with content, border and device-pixel-content boxes; the engine computes observations after layout, the framework delivers them before paint, with Chromium's timing | 70 | engine + framework | M | P0 | WPT `resize-observer` (JS; port the cases) |
| 6 | **`requestAnimationFrame`** per document, returning a typed handle, in every window; tied to the frame clock and driven by `advance_time` in headless tests | 70 (mount animation, `isAnimationActive: 'auto'`) | framework | M | P0 | Frame-time traces against Chromium |
| 7 | **Focus and accessibility in SVG:** `tabindex`, `role`, `aria-*` on SVG elements, focus outline, graphics roles in the accessibility tree | 70 | engine + framework | M | P1 | Components trace harness (focus/DOM state) |
| 8 | **Paint servers and clipping:** `defs`, `linearGradient`, `radialGradient`, `stop`, `pattern`, `clipPath`, `url(#id)` references | 10 (clip), 2 (gradient) | engine | M | P1 | WPT `svg/pservers`, `css/css-masking/clip-path` |
| 9 | **SVG geometry APIs:** `getBBox`, `getTotalLength`, `getPointAtLength`, `getComputedTextLength` | 10+ | framework (engine geometry) | M | P1 | WPT `svg/types` cases, ported |
| 10 | **Hit testing on SVG geometry** and `pointer-events` for SVG | 10 | engine | M | P1 | Harness: hover traces on pie sectors |
| 11 | **CSS transitions and animations on SVG properties** (`stroke-dasharray`, `transform`, `opacity`), through the one Web Animations surface (CSS animations in `getAnimations()`) | 10+ | engine | M | P1 | WPT `css/css-transitions` with SVG targets |
| 12 | **Canvas 2D** (`CanvasRenderingContext2D`, `Path2D`, `TextMetrics`), backing store and DPR, stacked canvases | Chart.js, ECharts | engine (`div-canvas`) | L | P0 for canvas packages | Rust tests generated from WPT's canvas YAML (1,157); call-trace replay vs Chromium |
| 13 | `<svg>` and `<canvas>` intrinsic size from attributes and defaults (HTML: 200×100 from the attributes, 300×150 for a bare canvas; Div: 300×0 for both; Chromium not yet measured here) | 70 | engine | S | P0 | WPT `html/rendering/replaced-elements` |
| 14 | `contextmenu` event; right click must not fire `click` | — | framework | S | P1 | Event traces vs Chromium |
| 15 | Per-element cost: 10,000 positioned elements take about 1.9 s to build, lay out and paint (probe 2). Recharts emits one SVG element per dot, label and grid line, so this becomes the inline-SVG cost too | — | engine + framework | M | P1 | Probe 2 against Chromium timing (to measure) |
| 16 | Component context so `view!` can compose `<AreaChart><XAxis/>…</AreaChart>` as Recharts does | — | framework | M (in progress, RFC 0003 (a)) | P1 | — |
| 17 | Replaceable or releasable image resources (resources are append-only; decoded images cached for the thread's life) | Scaffolding only | engine | S | P2 | Probe 3 streaming |

Rows 1, 3–6, 11 and 12 follow the platform design decisions the user
accepted on 13 November 2026 (`platform-plan/decisions-2026-11-13.md`,
decisions 2, 3, 4, 8 and 9). The owning tracks record them in their own
RFCs; the API details of `requestAnimationFrame`, `ResizeObserver` and
Pointer Events (mouse `pointerId` 1, typed enums and bitflags with web
values) come from the framework's RFC.

Workarounds are not on this list; the scaffolding they replace is in
[Workarounds](#workarounds-scaffolding-only). The RFC draft for items 1–11
and 13 is [rfc-inline-svg.md](rfc-inline-svg.md).

## How the sample ports Recharts

The port is Rust code in this repository that reproduces Recharts' output,
not a new chart design:

- **Same math.** d3-scale (linear, band, time, log), Recharts' nice-tick
  rules, d3-shape curves (linear, natural, monotoneX, step), stack offsets,
  pie and radial angles, and Recharts' layout (margins, axis sizes, band and
  bar gaps) are ported line by line with their tests.
- **Same DOM.** Components render the SVG tree Recharts renders: element
  names, attributes and `recharts-*` class names, through Div's inline SVG
  (`view! { <svg><g class="recharts-layer"><path d=… /></g></svg> }`).
  shadcn's `ChartContainer`, `ChartStyle`, `ChartTooltipContent` and
  `ChartLegendContent` are ported with the same markup and styles.
- **Same behaviour.** Tooltip tracking from wrapper mouse events,
  `accessibilityLayer` keys, item hover on sectors, mount animations on
  animation frames, `ResponsiveContainer` on `ResizeObserver`.
- **Typed on top.** Rows are Rust structs and series take accessors instead of
  string `dataKey`s; colours are theme tokens. Underneath, the DOM is
  Recharts'.

```rust
#[derive(Clone)]
struct Visits { month: &'static str, desktop: f64, mobile: f64 }

let config = ChartConfig::new()
    .series("desktop", "Desktop", theme.chart_1)
    .series("mobile", "Mobile", theme.chart_2);

view! {
    <ChartContainer config=config class="aspect-auto h-[250px] w-full">
        <AreaChart data=data margin=Margin { left: 12.0, right: 12.0, ..Default::default() }>
            <CartesianGrid vertical=false />
            <XAxis key=|r: &Visits| r.month tick_line=false axis_line=false tick_margin=8.0
                   tick_formatter=|m: &str| m[..3].to_string() />
            <ChartTooltip cursor=false content=ChartTooltipContent::new().indicator(Indicator::Dot) />
            <Area key=|r: &Visits| r.mobile kind=Curve::Natural fill="var(--color-mobile)"
                  fill_opacity=0.4 stroke="var(--color-mobile)" stack_id="a" />
            <Area key=|r: &Visits| r.desktop kind=Curve::Natural fill="var(--color-desktop)"
                  fill_opacity=0.4 stroke="var(--color-desktop)" stack_id="a" />
        </AreaChart>
    </ChartContainer>
}
```

This form needs component context (work item 16) and `class` with a
stylesheet (item 3); until then the same components take a builder.

**Canvas packages.** Chart.js (and later ECharts) are ported the same way,
onto Div's Canvas 2D (item 12): the same draw calls in the same order, so
call traces compare directly. 100k+ point series go through this path, with
Chart.js's own decimation plugin (LTTB, min-max) ported. Recharts is an SVG
library and is not built for 100k-point series.

## Fidelity plan

The same input rendered both ways, diffed:

1. **Reference.** Pinned Recharts 3.8.0 and shadcn `chart.tsx` from
   `shadcn@4.23.0` (the tag Div's components harness vendors), built into
   static pages per example, light and dark, `isAnimationActive={false}` for
   still frames, fixed container size, deterministic data. Same build
   approach as Div's `tests/fidelity/components/reference`; the harness lives
   in this repository (`tests/fidelity/`) since Div is not edited from here.
2. **DOM first.** Serialise Chromium's chart subtree (tags, attributes,
   computed `fill`/`stroke`, text positions) and Div's tree; they must be
   equal. A mismatch is a port bug; equality means remaining pixel
   differences belong to Div's platform implementation.
3. **Pixels.** Compare per layer with masks (marks, grid, text, HTML tooltip
   and legend) at the census profiles (scale 1 and 2), WSL Chromium 147 with
   the census real-font environment. Report differing pixels and max delta;
   "exact" only at 0.
4. **Behaviour.** Event and focus traces (hover a point, arrow keys, hover a
   sector) through the components trace harness: same DOM state after each
   step.
5. **Platform conformance.** Div's WPT runner for the SVG and CSS
   directories named in the work list; the canvas trace replay for Canvas 2D.

Order: bar (largest share, boxes and arcs), then line and area (curves,
text), then pie and radar, then radial.

## What Div has today: probe results

Run from `docs/probes` with Div CI's Skia variables (`FORCE_SKIA_BUILD=1`,
`SKIA_GN_ARGS`, `SKIA_NINJA_COMMAND=ninja`, `RUST_MIN_STACK=4194304`,
`LIBCLANG_PATH`) and, on Windows, the VS 2022 build tools environment:
`cargo test --release --test <name> -- --nocapture --test-threads 1`.
Output goes to `docs/probes/out/` (ignored); the quoted runs are in
[probes/results/](probes/results/).

- **Probe 7, inline SVG and canvas (`inline_svg`).** `Element::create`
  returns `UnknownTag` for `g`, `path`, `line`, `rect`, `circle`, `polygon`,
  `text`, `tspan`, `defs`, `clipPath`, `linearGradient`, `radialGradient`,
  `stop`, `pattern`, `marker`, `title`, `desc`. `svg` and `foreignObject`
  are created. `<svg width="200" height="100" viewBox="0 0 200 100">` lays
  out 300×0; `<canvas>` lays out 300×0. Nothing is painted.
- **Probe 1, a polyline today (`polyline_today`).** An SVG image resource
  with stroked `<polyline>`, `<path>` and `<line>` paints 0 px: Div's static
  SVG image rasterizer reads `rect` and filled axis-aligned `path` only.
  Rotated `<div>`s per segment draw a line with butt joins; an app-side Skia
  raster shown as an `image/x-openui-rgba8` resource draws it fully. Both are
  scaffolding.
- **Probe 2, one element per mark (`mark_cost`).** N absolutely positioned
  4×4 dots in an 800×400 box; ms:

  | Model | N | Build | Layout | Raster | Move all | Re-layout | Re-raster |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | raw | 1,000 | 123.27 | 23.51 | 4.19 | 18.56 | 26.00 | 4.74 |
  | raw | 10,000 | 1,466.23 | 396.05 | 42.83 | 257.87 | 356.81 | 24.35 |
  | raw | 100,000 | 14,801.81 | 3,134.47 | 216.21 | 2,408.33 | 3,244.96 | 217.00 |
  | reactive | 10,000 | 1,974.55 | 327.02 | 22.11 | 236.03 | 321.23 | 21.96 |

  Building costs about 0.15 ms per element (seven `set_property` calls,
  each parsing a CSS literal). The Chromium time for the same page is not
  yet measured (work item 15).
- **Probe 3, decimation and the raster scaffold (`raster_cost`).** Stroking
  a 100k-point path with Skia takes 253 ms at 800×400 and 1M points 11 s;
  LTTB to 1,600 points takes 0.3 ms (100k) and 2.6 ms (1M), and the stroke
  then 4 ms. Replacing the scaffold image 120 times keeps 153.6 MB that is
  never freed (work item 17).
- **Probe 4, input (`interaction`).** With pointer events, `pointerdown`
  then `set_pointer_capture` delivers `pointermove`/`pointerup` outside the
  element. Mouse events go to the element under the pointer (a release
  outside the plot does not reach it, as in the DOM; Recharts' `Brush`
  listens on `window` for that, which the port needs a document-level
  listener for; not yet probed). Right click fires `click`
  with button 2 and no `contextmenu`. Ctrl+wheel gives `delta_y = -120` with
  modifiers. A focused element receives `keydown` with DOM key names.
- **Probe 5, plotters (`plotters_backend`).** plotters draws through a
  Skia-backed `DrawingBackend` (100k points in 94 ms), but its backend API is
  integer pixels and it has its own layout and text, so it is not a port of
  any of the packages above. Not pursued.
- **Probe 6, Skia raster vs Chromium SVG (`fidelity_sample`, `compare.py`).**
  The same geometry as SVG in Chromium 147 and as a plain Skia raster shown
  in Div. Div shows the raster losslessly (0 px differ), but the raster is not
  Chromium's SVG painting: scale 1, line 1,443 px / grid 955 / area 787 /
  bars 48 differ, all by 1 of 255; scale 2, grid 0, line 3,480 (max 15), bars
  97 (max 24), area 1,791 (max 4). So item 1 has to paint SVG with
  Chromium's SVG paint setup; drawing paths with default Skia settings is
  not enough.

## Workarounds (scaffolding only)

Allowed in the sample only to keep building, each labelled
`// div-gap: <id> <web feature>` and logged in `docs/framework-gaps.md`.

| Scaffold | Replaced by |
| --- | --- |
| Marks rasterized by the app with Skia and shown as an RGBA `<img>` | Items 1, 8 (SVG) or 12 (canvas) |
| Tick labels and SVG text as absolutely positioned `<span>`s | Item 2 |
| Colours passed as typed values instead of `var(--color-*)` | Items 3, 4 |
| Fixed chart sizes | Item 5 |
| Animation stepped by `task::interval` | Item 6 |
| Tooltip hit testing in data space for pie sectors | Item 10 |
| Data-space dash-length maths instead of `getTotalLength` | Item 9 |

## Phase 2 milestones

One focused PR per step, each merged before the next. Ported logic is
tested for equality with the pinned npm packages run under Node
(`tools/reference`); rendering is compared with the same input rendered by
the package in pinned Chromium 147, with measured numbers only.

Node is a faithful reference for the ported logic only where its V8 gives
Chromium 147's results. The one difference that matters so far is
`Math.pow`: Chromium 147's V8 calls the platform C library's `pow`, while
Node 22's V8 still used fdlibm's. On the 684 `Math.pow` calls the axis
fixtures made, Chrome for Testing 147.0.7727.50 (Windows 11, headless)
differed from Node 22.18.0 on 70, for example `Math.pow(10, -4)`
(Chrome 0.0001, Node 9.999999999999999e-5); `Math.log10` agreed on all
1,723 calls. The fixtures are therefore generated with Node 24.22.3 (V8
13.6.233.17, `node-v24.22.3-win-x64.zip` from nodejs.org, SHA-256
`e612af99f626ab415ea225af6bce5c971feb448cb5cae250bb1c3fa50c5f7d9f`, checked
against the release's `SHASUMS256.txt`), whose V8 also calls the C library:
on the same 684 calls it gave Chromium's result for every one, both for
`Math.pow` and for `**`. CI checks the fixtures with the same version on
Linux. The generators build their inputs without `pow` (10^n is parsed from
`1e<n>`), so the inputs do not depend on the platform; with that, Node 22
reproduces the nice-tick fixture byte for byte. The port's `Math.pow`
(`crates/js`) does what Chromium does; the fixtures record every `Math.pow`
call with its result, and the tests check the port agrees on each.

| # | Content | Runs on Div today | Gaps it reaches |
| --- | --- | --- | --- |
| M1 | Workspace pinned to Div `92ea25a0`, CI, app window; Recharts' numeric tick algorithm (`getNiceTickValues`, `getTickValuesFixedDomain`) with decimal.js-light's arithmetic, equal to recharts@3.8.0 on 3,487 generated cases | yes | V1 (no charts drawn) |
| M2 | d3-array 3.2.5 ticks, d3-scale 4.0.3 linear, band and point scales (the npm modules Recharts' browser build imports through victory-vendor 37.4.1); Recharts' cartesian axis selectors: applied values, domain (`'auto'`, `dataMin`/`dataMax`, `allowDataOverflow`, user domains), nice ticks, range and padding, the configured scale and the axis ticks. Equal to recharts@3.8.0 on all 2,657 axes of 1,204 generated and shadcn charts, except 128 axes needing what is not ported yet (58 time or log scales, 70 stacked series) | yes | — |
| M2b | d3-scale time and log scales (with d3-time and d3-time-format) for axes; the tick label filter (`interval`, `minTickGap`), which measures text | yes; label filter waits for V4 | V4, V2 (text measurement) |
| M3a | d3-path 3.1.1 (with `digits` rounding) and d3-shape 3.2.1's `line` and `area` with the 13 curves Recharts names (basis, basisClosed, basisOpen, bumpX, bumpY, linear, linearClosed, monotoneX, monotoneY, natural, step, stepAfter, stepBefore); Recharts' `Curve` path (`getCurveFactory`, `getPath`: base lines, layouts, `connectNulls`). Equal to the packages on 2,000 generated line and area paths and 2,000 `getPath` calls | yes | — |
| M3b | d3-shape stack and offsets; the stacked domain in the axis selectors (the 70 stacked axes M2 leaves out) | yes | — |
| M3c | d3-shape arcs, pie layout and symbols; Recharts' rectangle and sector path strings | yes | — |
| M4 | Recharts' cartesian and polar layout producing the SVG element tree as data; DOM-equality harness against the same chart rendered by Recharts in Chromium 147 | partly (tree only) | V1, V2, V4, S1, S2, O1 |
| M5 | shadcn's HTML parts (`ChartContainer` sizing, `ChartTooltipContent`, `ChartLegendContent`) as Div elements, pixel-compared with Chromium | yes, with cascade limits | S1, S2, S3 |
| M6 | Gallery of shadcn's bar, line, area, pie, radar and radial examples through Div's SVG once V1 lands; until then the tree stays unrendered (no private SVG painter) | waits for V1 | V1, V2, V3, A1, R1, R7 |
| M7 | Interaction: tooltip tracking, `accessibilityLayer` keyboard navigation, sector hover, brush | partly | V5, I1, I2 |
| M8 | Chart.js port on Canvas 2D | waits for C1–C4 | C1–C4, O1, R1, R3 |

## Appendix: how the examples were counted

The 70 example sources at `shadcn@4.23.0` were scanned for Recharts
components and props. Rules: every example needs the SVG root, layers,
paths, `ResizeObserver`, the stylesheet and a focusable root (Recharts
`accessibilityLayer` defaults to true); `var(--color-` for custom
properties; any axis, label or `<text>` for SVG text; `CartesianGrid`,
`PolarGrid` or axes with tick lines for `<line>`; `Pie`, `RadialBar` or
`Bar` with `radius` for arcs; `Line`, `Area`, `dot`, `activeDot` or a
circular polar grid for `<circle>`; `type="natural"`/`"monotone"` for
Béziers; `Line` or `strokeDasharray` for dashes; `Area` for `clipPath`;
`PieChart` with a tooltip for shape hit testing; `linearGradient` for
gradients. The `transform` row is a lower bound (`PolarRadiusAxis` alone).
