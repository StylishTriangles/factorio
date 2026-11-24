# Factorio Science Planner

[Open the calculator](https://stylishtriangles.github.io/factorio/)

Choose a science pack, set its output per minute or per second, and expand individual recipes in the dependency graph. Drag or scroll the graph in either direction to follow branches. **Center view** returns to the starting position and keeps the current branches open. The material table adds up all inputs at the ends of the open branches. If an ingredient is still collapsed anywhere, its remaining rate appears in the table.

The planner covers the seven base-game packs and all twelve Space Age packs. Choose **Space science pack (base game)** for the satellite launch recipe. All calculations run in the browser in Rust compiled to WebAssembly.

## Calculation model

- Normal-quality machines, modules, and beacons. Standard assembly uses Assembling machine 3. Recipes that require other machine types use those machines' normal crafting speed, native slots, and built-in productivity.
- Each eligible craft fills the machine's internal slots with the selected productivity tier. Four tier 3 modules give an Assembling machine 3 +40% output and −60% speed before beacon effects. A machine that cannot use the chosen productivity modules fills its available slots with the selected speed tier.
- For `N` normal beacons with two speed modules each, the bonus to crafting speed is `N × profile[N − 1] × 1.5 × 2 × module speed bonus`. The game's exact 2.1.25 transmission table is used. With tier 3 modules, 4 and 8 beacons add 300.00% and 424.20% speed respectively. Zero beacons give zero transmitted bonus. Custom counts use the same table, and feasible placement depends on the machine footprint.
- Required crafts per second equal output demand divided by the recipe yield with productivity. Each ingredient rate is its recipe quantity times the craft rate. Speed changes required machine capacity. The calculator uses full double precision, with two decimals shown in the interface.
- The graph follows fixed recipe routes, including one route for each oil fraction. Material rates are gross inputs before credits for co-products or returns shared across recipes. Where relevant, node tooltips describe the assumptions, such as holmium's expected scrap yield and retaining a seed egg for pentapod production. Base-game space science shows buffered rocket-launch capacity and rocket-part production as separate stages.

Recipe and machine data are based on [Factorio 2.1.25 prototypes](https://github.com/wube/factorio-data/tree/2.1.25). See also the [official beacon documentation](https://wiki.factorio.com/Beacon) and [rocket launch throughput](https://wiki.factorio.com/Rocket_silo#Maximum_throughput). Item artwork is served by the [Official Factorio Wiki](https://wiki.factorio.com/Factorio:Copyrights) and belongs to Wube Software. This project is unofficial.

## Run locally

Install Rust, its `wasm32-unknown-unknown` target, and [wasm-pack](https://wasm-bindgen.github.io/wasm-pack/). Use Python 3 for the local server.

```sh
rustup target add wasm32-unknown-unknown
make test
make serve
```

Open [http://127.0.0.1:8000/](http://127.0.0.1:8000/). GitHub Actions builds and publishes the static output from `main` to GitHub Pages.
