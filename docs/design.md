# Charts on Div: design

Status: phase 1 (investigation), 13 November 2026. Nothing here is built
yet except the probes in [probes/](probes/).

Pinned Div: `565db46a` (build-with-div/div `main`). Every statement about
Div below was checked against that source or measured with the probes;
measurements give the command and conditions. Windows 11, 24 cores, release
build of the probes (`opt-level = 3`), headless `Document`, CPU raster.

## Goal

A gallery of professional desktop charts built with Div's public API, used
to find what Div lacks:

- line, area, bar (stacked and grouped), scatter, pie and donut, sparkline;
- axes with nice ticks, gridlines, legends;
- hover crosshair and tooltip, selection (brushing), zoom and pan;
- live (streaming) updates and animated transitions;
- 100k+ point series with decimation (LTTB);
- shadcn theme tokens, accessibility (data table fallback, keyboard focus
  on points), export to PNG.

The visual reference is shadcn/ui charts, which are Recharts (SVG) with
shadcn's `ChartContainer`, `ChartTooltipContent` and `ChartLegendContent`,
rendered in pinned Chromium 147.

## Summary

1. Div cannot draw a diagonal line with its own primitives. Its SVG image
   rasterizer draws filled rects and axis-aligned paths only; a stroked
   `<polyline>`, `<path>` or `<line>` paints nothing (probe 1: 0 px of ink).
2. Two workarounds exist today. A rotated `<div>` per segment draws a line
   (butt joins, one element per segment). An app-side Skia raster handed to
   Div as an `image/x-openui-rgba8` resource draws anything Skia can.
3. Div's image path is lossless: the app's Skia raster and Div's render of
   it are pixel-equal (probe 6, 0 px differ at scale 1 and 2).
4. That raster is close to Chromium's SVG painting of the same geometry but
   not equal: up to 1,443 px differ at scale 1 (each by 1 of 255), and up
   to 3,480 px at scale 2 with up to 15 (lines) and 24 (rounded bars) of
   255 (probe 6). Exactness needs the drawing to go through Div's
   Chromium-matched paint setup, which is the canvas work's job.
5. Image resources cannot be replaced or freed: every update registers a
   new resource, and Div caches each decoded image for the thread's life.
   Streaming 120 frames of an 800x400 chart keeps
   153.6 MB (probe 3). The raster route suits static and slowly
   changing charts only.
6. One element per mark costs about 1.9 s for 10,000 dots (build + layout +
   raster) and about 0.64 s to move them all (probe 2). 100k marks as
   elements is about 18 s to mount and 5.9 s to move.
7. Pointer events with capture work for brushing and panning; wheel events
   carry deltas and modifiers; a focused element receives arrow keys (probe 4).
   Right click fires `click` (button 2) and no `contextmenu`.
8. `plotters` works through a Skia-backed `DrawingBackend` (100k-point line
   in 94 ms, probe 5) but hands the backend integer pixel coordinates, draws
   text through its own font stack, and has its own layout and tick rules, so
   it cannot match Recharts geometry.
9. **Recommendation:** a purpose-built Div chart layer: Recharts-compatible
   scales, layout and curves in Rust; marks drawn on a 2D canvas surface;
   axes text, legend, tooltip, crosshair and the data table as ordinary Div
   elements. It starts on the raster workaround behind a `PlotSurface`
   trait and moves to the canvas when it lands.

## What Div has today, by probe

The probes are integration tests in [probes/](probes/), using only `divui`
(lib name `div`) and `skia-safe` (the same Skia build Div links, no extra
features). Run from `docs/probes`:

```
cargo test --release --test <name> -- --nocapture --test-threads 1
```

with Div CI's Skia variables set (`FORCE_SKIA_BUILD=1`, `SKIA_GN_ARGS`,
`SKIA_NINJA_COMMAND=ninja`, `RUST_MIN_STACK=4194304`, `LIBCLANG_PATH`) and,
on Windows, the VS 2022 build tools environment. Output images go to
`docs/probes/out/` (ignored); the runs quoted here are kept in
[probes/results/](probes/results/).

