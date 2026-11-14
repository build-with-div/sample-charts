# sample-charts

Charts on [Div](https://github.com/build-with-div/div), a native Rust UI
framework, by porting the npm chart packages web apps use (Recharts, which
shadcn/ui's charts wrap, first; then Chart.js) and recording the web
platform features Div must provide for them.

- [crates/recharts](crates/recharts): the Recharts 3.8.0 port. So far: the
  numeric axis tick algorithm with the decimal.js-light arithmetic under it,
  and the cartesian axis selectors (domain, nice ticks, range, scale and
  ticks of every axis), tested for equality with the npm package.
- [crates/d3](crates/d3): the d3 modules Recharts' browser build uses:
  d3-array's ticks and d3-scale's linear, band and point scales.
- [crates/js](crates/js): the ECMAScript semantics the ports depend on as
  V8 implements them: number parsing and printing, `Math.log10`,
  `Math.pow`, values and `Map` keys.
- [src](src): the app. Its window shows what the port computes; charts
  appear once Div renders inline SVG.
- [tools/reference](tools/reference): the pinned npm packages, run under
  Node 24 to generate the fixtures the ports are tested against
  (`npm ci --ignore-scripts && npm run fixtures`).
- [docs/framework-gaps.md](docs/framework-gaps.md): the platform features
  the port has hit that Div lacks.
- [docs/design.md](docs/design.md): packages → platform features, the Div
  work list, the fidelity plan and the milestones.
- [docs/rfc-inline-svg.md](docs/rfc-inline-svg.md): RFC draft for inline
  SVG, `ResizeObserver` and `requestAnimationFrame` in Div.
- [docs/probes](docs/probes): the phase 1 probes (pinned to Div `565db46a`,
  built separately).

Div is pinned to `92ea25a0`.

```bash
cargo run
```

```bash
cargo test --workspace
```

## License

Apache-2.0.
