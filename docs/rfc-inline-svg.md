# RFC draft: inline SVG, faithful to Chromium

Status: draft from the charts sample (build-with-div/sample-charts),
13 November 2026, against Div `565db46a`. For the engine and framework
tracks to accept, change or split. Nothing here edits Div.

Aligned with the platform design decisions accepted on 13 November 2026
(`platform-plan/decisions-2026-11-13.md`): inline SVG painted Chromium's
way starts right after the engine's Tier 0 items, and the scheduled
stroked-icons item is its stage 1 rather than a second painter (decision
3); `view!` uses SVG's real attribute names (decision 3); shadcn's
`ChartStyle` sheet goes through the browser-compatible ingestion mode and
runtime custom-property writes are supported (decision 2); the shapes of
`ResizeObserver` and `requestAnimationFrame` follow decision 4.

## Summary

Implement inline SVG in Div as the web platform defines it and as pinned
Chromium 147 renders it: the SVG element tree in the document, styled by
CSS, laid out in the SVG coordinate system, painted with Chromium's SVG
paint setup, hit-tested on geometry, focusable and exposed to accessibility,
with the SVG geometry DOM methods. Plus two framework features the same
packages need everywhere: `ResizeObserver` and `requestAnimationFrame`.

## Motivation

The chart packages professionals use on the web render to inline SVG
(Recharts, which shadcn/ui charts wrap; visx; d3) or to Canvas 2D
(Chart.js, ECharts). Porting them to Div means providing that platform.
Measured on Div `565db46a` (sample-charts probes 1, 6, 7):

- `Element::create` rejects every SVG child element (`g`, `path`, `line`,
  `rect`, `circle`, `text`, `defs`, `clipPath`, `linearGradient`, …) with
  `UnknownTag`; `<svg>` is an empty replaced box that ignores
  `width`/`height`.
- SVG as an image resource (`image/svg+xml`) goes through a separate static
  rasterizer that reads filled and stroked `rect` and filled axis-aligned
  `path` only; stroked `path`, `polyline` and `line` paint nothing. The same
  gap blocks stroked icon sets (Lucide), reported by sample-git.
- Painting the same geometry with default Skia settings is close to
  Chromium's SVG output but not equal (up to 1,443 px differ by 1/255 at
  scale 1; up to 24/255 at scale 2 on rounded bars), so faithful SVG must go
  through a Chromium-matched paint setup, as Div's CSS painting already does.

All 70 shadcn chart examples (`shadcn@4.23.0`, Recharts 3.8.0) need the SVG
root, `g` and `path`; 60 need SVG text; 44 `line`; 34 arcs; 28 `circle`; 69
use CSS custom properties in SVG paint. The full ranking is in
sample-charts `docs/design.md`.

## Design

### DOM

- SVG elements are elements in Div's tree with the SVG namespace, created
  from `view!` and `Element::create` with their case-sensitive names
  (`clipPath`, `linearGradient`, `foreignObject`); `view!` spells
  attributes with their real names (`stroke-width`, `viewBox`,
  `xlink:href`), as decided. Today `tag_definition`
  lowercases names; SVG names must keep their case.
- Attributes keep web semantics and names (`d`, `viewBox`, `stroke-width`,
  `text-anchor`, `transform`, `clip-path`, `href`/`xlink:href`).
  Presentation attributes map to CSS declarations at the presentation-hint
  level, so author CSS (classes, `var()`) overrides them as in Chromium.
- Rust-typed helpers sit on top and produce the same attributes: a
  `PathData` builder that formats `d`, typed `Transform`, typed paint
  values. Web semantics underneath; strings stay accepted.
- `id` references (`url(#id)`, `href="#id"`) resolve within the document,
  with Chromium's behaviour for missing and late-added targets.

### Style

- SVG properties as CSS properties with their inheritance and initial
  values: `fill`, `fill-opacity`, `fill-rule`, `stroke`, `stroke-width`,
  `stroke-linecap`, `stroke-linejoin`, `stroke-miterlimit`,
  `stroke-dasharray`, `stroke-dashoffset`, `stroke-opacity`, `opacity`,
  `paint-order`, `vector-effect`, `shape-rendering`, `text-anchor`,
  `dominant-baseline`, `clip-path`, `clip-rule`, `pointer-events`,
  `visibility`, `display`, `transform`, `transform-origin`, `transform-box`.
- Depends on the stylesheet cascade and selectors (in progress) and on CSS
  custom properties (missing): shadcn sets colours as
  `fill="var(--color-desktop)"` with the variables in a `<style>` element
  (`ChartStyle`), a third-party sheet read through the browser-compatible
  ingestion mode, which drops what Chromium drops and reports it.

### Layout

- `<svg>` in CSS layout is a replaced element with intrinsic size from
  `width`/`height` and `viewBox` ratio (300×150 default), as in Chromium.
- Inside, SVG content is laid out in user units: `viewBox` and
  `preserveAspectRatio`, nested `svg`, `transform` on any element,
  percentage lengths against the viewport, `foreignObject` (exists) hosting
  CSS boxes.

### Text

