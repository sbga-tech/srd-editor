# SRD runtime shader-state audit boundary

> **历史审计记录。** 本页数字来自已删除的 compact-key/D3D9 compatibility implementation。审计结果继续作为 game corpus 证据；工具、runtime key table 与 D3D9 backend 不再属于当前编辑器。

该审计曾区分三个不能混淆的事实：

1. 当时已证明的 ShapeEnv 输入所选择的精确 compact key；
2. 该 key 是否属于游戏的 82-key `SimpleShaderVSSimpleShaderPS` collection；
3. 对应 bytecode 当时是否已经封装进兼容后端。

The deleted `src/bin/srd-runtime-state-audit.rs` scanned all 91 files under the
local `surfboard` corpus. Its default mode built a conservative upper bound for
Image/Slice/Number texture-presence changes: an unanimated CREF/CRE1 channel
kept its exact initial presence, while an animated channel could be absent or
use any valid texture referenced by that channel. It also included both the
authored layer dimension and every independently copied RefCast dimension
recorded by `ProjectRuntime`.

The complete-corpus run reported:

```text
files=91
potential_upper_bound_shader_keys=82
potential_upper_bound_unpacked_keys=51
potential_upper_bound_keys_outside_game_collection=51
```

The equality of the last two counts is the useful result: every key in this
conservative upper bound that is present in the original 82-key collection now
has an embedded VS/PS pair. The 51 remaining keys are all outside the original
collection; the audit does not claim that any of them reaches a shipped draw.
They include deliberately broad CREF/CRE1 combinations and authored states
that may be disabled, invisible, or replaced by another scene/pass context.
They remain diagnostics, not permission to compile or substitute new shaders.

The existing exact initial-state corpus test remains narrower and reports
19,484 image bases, 65 direct ShapeEnv keys, six keys represented only by a
position-2 sibling, and 45 direct keys outside the collection. Those numbers
also do not establish runtime reachability because that test intentionally
ignores ANMS layer gates and world visibility.

The non-text audit also covers 18,192 Image/Slice/Number node/dimension
contexts (the difference is the 1,292 TextCast definitions). Their exact
initial packet states contain 216 alpha-test contexts across six keys and zero
stencil contexts. Nineteen alpha-test contexts use broad direct keys outside
the original collection; the other 197 already have exact packaged collection
shaders. SrImage alpha/stencil fields are not animation targets, so CREF/CRE1
animation does not turn the zero authored stencil result into a stencil path.

The same tool now audits special CAST matrix coverage before any runtime host
scan. All 1,226 flagged nodes use matrix kind `0x10000`; 134 also carry the
independent `0x01000000` modifier, all in 2D runtime layers. The 101 affected
layers contain 10,369 CASTs. After the executable-derived matrix branch was
implemented, the complete initial runtime emits 21,136 Image/Slice/Number
draws: 5,124 come from those layers and 1,155 directly from flagged nodes.
The exact matrix and camera evidence is recorded in
[`special-cast-matrix.md`](special-cast-matrix.md).

The optional `--integer-frames` mode uses fresh runtime state for each frame.
It reduces repeated tails exactly: non-wrapped tracks are constant after their
last range end, while wrapped integer-frame tracks repeat after the least
common multiple of their selected spans. Geometry-heavy full-corpus execution
is retained as a diagnostic tool, but no incomplete run is recorded as a
result here.

Two concrete hosts have complete integer-cycle results:

```text
AdvertiseLogo / CHU_UI_Advertise_00_v10.srd
  ANMS=19, covered frames=2303, draws=82158
  distinct keys=7, unpackaged=0, outside collection=0
  texture masks: none=898, slot0=62640, slot0+slot1=18620
  stencil=0, alpha-test=0

CommonBackGround / CHU_UI_Common_BK_00_v11.srd
  ANMS=10, covered frames=19580, draws=2683732
  distinct keys=6, unpackaged=0, outside collection=0
  texture masks: none=75532, slot0=2470238, slot0+slot1=137962
  stencil=0
  alpha-test draws=13403, exact key AAEBABBAABCBAAAAAA
```

For Common `ANMS[1] blue_in`, frame `1`, the alpha-test path is exactly thirteen
Image draws: `LAYR[5]/NODE[10..17]` followed by
`LAYR[1]/NODE[35,36,52,53,58]`. The second group belongs to a 3D layer whose
`NODE[2]` uses matrix kind `0x10000`; the old whole-layer rejection hid these
five normal Image nodes. A corpus regression fixes the complete count, node
sequence and key. The D3D9Ex smoke submits the full Image/Slice stream and then
submits those thirteen alpha-test sources alone; both ResetEx passes succeed.
The alpha-test-only readback changes zero RGB pixels at that frame, which is
recorded rather than treated as a failure: successful packet/shader submission
and final visibility are separate facts, and later/failed-alpha pixels must not
be invented to make a diagnostic nonzero.

Advertise `ANMS[10] AS_title_in` 包含十个 dual-texture Image draw：
`LAYR[3]/NODE[41,44,47,52,54,55,56,57,60,62]`。早期 audit 正确记录了每条 draw 都绑定 TEXL stage 0/1，但错误地把七条 CIMG `0x4C = 4` 解释成 MultiTex0 `0`；实机 `srd-renderer-20260814T235050494Z.trace` 已用 mask draw 的 shader `0x2f6273f0` 和 `mul r0.w, r0, r1.x` 证明它们实际为 MultiTex0 `12`。其余三条原始值 `1` 仍为 MultiTex0 `9`。

修正后，`NODE[47,52,54,55,56,57,62]` 选择 `MultiplyAlphaBySecondaryRed(12)`；`NODE[41,44,60]` 选择 `ReplaceWithSecondary(9)`。frame `30` 的 WebGPU integration regression 同时固定十条 source、双 texture binding 与这组模式分布；frame `39` 的 native framebuffer delta 只覆盖 CHUNITHM/Mate/M/a/t/e/illumination mask 轮廓，不再接受覆盖整个 quad 的旧输出。

当时的统一 runtime draw builder 已不因派生 key 未封装或 packet stencil 启用而直接丢弃 Image、Slice 或 Number draw；它保留 authoritative packet 与 texture binding，再由 audit/backend 在使用点派生 compact key。历史 D3D9Ex backend 仍拒绝未注册 key，merged submission planner 仍拒绝 renderer lifetime 尚未闭合的 stencil sequence state；两条路径都不会静默替换或合并 unsupported state。

当前原生 pipeline 不再派生 compact key。它直接把同一批已证明字段编译成 `SrdDrawState`、geometry 与 WGSL material 参数；原生 contract 接受 render preset `0..21` 与 target-color preset `33..61`。后者保留 CATR override、source order 与 `textureTargetColor` snapshot，完整语料实际出现的 `34..58`、`60` 均通过编译；shader ExtParam fixture 另覆盖 54 组真实无纹理/有纹理 material 状态，测试只把未闭环的宿主 transform 换成确定性 gallery。未证明的 `22..32` 与未知 scene/pass host context 仍返回带 CAST 来源的错误，而不是映射到近似 shader variant。

The complete collection packaging and its HAL evidence are documented in
[`render-shader-bytecode.md`](render-shader-bytecode.md). The unresolved work is
the executable-derived scene/pass context that determines which of the broad
direct states can actually reach a submitted draw.
