# Animation channels and shipped-corpus evidence

This page records the measured animation-channel corpus and the source-level consumers that are currently established. It intentionally does not infer meanings for channels that have no consumer. Corpus numbers are reproducible with the new binary target:

```text
cargo run --quiet --manifest-path srd-editor/Cargo.toml --bin srd-anim-stats
```

The binary is `srd-editor/src/bin/srd-anim-stats.rs`. It discovers `sample/data` recursively, loads each `.srd` with `EditorDocument::load`, sorts paths and report values, and continues after load failures. The run used for this page discovered and loaded 91/91 files, with 263 scenes, 692 layers, 3811 ANIM records, and 150327 tracks.

## Runtime dispatch boundary

The parsed `Track.target` is read at `srd-editor/src/animation.rs:94-98`, and the parsed `KeyData` variants are selected at `srd-editor/src/animation.rs:107-132`. Runtime animation applies common channels first (`srd-editor/src/animation.rs:244-265` and `srd-editor/src/animation.rs:284-293`) and then CAST-specific image/reference channels (`srd-editor/src/reference_runtime.rs:550-575`). The table below uses the shipped-corpus counts printed by sections 14, 19, and 20 of `srd-anim-stats`.

## Complete channel table

`UNNAMED` in the editor-name column means that `animation_target_name` falls through to `Channel N` at `srd-editor/src/editor/model.rs:1546-1563`; it does not mean that the runtime meaning is unknown. `KeyData` entries include the measured count for that channel. `Unsupported` is an enum variant but did not occur in any channel below.

