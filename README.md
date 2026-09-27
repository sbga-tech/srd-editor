# SRD Editor

An offline desktop editor and previewer for SEGA Surfride `.srd` files.

SRD Editor loads SRD scenes, reconstructs their runtime composition, and previews them with a native WebGPU renderer. It is designed for preservation-first editing: unsupported or unproven data is shown as read-only instead of being guessed or silently rewritten.

> **Status:** active development. The editor is useful for inspecting and previewing existing SRD files, but it is not yet a general-purpose SRD authoring tool.

## Features

- Open an SRD file or start with an empty project.
- Inspect scenes, layers, CAST nodes, references, animations, fonts, and textures.
- Preview 2D and 3D transforms, image and slice casts, number casts, text casts, references, animation sets, and supported Fennel/RFZ fonts.
- Use a hierarchy tree, composition viewport, transform and CAST inspectors, and dedicated Design and Animate workspaces.
- Select, lock, hide, and highlight layers or CAST nodes while investigating a scene.
- Edit supported transforms, names, text, active/visible states, animation data, and proven image, slice, number, and reference properties.
- Save or Save As while preserving unknown fields, unknown flag bits, and unrelated source bytes.
- Run without the original game, D3D9, D3DX, DXVK, Cg, or a runtime shader translator.

## Preservation-first editing

SRD Editor writes from the original file as its baseline. It patches only existing records and fields whose encoding and runtime behavior are understood.

Supported edits currently include:

- Existing layer and CAST names.
- Position, rotation, and scale in existing TRS2/TRS3 records.
- Layer active state, CAST active state, and supported visibility controls.
- Existing text content and supported image, slice, number, and reference payload properties.
- Animation sets, animation assignments, animation names, durations, loop state, tracks, and keys where the source format supports the requested change.

The editor does not currently add, remove, or reorder existing layers or CAST records. Unsupported combinations remain visible as preserved data and are rejected rather than normalized into a potentially incorrect value. Semantic flag controls update only their owned bits; unknown bits are retained.

## Preview requirements

The preview uses the asset layout surrounding the SRD file. For a complete preview, keep the original game data directory intact, including referenced DDS textures and RFZ font files under `A000/font`.

The editor locates the nearest ancestor directory named `data` and resolves resources from there. For example:

```text
data/
├── A000/
│   └── font/
├── surfboard/
│   └── ...
└── ...
```

If a required resource or a not-yet-supported shader/provider state is missing, the UI reports the preview error instead of falling back to CPU rendering, D3D9, or a software adapter.

## Quick start

### Requirements

- Rust stable toolchain.
- A macOS, Windows, or Linux system with a working native WebGPU backend.
- The original `data` directory when previewing external textures or RFZ fonts.

### Run the editor

Clone the repository, then run:

```text
cargo run --release
cargo run --release -- /path/to/data/surfboard/example.srd
```

With no path, the editor opens an empty project. With an SRD path, it loads that document and uses its surrounding `data` directory for preview resources.

### Inspect an SRD from the command line

The `srd-inspect` utility prints the parsed project, scenes, layers, nodes, animations, and optional draw information:

```text
cargo run --bin srd-inspect -- /path/to/file.srd
cargo run --bin srd-inspect -- /path/to/file.srd --initial-draws=1920x1080
cargo run --bin srd-inspect -- /path/to/file.srd --animation-set-draws=0:24@1920x1080
```

## Development

Run the local checks with:

```text
cargo check --all-targets
cargo test
cargo fmt --all --check
```

Tests that require a complete game-data corpus use `GAME_DATA_CORPUS` and skip cleanly when it is not set:

```text
GAME_DATA_CORPUS=/path/to/data cargo test
```

To capture the stabilized editor framebuffer for a layout or visual smoke test:

```text
SRD_EDITOR_SCREENSHOT=/path/to/output.png cargo run --release -- /path/to/file.srd
```

## Technical documentation

- [Runtime architecture](docs/runtime-architecture.md) — parsed resources, runtime state, ownership, and composition flow.
- [SRD format notes](docs/srd-format.md) — format behavior, supported fields, and preservation rules.
- [Open limitations](docs/TODO.md) — boundaries that are intentionally not guessed or implemented.
- [Scene transform evidence](docs/evidence/scene-transform.md) — scene hierarchy, transforms, and active-state behavior.

## Architecture overview

The application keeps UI state, document persistence, runtime evaluation, and GPU submission separate:

```text
main.rs
  -> editor/                 iced UI, workspaces, actions, history
       -> document.rs        SRD loading and constrained source writeback
       -> renderer/preview/  runtime evaluation and composition planning
            -> renderer/pipeline/  semantic geometry and render state
                 -> renderer/wgpu/ native WebGPU resources and submission
```

The renderer exposes semantic draw state to WebGPU rather than game-specific packets, D3D9 registers, or reverse-engineered runtime pointers. The editor and preview each own their GPU resources, and the UI does not hold renderer handles.
