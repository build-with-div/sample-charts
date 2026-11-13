# sample-charts

Charts on [Div](https://github.com/build-with-div/div), a native Rust UI
framework, by porting the npm chart packages web apps use (Recharts, which
shadcn/ui's charts wrap, first; then Chart.js) and recording the web
platform features Div must provide for them.

- [crates/recharts](crates/recharts): the Recharts 3.8.0 port. So far: the
  numeric axis tick algorithm and the decimal.js-light arithmetic under it,
  tested for equality with the npm package.
- [src](src): the app. Its window shows what the port computes; charts
  appear once Div renders inline SVG.
- [tools/reference](tools/reference): the pinned npm packages, run under
  Node to generate the fixtures the port is tested against
  (`npm ci --ignore-scripts && node gen_fixtures.mjs`).
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
