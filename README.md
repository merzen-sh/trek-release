# trek-release

Package FiveM resources for release. Walks the input directory and packs files matching patterns in `.trek-pack` into a `.zip` archive.

## CLI

```bash
trek-release pack --input ./my-resource --name my-resource --version 1.0.0 --output ./dist
```

Creates `./dist/my-resource-1.0.0.zip` containing files matched by `.trek-pack` patterns.

### Options

| Flag | Description |
|---|---|
| `-i, --input` | Input resource directory |
| `-n, --name` | Package name (used in output filename) |
| `-v, --version` | Package version (used in output filename) |
| `-o, --output` | Output directory (default: `.`) |
| `-s, --summary` | Print a markdown summary after packing |
| `--dry-run` | Print files that would be packed without creating the zip |

## `.trek-pack` file

Place a `.trek-pack` file in the resource root to control which files are included. Uses glob patterns — one per line, blank lines and `#` comments are ignored. If no `.trek-pack` exists, all files are included.

```
fxmanifest.lua
client/**/*.lua
server/**/*.lua
config.json
!config.secret.json
```

Lines starting with `!` exclude matching files (takes precedence over include patterns).

