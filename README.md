# YPBank Parser & CLI Utilities
Project for parcing, converting and comparing transactions presented in csv, text and bin format. See [specs](./docs/format/specs) for the formats specifications and [examples](./docs/format/examples) for the examples.

## Project structure
- `parser/` – Parser lib, supports csv, text and bin format. Provides a common Transaction struct.
- `converter/` – CLI to convert transactions from one format to another.
- `comparer/` – CLI to compare two files containing transactions.

## Build
Build binaries (will appear in `target/release/`)
```bash
cargo build --release
```

Generate and open docs
```bash
cargo doc -p parser --no-deps --open
```

## Examples
```bash
comparer --file1 "docs\format\examples\records_example.csv" --format1 csv --file2 "docs\format\examples\records_example.txt" --format2 text
```

```bash
converter --input "docs\format\examples\records_example.csv" --input-format csv --output-format text
```