- `text` and `tspan` with `x`, `y`, `dx`, `dy` (lists and `em` units),
  `text-anchor`, `dominant-baseline`, `alignment-baseline`, `textLength` later;
  shaped by Div's text stack with fonts from CSS; painted as Chromium paints
  SVG text (geometric positioning, its anti-aliasing rules).

### Paint

- Shapes: `path` (full grammar: `M L H V C S Q T A Z`, relative forms),
  `line`, `rect` (`rx`/`ry`), `circle`, `ellipse`, `polygon`, `polyline`.
- Painted through a painter modelled on Chromium's `SVGShapePainter`:
  fill, then stroke (or `paint-order`), stroke geometry with Chromium's
  dash, cap, join and miter rules, opacity groups, the same anti-aliasing
  flags. Paint servers: `linearGradient`, `radialGradient` (with
  `gradientUnits`, `gradientTransform`, `spreadMethod`, `href` inheritance),
  `pattern`. `clipPath` (`clipPathUnits`), later `mask` and `marker`.
- The static SVG image rasterizer (`div-paint/src/image_resource.rs`)
  should parse into the same tree and use the same painter, so `<img
  src=icon.svg>` and inline SVG render identically.

### Events and hit testing

- Hit testing on painted geometry: fill area by `fill-rule`, stroke area by
  stroke width, governed by `pointer-events` (`visiblePainted` default),
  as Chromium does. Events target SVG elements and bubble through `g` and
  `svg` into HTML ancestors.

### Focus and accessibility

- `tabindex` on SVG elements, `focus()`, `:focus-visible` and `outline` as
  in Chromium; `role`, `aria-*`, `<title>`/`<desc>` names; graphics roles
  (`graphics-document`, `graphics-symbol`) in the accessibility tree.

### Geometry methods

On SVG element handles, typed: `bbox()` (`getBBox`), `total_length()`
(`getTotalLength`), `point_at_length()` (`getPointAtLength`),
`computed_text_length()` (`getComputedTextLength`), `screen_ctm()`
(`getScreenCTM`), with the web method names in the docs.

### Animation

- CSS transitions and animations apply to SVG properties
  (`stroke-dasharray`, `stroke-dashoffset`, `opacity`, `transform`, `fill`),
  through Div's existing animation engine.

## Two framework features the packages need

- **`ResizeObserver`**: one API observing an element's content, border or
  device-pixel-content box; the engine computes observations after layout
  and the framework delivers them before paint, with Chromium's ordering and
  loop-limit rule; typed entries. Every Recharts chart (`ResponsiveContainer`) and Chart.js
  chart uses it.
- **`requestAnimationFrame`**: per document, returning a typed handle, in
  every window; callbacks run once per frame before style and layout with
  the frame timestamp, as in Chromium; headless tests drive
  them with `Document::advance_time`/`begin_frame`. Recharts animates every
  series on mount with it; Chart.js and ECharts animate with it.

## Conformance

- WPT, through Div's runner: `svg/shapes`, `svg/path`, `svg/painting`,
  `svg/pservers`, `svg/text`, `svg/coordinate-systems`, `svg/styling`,
  `svg/types` (JS cases ported), `css/css-masking/clip-path`,
  `css/css-variables`, `resize-observer` (JS cases ported). Report pass
  counts per directory; claim nothing for unmeasured directories.
- Pixel harness against pinned Chromium 147 at the census profiles: the
  sample-charts Recharts pages (per example, light and dark) and their DOM
  comparison.

## Stages, owners, sizes

| Stage | Content | Owner | Size |
| --- | --- | --- | --- |
| 1 | Namespace, element names, `svg` sizing and `viewBox`; `g`, `path`, `line`, `rect`, `circle`, `ellipse`, `polygon`, `polyline`; fill and stroke properties; `transform`; Chromium-matched shape painting; static SVG images (stroked icons) through the same tree and painter; `view!` with SVG's attribute names | engine (painting), framework (`view!`) | L |
| 2 | `text`, `tspan`, anchors and baselines | engine | L |
| 3 | SVG properties in the cascade, presentation attributes, `var()` | engine (with the cascade work) | M after the cascade |
| 4 | `defs`, gradients, `pattern`, `clipPath`, `url(#id)` | engine | M |
| 5 | Geometry hit testing, `pointer-events`; focus, `tabindex`, accessibility roles | engine + framework | M |
| 6 | Geometry methods (`getBBox`, `getTotalLength`, …) | framework (engine geometry) | M |
| 7 | Transitions and animations on SVG properties | engine | M |
| — | `ResizeObserver` | engine (observations) + framework (delivery) | M |
| — | `requestAnimationFrame` | framework | M |

Stages 1 and 3 unblock 70 of 70 examples for static shapes; stage 2 brings
the 60 with text.

## Open questions

- Per-element cost: 10,000 absolutely positioned `<div>`s take about 1.9 s
  to build, lay out and paint headless (probe 2). Recharts emits one SVG
  element per dot, tick and grid line; SVG elements should not need CSS box
  layout, but the build cost (style setting) is shared. Chromium's time for
  the same page should be measured before setting a target.
- Whether `foreignObject`'s existing support moves under the new SVG tree
  unchanged.
- Resolved: `view!` uses SVG's real attribute names and case-sensitive
  element names (decision 3).