### Probe 1: drawing a polyline (`polyline_today`)

A 40-point, 2 px polyline, 360x160 px, three ways:

| Way | Result | Build |
| --- | --- | --- |
| A. 39 `<div>`s, each `transform: rotate(..rad)` about its centre | Draws (2,966 px of ink). Butt joins leave notches at sharp turns; no miter or round joins. | 4.94 ms |
| B. `<img>` with an `image/svg+xml` resource: `<polyline>`, `<path d="M..L..">`, `<line>`, all `fill="none" stroke=..` | **Nothing drawn (0 px).** Div's static SVG rasterizer (`div-paint/src/image_resource.rs`, `parse_static_svg`) reads `rect` (filled and stroked) and filled axis-aligned `path` only. | 0.18 ms |
| C. `<img>` with an `image/x-openui-rgba8` resource the app rasterized with Skia | Draws (3,047 px), anti-aliased, with SVG's default miter joins. | 0.88 ms |

First render of all three: 7.71 ms. `Document::hit_test` at the middle of a
rotated segment returns a `<div>` (the segment or its panel; the probe does
not tell them apart).

### Probe 2: one element per mark (`mark_cost`)

N absolutely positioned 4x4 px dots (`border-radius: 2px`) in an 800x400
plot. "raw" sets styles through `Element::set_property`; "reactive" binds
each dot to its own signal through an effect (what a `For`/`view!` chart
does) and moves all of them in one `batch`. Times in ms.

| Model | N | Build | Layout | Raster | Move all | Re-layout | Re-raster |
| --- | --- | --- | --- | --- | --- | --- | --- |
| raw | 100 | 10.77 | 2.56 | 3.34 | 1.96 | 1.85 | 3.06 |
| raw | 1,000 | 123.27 | 23.51 | 4.19 | 18.56 | 26.00 | 4.74 |
| raw | 10,000 | 1,466.23 | 396.05 | 42.83 | 257.87 | 356.81 | 24.35 |
| raw | 100,000 | 14,801.81 | 3,134.47 | 216.21 | 2,408.33 | 3,244.96 | 217.00 |
| reactive | 100 | 16.06 | 3.65 | 2.47 | 1.83 | 3.23 | 2.76 |
| reactive | 1,000 | 275.49 | 60.70 | 4.34 | 23.08 | 45.37 | 5.46 |
| reactive | 10,000 | 1,974.55 | 327.02 | 22.11 | 236.03 | 321.23 | 21.96 |
| reactive | 100,000 | 14,811.46 | 3,543.17 | 242.96 | 5,355.10 | 2,988.84 | 189.13 |

Building costs about 0.15 ms per dot (seven `set_property` calls, each
parsing a CSS literal); style and layout about 0.03–0.04 ms per dot; both
grow linearly. Moving every dot costs about as much as laying them out.
One run; single-run numbers vary, so read them as orders of magnitude.

### Probe 3: app-side raster at scale (`raster_cost`)

A line series, stroked 2 px by Skia into an 800x400 image (scale 1 and 2),
optionally decimated with LTTB to 1,600 points, then registered with Div,
set on an `<img>`, laid out and rendered. Times in ms.

| N | Scale | LTTB to | LTTB | Map | Raster | Hand-off + render | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1,000 | 1 | – | 0.01 | 0.00 | 3.98 | 20.61 | 24.61 |
| 10,000 | 1 | – | 0.06 | 0.06 | 12.20 | 20.69 | 33.01 |
| 10,000 | 1 | 1,600 | 0.04 | 0.01 | 3.98 | 19.43 | 23.46 |
| 100,000 | 1 | – | 0.51 | 0.72 | 253.34 | 21.15 | 275.72 |
| 100,000 | 1 | 1,600 | 0.33 | 0.01 | 4.18 | 19.91 | 24.43 |
| 100,000 | 2 | – | 0.51 | 0.78 | 331.47 | 23.83 | 356.60 |
| 100,000 | 2 | 1,600 | 0.28 | 0.01 | 8.46 | 25.14 | 33.90 |
| 1,000,000 | 1 | – | 5.39 | 7.37 | 10,985.40 | 18.08 | 11,016.24 |
| 1,000,000 | 1 | 1,600 | 2.60 | 0.02 | 4.32 | 18.85 | 25.80 |
| 1,000,000 | 2 | 1,600 | 2.95 | 0.02 | 8.49 | 23.36 | 34.83 |

