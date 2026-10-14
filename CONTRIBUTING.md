# Contributing

Thanks for helping! Bug reports, accuracy fixes and improvements are all
welcome.

## Before you start

- Never commit ROM images or other copyrighted game data. Tests use
  hand-assembled programs instead.
- For accuracy changes, cite your source: a schematic, MAME's driver, a
  datasheet or a measurement of real hardware.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- --deny warnings
cargo fmt --all
```

For the browser frontend, see the README for the `wasm-bindgen` CLI, then:

```sh
cd web
npm ci
npm run dev     # local server with the WebAssembly module rebuilt
npm run check   # type-check, lint and check formatting
npm test
npm run format
```

The CPU diagnostics are slow without optimizations, so run them in
release mode after fetching them:

```sh
scripts/fetch-cpu-tests.sh
cargo test --release -p i8080 --test cpm -- --ignored
```

## Commits

Commits follow [Conventional Commits](https://www.conventionalcommits.org):
`type(scope): summary`, for instance `fix(i8080): set the auxiliary carry
of ANA`. Common types are `feat`, `fix`, `perf`, `refactor`, `test`,
`docs`, `build` and `ci`; scopes are the crate or area touched (`i8080`,
`mw8080bw`, `desktop`, `web`). Keep each commit to one logical change,
with passing tests.
