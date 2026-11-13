# sample-charts

Desktop charts built on [Div](https://github.com/build-with-div/div), a
native Rust UI framework: line, area, bar, scatter, pie and donut, and
sparklines in the style of shadcn/ui charts, with hover, brushing, zoom,
streaming and large datasets.

This is a sample app that ports chart packages onto Div and records the
web platform features Div must provide for them. It is in
its investigation phase: there is no app yet.

- [docs/design.md](docs/design.md): the chart packages to port (Recharts
  for shadcn charts, visx/d3, Chart.js, ECharts), the web platform features
  they run on, what Div has and lacks (the work list), the port and the
  fidelity plan.
- [docs/rfc-inline-svg.md](docs/rfc-inline-svg.md): RFC draft for inline SVG,
  `ResizeObserver` and `requestAnimationFrame` in Div.
- [docs/probes/](docs/probes/): the probes behind those measurements.

Div is pinned to `565db46a`.

## License

Apache-2.0.