(1,000,000 points at scale 2 without LTTB: 23,625 ms raster. Full table in
the probe output.)

- Decimation is required, not optional: stroking a dense, self-overlapping
  100k-point path takes 253 ms and 1M takes 11 s, while LTTB to 1,600
  points takes 0.3 ms (100k) and 2.6 ms (1M) and the stroke 4 ms.
- "Hand-off + render" (register, set on the `<img>`, `update_all`,
  `render_to_bitmap` of the whole 800x400 document) is about 20 ms at
  scale 1 whatever the series. Part of it is the full-frame headless
  render; it is not a measurement of a native window's frame time.

Streaming (`streaming_frames`): a 10k-point window sliding over 100k
points, LTTB to 1,600, re-rastered and replaced 120 times: mean 29.91 ms per frame, worst 44.63 ms, and **153.6 MB of
resource bytes that are never freed** (120 × 1.28 MB, plus the decoded copy
Div caches for each).

### Probe 4: input on a plot surface (`interaction`)

An `<img>` plot at (50, 50), 400x200, `tabindex="0"`:

- `dispatch_pointer_event(.., pointer_names: true)`: `pointerdown` inside,
  `set_pointer_capture` → `Ok`, then `pointermove` and `pointerup` at
  (600, 300), outside the plot, **are delivered to the plot**. A brush or pan
  drag works. `click` fires on the plot after the release.
