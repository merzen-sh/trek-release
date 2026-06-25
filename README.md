# trek-release

Package FiveM resources for release. Validates `fxmanifest.lua` references, packs only declared files into a `.zip` archive.

## CLI

```bash
trek-release pack --input ./my-resource --name my-resource --version 1-0-0 --output ./dist
```

Creates `./dist/my-resource-1-0-0.zip` containing only files referenced in `fxmanifest.lua`.

### Options

| Flag | Description |
|---|---|
| `-i, --input` | Input resource directory |
| `-n, --name` | Package name (used in output filename) |
| `-v, --version` | Package version (used in output filename) |
| `-o, --output` | Output directory (default: `.`) |

## GitHub Action

```yaml
- uses: merzen-sh/trek-release@v1
  with:
    input: ./trek-core
    name: trek-core
    version: 1-0-0
```

Uploads `trek-core-1-0-0.zip` as a workflow artifact.

### Action inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `input` | yes | — | Input resource directory |
| `name` | yes | — | Package name |
| `version` | yes | — | Package version |
| `output` | no | `.` | Output directory |
| `cli-version` | no | `latest` | CLI version to download |

## Requirements

- FiveM resource with valid `fxmanifest.lua`
- All files referenced in the manifest must exist on disk