| id | editor name | established meaning | corpus tracks | KeyData variants (count) | observed `Track.format` values (count) | status | exact consumer citation |
| ---: | --- | --- | ---: | --- | --- | --- | --- |
| 0 | Position · X | Position X | 11043 | Key20F32 (11043) | `0x13` (9876), `0x113` (1167) | IMPLEMENTED | `srd-editor/src/transform.rs:55` — `0..=2` writes `translation[target]` |
| 1 | Position · Y | Position Y | 10704 | Key20F32 (10704) | `0x13` (9514), `0x113` (1190) | IMPLEMENTED | `srd-editor/src/transform.rs:55` — `0..=2` writes `translation[target]` |
| 2 | Position · Z | Position Z | 1586 | Key20F32 (1586) | `0x13` (821), `0x113` (765) | IMPLEMENTED | `srd-editor/src/transform.rs:55` — `0..=2` writes `translation[target]` |
| 3 | Rotation · X | Rotation X | 286 | Key20I32 (286) | `0x43` (188), `0x143` (98) | IMPLEMENTED | `srd-editor/src/transform.rs:56` — `3..=5` writes `rotation[target - 3]` |
| 4 | Rotation · Y | Rotation Y | 664 | Key20I32 (664) | `0x43` (519), `0x143` (145) | IMPLEMENTED | `srd-editor/src/transform.rs:56` — `3..=5` writes `rotation[target - 3]` |
| 5 | Rotation · Z | Rotation Z | 11838 | Key20I32 (11838) | `0x43` (10276), `0x143` (1562) | IMPLEMENTED | `srd-editor/src/transform.rs:56` — `3..=5` writes `rotation[target - 3]` |
| 6 | Scale · X | Scale X | 16646 | Key20F32 (16646) | `0x13` (13299), `0x113` (3347) | IMPLEMENTED | `srd-editor/src/transform.rs:57` — `6..=8` writes `scale[target - 6]` |
| 7 | Scale · Y | Scale Y | 16349 | Key20F32 (16349) | `0x13` (13029), `0x113` (3320) | IMPLEMENTED | `srd-editor/src/transform.rs:57` — `6..=8` writes `scale[target - 6]` |
| 8 | Scale · Z | Scale Z | 238 | Key20F32 (238) | `0x13` (155), `0x113` (83) | IMPLEMENTED | `srd-editor/src/transform.rs:57` — `6..=8` writes `scale[target - 6]` |
| 9 | Multiply color | Multiplicative color RGB | 4855 | Key8Bytes4 (4855) | `0x51` (4490), `0x151` (365) | IMPLEMENTED | `srd-editor/src/transform.rs:62-75` — `9 | 19` writes color bytes |
| 10 | Visibility | Visibility word | 31369 | Key20I32 (31369) | `0x23` (31252), `0x123` (117) | IMPLEMENTED | `srd-editor/src/transform.rs:58` — `10` writes `visibility_word` |
| 11 | UNNAMED | SrImage geometry width (`size[0]`) | 461 | Key20F32 (461) | `0x13` (453), `0x113` (8) | IMPLEMENTED | `srd-editor/src/image.rs:320-332` maps `11` to component 0; dispatch `:582-584` |
| 12 | UNNAMED | SrImage geometry height (`size[1]`) | 1447 | Key20F32 (1447) | `0x13` (1443), `0x113` (4) | IMPLEMENTED | `srd-editor/src/image.rs:320-332` maps `12` to component 1; dispatch `:582-584` |
| 13 | UNNAMED | CREF packed vertex color, vertex 0 | 1293 | Key8Bytes4 (1293) | `0x51` (967), `0x151` (326) | IMPLEMENTED | `srd-editor/src/image.rs:558-572` maps `13` to vertex 0; dispatch `:582-586` |
| 14 | UNNAMED | CREF packed vertex color, vertex 2 | 1301 | Key8Bytes4 (1301) | `0x51` (975), `0x151` (326) | IMPLEMENTED | `srd-editor/src/image.rs:558-572` maps `14` to vertex 2; dispatch `:582-586` |
| 15 | UNNAMED | CREF packed vertex color, vertex 1 | 1157 | Key8Bytes4 (1157) | `0x51` (911), `0x151` (246) | IMPLEMENTED | `srd-editor/src/image.rs:558-572` maps `15` to vertex 1; dispatch `:582-586` |
| 16 | UNNAMED | CREF packed vertex color, vertex 3 | 1247 | Key8Bytes4 (1247) | `0x51` (1001), `0x151` (246) | IMPLEMENTED | `srd-editor/src/image.rs:558-572` maps `16` to vertex 3; dispatch `:582-586` |
| 17 | UNNAMED | CREF reference selector / explicit image and rectangle | 11013 | Key20I32 (11013) | `0x23` (7033), `0x123` (3980) | IMPLEMENTED | `srd-editor/src/image.rs:589-595` dispatches to `ImageReferenceChannel::Cref` |
| 18 | UNNAMED | Unestablished; no runtime state consumer found | 0 | none | none | PARSED-BUT-IGNORED | No consumer found in the searched source/docs. Parser: `srd-editor/src/animation.rs:94-132`; fallbacks: `srd-editor/src/transform.rs:88-89`, `srd-editor/src/image.rs:602-603`; the default is also documented at `srd-editor/docs/evidence/image-coordinate-animation.md:22` |
| 19 | Additive color | Additive color RGB | 5023 | Key8Bytes4 (5023) | `0x51` (4902), `0x151` (121) | IMPLEMENTED | `srd-editor/src/transform.rs:62-75` — `9 | 19` writes color bytes |
| 20 | UNNAMED | CRE1 reference selector / explicit image and rectangle | 833 | Key20I32 (833) | `0x23` (262), `0x123` (571) | IMPLEMENTED | `srd-editor/src/image.rs:596-602` dispatches to `ImageReferenceChannel::Cre1` |
| 21 | Opacity | Multiplicative opacity (color alpha) | 20050 | Key20F32 (20050) | `0x13` (17626), `0x113` (2424) | IMPLEMENTED | `srd-editor/src/transform.rs:76-85` — `target == 21` writes `multiply_color[3]` |
| 22 | Additive opacity | Additive opacity (color alpha) | 806 | Key20F32 (806) | `0x13` (806) | IMPLEMENTED | `srd-editor/src/transform.rs:76-85` — the other branch writes `additive_color[3]` |
| 23 | UNNAMED | Reference animation frame request (CRFD / RefCast) | 118 | Key20F32 (118) | `0x13` (118) | IMPLEMENTED | `srd-editor/src/reference.rs:72-95` gates `track.target == 23`; requests are consumed at `srd-editor/src/reference_runtime.rs:606-615` |