- `dispatch_mouse_event`: `mousedown` inside, then a move outside gives only
  `mouseleave`, and the `mouseup` outside is not delivered (mouse events have
  no capture; Chromium's implicit capture would deliver it).
- Event coordinates are viewport (`mouse_x`, `mouse_y`); plot-local
  coordinates need `bounding_rect()` (no `offsetX`).
- Right button: `mousedown`/`mouseup` and a **`click` with button 2**, no
  `contextmenu` (Chromium fires `contextmenu` and `auxclick`, not `click`).
- `wheel` with Ctrl: `delta_y = -120`, modifiers present. Usable for zoom.
- `focus()` then `dispatch_key(ArrowRight)` and `Home`: `keydown` reaches
  the plot with the DOM key names.
- Accessibility: the update has 2 nodes for this page; an `<img>` exposes no
  structure, so a chart needs a separate accessible fallback.

### Probe 5: `plotters` through a Skia backend (`plotters_backend`)

A `DrawingBackend` on a `skia_safe::Canvas` (about 100 lines) drew a
plotters line chart; the result shown in Div as a PNG resource. 1,000 points:
2.13 ms; 100,000 points: 94.42 ms (one `draw_path` call; plotters does not
decimate; it is likely quicker than probe 3's 253 ms because plotters rounds
points to whole pixels first, not checked). See [Approach 3](#3-an-existing-rust-charting-crate).

### Probe 6: fidelity of a Skia raster against Chromium SVG (`fidelity_sample`, `compare.py`)

Four scenes at 400x200 CSS px, each written twice: as SVG in an HTML page,
rendered by pinned Chromium 147 (Div's own `render_chrome` from
`tools/accountability/run_all_pixel_comparisons.py`: headless,
`--disable-gpu`, CDP device metrics), and as the same geometry drawn with
`skia-safe` and shown in Div. Compared pixel for pixel (any channel
difference counts):

| Scene | Scale 1: px differ / max delta | Scale 2: px differ / max delta |
| --- | --- | --- |
| line (40 points, 2 px, `chart-1`) | 1,443 / 1 | 3,480 / 15 |
| grid (5 dashed 1 px lines, `3 3`) | 955 / 1 | **0** |
| area (filled path, opacity 0.4) | 787 / 1 | 1,791 / 4 |
| bars (6 bars, Recharts radius-4 arc path vs Skia `RRect`) | 48 / 1 | 97 / 24 |

- Div's image path adds nothing: the raw Skia raster and Div's render are
  equal (0 px) in all eight cases.
- Filling the opaque background first (so marks blend once, as in the page)
  changes the area case only, by 15 px at scale 1.
- The remaining differences are between this Skia setup and Chromium's SVG
  painting. Not diagnosed further: candidates are Chromium's paint flags and
  raster tiling for SVG shapes, and the bar case draws an `RRect` where
  Recharts emits an arc path. Div's own painter already carries
  Chromium-matching raster logic, so a canvas that paints through it is the
  place to reach exactness.

Command: `cargo test --release --test fidelity_sample`, then in WSL
`python3 compare.py --div-checkout <Div at 565db46a>`.

## Approaches

### 1. Elements and SVG

Recharts renders one SVG element per mark. In Div the equivalent is one
element per mark, since stroked SVG does not exist.

- **Fidelity.** Bars, gridlines (axis-aligned, solid), crosshair, legend
  and tooltip map onto boxes and could be exact (they are CSS boxes and
  text, which Div matches Chromium on). Lines, areas, curves, dashed lines
  (no dashed borders at 1 px pitch matching `3 3`), arcs and pie slices
  cannot be drawn: rotated divs approximate lines with butt joins, nothing
  approximates a filled area or arc.
- **Performance.** See probe 2: 10,000 dots take about 1.9 s to mount and 0.64 s to move; 100,000 take about 18 s and 5.9 s. 100k points as elements is
  not viable.
- **Interactivity.** Best of the three: every mark is a hit-testable,
  focusable element with its own listeners and accessibility node.
- **Accessibility.** Natural: role and label per mark, real focus order.
- **Effort.** Low for bars; blocked for everything with curves until
  stroked SVG lands (engine, L), and even then 100k-point series need a
  canvas.
- **Verdict.** Use elements for what is a box anyway: tooltip, legend,
  crosshair line, brush rectangle, focus ring, axis labels. Not for marks.

### 2. A 2D canvas surface

One canvas per plot area; the chart layer draws all marks into it. This is
how Chart.js, ECharts and uPlot reach 100k+ points.

- **Fidelity.** Equal to Chromium only if the canvas paints with Div's
  Chromium-matched setup (probe 6 shows a plain Skia surface is close, not
  equal). Target: Recharts' SVG paths drawn with the same geometry give the
  same pixels.
- **Performance.** Skia strokes a 100k-point path in 253 ms at 800x400
  (probe 3); LTTB to 1,600 points and the stroke take 4.5 ms together
  (24.4 ms with the image hand-off and a full headless render). Hover needs
  an overlay so the series is not redrawn per pointer move.
- **Interactivity.** Hit testing in data space (binary search on sorted x;
  a uniform grid for scatter; angle and radius for pie). `is_point_in_path`
  helps for irregular shapes. Pointer capture already works (probe 4).
- **Accessibility.** A canvas is one node. The chart supplies a fallback:
  a visually hidden data table, and a keyboard model (Recharts'
  `accessibilityLayer`: arrow keys move the active point and the tooltip
  follows) announced through a live region.
- **Effort.** Depends on the canvas work (sample-canvas session); the
  chart layer is the same either way.
- **Today.** The raster workaround is this approach with an image as the
  surface: fully drawable, lossless through Div, but no in-place update
  (resource leak per frame) and no Chromium exactness.

### 3. An existing Rust charting crate

- **plotters** (0.3.7, measured): a backend is easy to write (probe 5), and
  a Div canvas backend would be about the same 100 lines. But the backend API
  is integer pixels (`BackendCoord = (i32, i32)`), so SVG's fractional
  geometry and half-pixel 1 px lines are lost; text goes through plotters'
  own font features rather than Div's text stack; layout, tick and curve
  rules are plotters', not Recharts'; there is no interaction, hit testing,
  animation or accessibility model. It cannot match shadcn charts.
- **Not considered further** (from their documentation, not measured):
  `charming` and `plotly` render through JavaScript chart libraries (Div has
  no JavaScript); `egui_plot` is tied to egui's immediate-mode UI;
  `poloto` and similar emit SVG text, which Div cannot stroke.
- **Verdict.** Offer a plotters backend on the Div canvas later as an
  adapter for scientific plots (S once the canvas exists). The shadcn-style
  chart layer is purpose-built.

### Comparison

| | Elements/SVG | Canvas (raster today) | plotters |
| --- | --- | --- | --- |
| Lines, areas, arcs | No (rotated divs for lines only) | Yes | Yes |
| Match Recharts in Chromium | Boxes and text only | Close today (probe 6); exact needs canvas through Div's paint | No (integer coords, own layout) |
| 100k points | ~18 s mount (probe 2) | 24.4 ms with LTTB | 94 ms, no decimation |
| Hit testing, focus | Per element | In the chart layer | None |
| Streaming | Element churn | Leaks today; canvas fixes | Redraw only |
| Effort | Low for bars, blocked for curves | Chart layer M–L + canvas | Backend S; still no Recharts match |

## Recommended architecture

```
data (Vec<Row>, typed accessors)
  │
  ├─ scales: linear, band, time, log; Recharts' nice-tick rule
  ├─ layout: Recharts' margins, axis sizes, band gaps, legend space
  ├─ geometry: d3-shape curves (linear, monotoneX, natural, step),
  │            stack offsets, pie arcs, bar paths with radius
  ├─ decimation: LTTB / min-max per pixel column for the visible range
  │
  ├─ PlotSurface (trait) ─┬─ RasterSurface: skia-safe → rgba8 <img>   (today; div-gap)
  │   marks, grid         └─ CanvasSurface: Div 2D canvas              (when it lands)
  │
  └─ Div elements: axis tick labels, legend, tooltip, crosshair line,
                   brush rectangle, focus ring, hidden data table
```

- **One plot surface per chart, plus overlays as elements.** The crosshair
  is a 1 px vertical box, the brush a translucent rectangle, the active dot
  a small rounded box: all axis-aligned, so they are elements and moving
  them does not redraw the series. Only zoom, pan and data changes redraw
  the surface.
- **Text as elements first.** Recharts' tooltip and legend are HTML in the
  page, so Div elements are the exact counterpart. Tick labels are SVG
  `<text>` in Recharts; they start as absolutely positioned spans, measured
  through layout (Recharts measures through a hidden span too). Whether
  spans match SVG text pixels is a fidelity question to measure; canvas
  text is the fallback.
- **Interaction state in signals.** `active_index`, `hover_point`,
  `brush_range`, `view_domain` are signals; overlays and tooltip react to
  them; the surface redraws on `view_domain` and data changes.
- **Large data.** Decimate for the visible domain to about two points per
  pixel column, off the UI thread with `task::spawn` for 1M+ points, and
  cache per zoom level. Scatter uses a density grid when points exceed the
  pixel count.
- **Animation.** Interpolate geometry (Recharts animates path points and bar
  heights from the previous state) on a frame callback; honour
  `Document::prefers_reduced_motion`. Until a public frame callback exists,
  `task::interval` drives it (not frame-synced).
- **Theming.** `ChartConfig` maps series keys to a label and a color; colors
  come from `div_components::theme` (`chart_1` … `chart_5` exist for light
  and dark). Dark mode is an app-level theme signal (Div has no media
  queries or CSS variables).
- **Export.** Mount the same chart into an offscreen `Document` at the
  export size and scale, `render_to_png_buffer`, save through the `dialog`
  feature. No per-element export API is needed for this.

## Public API sketch

Rows are typed; series name their accessor instead of a string key.

```rust
#[derive(Clone)]
struct Visits { month: &'static str, desktop: f64, mobile: f64 }

let config = ChartConfig::new()
    .series("desktop", "Desktop", theme.chart_1)
    .series("mobile", "Mobile", theme.chart_2);

let chart = AreaChart::new(data)            // Signal<Vec<Visits>> or Vec<Visits>
    .margin(Margin { left: 12.0, right: 12.0, ..Margin::default() })
    .grid(CartesianGrid::new().vertical(false))
    .x_axis(XAxis::category(|r: &Visits| r.month)
        .tick_line(false).axis_line(false).tick_margin(8.0)
        .tick_format(|m| m[..3].to_string()))
    .area(Area::new("mobile", |r: &Visits| r.mobile)
        .curve(Curve::Natural).fill_opacity(0.4).stack("a"))
    .area(Area::new("desktop", |r: &Visits| r.desktop)
        .curve(Curve::Natural).fill_opacity(0.4).stack("a"))
    .tooltip(ChartTooltip::new().cursor(false).indicator(Indicator::Dot))
    .legend(ChartLegend::new());

view! {
    <ChartContainer config=config style:height="250px">
        <Chart chart=chart on_active=move |i| active.set(i) />
    </ChartContainer>
}
```

- A builder, not Recharts' child composition, because `view!` children are
  `ViewNode`s, not configuration a parent can read. The composed form
  (`<AreaChart><XAxis/>…</AreaChart>`) becomes possible with component
  context (framework RFC 0003 (a), in progress) and can be added on top.
- Same shape for `LineChart`, `BarChart` (`.bar(..)` with `stack`/`radius`;
  grouped when unstacked), `ScatterChart`, `PieChart` (`.pie(Pie::new(..)
  .inner_radius(60.0))` for a donut) and `Sparkline` (no axes, no
  interaction, fixed small size).
- Interaction options: `.brush(Brush::x())`, `.zoom(Zoom::x().wheel(true))`,
  `.stream(window: usize)`, `.decimate(Decimate::Lttb)`,
  `.accessibility_layer(true)` (default on, as shadcn examples set it).
- `ChartContainer`, `ChartTooltipContent` and `ChartLegendContent` follow
  shadcn's markup and classes as Div styles; a later move into
  `div-components` keeps the API.

## Div capabilities needed

Owner tracks: **engine** (div-style, div-layout, div-paint, div-text,
div-engine), **framework** (divui, div-platform, div-macros, div-cli),
**components** (div-components), **canvas** (the 2D canvas work). Sizes:
S (days), M (a week or two), L (more). Priority: P0 blocks the gallery
done properly; P1 blocks an interaction or quality feature; P2 later.

| # | Capability | Owner | Size | Pri | Today / workaround |
| --- | --- | --- | --- | --- | --- |
| 1 | 2D canvas element with a retained surface: paths (lines, cubic Béziers, arcs, round rects), stroke width/cap/join/dash, solid and linear-gradient fill, save/restore, transforms, clip | canvas | L | P0 | Skia raster as an image resource (`div-gap`) |
| 2 | Canvas painting through Div's Chromium-matched paint setup, so canvas paths equal Chromium's SVG paths (probe 6 deltas) | canvas, engine | M | P0 | Accept small deltas; measure |
| 3 | Public animation-frame callback, driven by `advance_time`/`begin_frame` in headless tests | framework (canvas) | M | P0 | `task::interval`, not frame-synced, not testable by time |
| 4 | Replaceable or releasable image resources (`unregister_image_resource`, or update in place); decoded-image cache eviction | engine | S | P0 until 1 lands | New resource per update; leaks (153.6 MB per 120 frames) |
| 5 | Canvas text: `fill_text` with Div fonts and `measure_text` agreeing with DOM shaping | canvas | M | P1 | Labels as DOM spans |
| 6 | Text measurement without layout (`measure_text(style, text)`) | framework / engine | S | P1 | Hidden span + `update_all` + `width()` |
| 7 | Element resize notification and device-pixel-ratio change (ResizeObserver-like) for responsive charts | framework | M | P0 | Fixed chart sizes; window size only |
| 8 | Cheap overlay layer or stacked canvases (crosshair over 100k points without redraw) | canvas | S | P1 | Crosshair as elements |
| 9 | `is_point_in_path` / `is_point_in_stroke` | canvas | S | P1 | Hit testing in data space |
| 10 | Canvas accessibility: fallback children in the accessibility tree and focus order | canvas, engine | M | P1 | Hidden data table + focusable plot |
| 11 | `contextmenu` event; right click must not fire `click` | framework | S | P1 | Check `mouse_button == 2` in `click` |
| 12 | Implicit capture for mouse events (mouseup outside after mousedown) | framework | S | P2 | Use pointer events with explicit capture |
| 13 | Plot-local event coordinates (`offset_x`/`offset_y`) | framework | S | P2 | `bounding_rect()` per event |
| 14 | Copy image to clipboard | framework | S | P2 | Export to file only |
| 15 | Stroked SVG (`path`, `line`, `polyline`, `circle`) in the static SVG image rasterizer | engine | L | P2 for charts (icons need it more) | Not used by the chart layer |
| 16 | Component context for composed charts (`<AreaChart><XAxis/>…`) | framework | M | P2 | Builder API (in progress as RFC 0003 (a)) |
| 17 | Layout cost with nested flex (dashboard of chart cards) | engine | M | P1 | In progress; absolutely positioned frame |
| 18 | `:hover`/`:focus-visible` styles (legend items, tooltip rows) | engine | M | P2 | Signals per hover (in progress) |
| 19 | Top layer for tooltips that leave a clipped card | engine / framework | M | P2 | Tooltip inside the chart box |
| 20 | Chart components in `div-components` (container, tooltip, legend content) | components | L | later | In this sample first |

## Fidelity plan against shadcn charts

1. **Reference pages.** Vendor shadcn's `chart.tsx` from the same tag the
   components harness pins (`shadcn@4.23.0`, `new-york-v4`) and the
   Recharts version its registry entry pins. Build static pages from the
   shadcn chart examples (area, bar stacked and grouped, line, scatter, pie,
   donut, radial later) with deterministic data, `isAnimationActive={false}`,
   a fixed-size container instead of `ResponsiveContainer`, light and dark.
   Same build approach as Div's `tests/fidelity/components/reference`.
2. **Harness.** A copy of the components harness's compare mode in this
   repo (`tests/fidelity/`), run in WSL with Chromium 147, real fonts
   (Geist, the census font profile) and the census scales (1 and 2). It
   stays here because Div is not edited from this sample; it moves to Div if
   charts move into `div-components`.
3. **Geometry first.** Extract every Recharts path `d`, `x`/`y`, and text
   position from the Chromium DOM and compare with the Rust layer's numbers
   before comparing pixels (exact string or 1e-6 equality). This isolates
   the port (nice ticks, band scale, curves, stack, pie angles, layout) from
   raster differences.
4. **Pixels by layer.** Compare with masks: plot marks, grid, axis text,
   legend and tooltip separately, so text differences do not hide exact
   geometry and the other way round. Report counts and max deltas like
   probe 6; claim "exact" only at 0 px.
5. **Order.** Bar chart first (boxes: largest exact share possible today),
   then line and area (needs canvas exactness, capability 2), then pie.

## Performance plan

- Report per chart: first mount, update on new data, hover move (overlay
  only), zoom step, at 1k, 10k, 100k and 1M points, with LTTB on and off,
  at scale 1 and 2, release build, headless and native window.
- Budgets to test against (targets, not claims): a hover move under 4 ms,
  a zoom redraw under 16 ms at 100k points.

## What can be prototyped now

With Div `565db46a` and the workarounds above:

- The chart layer: scales, nice ticks, layout, curves, stacking, pie
  geometry, LTTB, with unit tests against numbers taken from Recharts.
- Line, area, bar (stacked, grouped), scatter, pie/donut and sparkline,
  drawn through `RasterSurface` (`div-gap: 1, 4`).
- Axes labels, legend, tooltip, crosshair, brush rectangle and focus ring as
  Div elements; hover, keyboard navigation (`accessibilityLayer` model),
  pointer-capture brushing, wheel zoom and drag pan.
- Hidden data table fallback and a live-region announcement.
- Export to PNG through an offscreen `Document`.
- The fidelity harness and geometry comparison for bar and line charts.

Not now: frame-synced animation (capability 3), sustained streaming
(capability 4 or 1), responsive resizing (capability 7), exact line pixels
(capability 2).