The public editor-name mapping is intentionally narrower than the runtime dispatch. The runtime source establishes the meanings for the unnamed CAST-specific IDs above. The source also establishes that channel 23 is a RefCast-specific request: `srd-editor/src/reference.rs:72-95` builds a `ReferenceAnimationRequest`, and `srd-editor/src/reference_runtime.rs:606-615` forwards it to the child animation. The documented virtual-table boundary says the channel is a no-op for non-RefCast types and is implemented only by SrRefCast (`srd-editor/docs/evidence/crfd-reference-cast.md:34-49`).

## Track.format bit observation

The 150327-track histogram is exactly the table above. Every channel with at least two observed format values uses exactly two values whose numeric difference is `0x100`:

| format family | channels | per-channel observations |
| --- | --- | --- |
| `0x13` / `0x113` | 0, 1, 2, 6, 7, 8, 11, 12, 21 | counts are listed in the complete table above |
| `0x43` / `0x143` | 3, 4, 5 | counts are listed in the complete table above |
| `0x23` / `0x123` | 10, 17, 20 | counts are listed in the complete table above |
| `0x51` / `0x151` | 9, 13, 14, 15, 16, 19 | counts are listed in the complete table above |

Two measured exceptions prevent a literal claim that every numeric channel id has two observed values: channel 18 has zero tracks, while channel 22 has only `0x13` (806) and channel 23 has only `0x13` (118). No other format values occur. Thus the exact corpus claim is: every channel with two observed values has the `0x100` pair shown above; channels 18, 22, and 23 have fewer than two observed values.

`srd-editor/src/animation.rs:427-445` establishes the time behavior for `format & 0x300`: when both bits are clear, `wrap_time` returns the input frame; when either `0x100` or `0x200` is set, it wraps the frame into `[start, end)`. The corpus only observes the `0x100` delta shown above, so the source establishes that these high-format values select the wrapping path, but does not establish an independent meaning distinguishing bit `0x100` from bit `0x200`.

## MOT target padding convention

The follow-up section of `srd-anim-stats` measured the following across all parsed `Motion` values:

- `Motion.target < 0`: **9931**; the exact negative-value histogram is `target == -1 -> 9931`.
- `Motion.target >= layer.nodes.len()`: **0**.
- Out-of-range motions with `tracks.len() == 0`: **9931**.
- Out-of-range motions with at least one track: **0**; therefore the required dangling-track list is empty.
- Zero-track motions with a valid in-range target: **0**.

This is a measured padding convention in this corpus. It is not a claim about other corpora. The existing runtime guard already skips negative targets: `srd-editor/src/animation.rs:251-254` contains `if motion.target < 0 { continue; }`. The subsequent conversion and bounds check are at `srd-editor/src/animation.rs:255-263`.

## ANMS and ANIM structural invariants

These values are measured by sections 1–10 of `srd-anim-stats` for the same 91-file run:

- **939/939 ANMS sets** have `slots.len() == owning_scene.layers.len()`; the less/equal/greater relation is `0 / 939 / 0`.
- All 939 sets have `start_frame == 0`.
- There are **0 dangling non-empty ANMS slot names** (`layer.find_animation` resolved every named slot).
- There are **0 duplicate ANMS-name groups** and **0 empty ANMS names**.
- There are **0 duplicate ANIM-name groups** and **0 empty ANIM names**.
- Of 3811 ANIM records, **573** are resolved by at least one ANMS slot and **3238** are never referenced by an ANMS in their scene.
- The run measured 4122 ANMS slots: 1805 enabled and 2317 disabled; 2337 have an empty `animation_name` (1058 enabled+empty, 1279 disabled+empty).

All statements above are either direct output fields from `srd-editor/src/bin/srd-anim-stats.rs` or source citations shown inline. No meaning is assigned to channel 18: it is unattested in this corpus and unconsumed by the searched runtime code.
