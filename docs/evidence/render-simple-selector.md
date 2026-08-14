# SRD SimpleShaderSelector 选择、紧凑键与 feature 表

本页记录 SRD/Ceylon ShapeEnv 实际选择的 selector、Simple 键的字节编码、完整 71 项 descriptor，以及已经闭环的 ShapeEnv 模块参数映射。结论来自游戏二进制调用链，并以完整游戏 data 目录中的 shader collection 作独立语料校验。

分析对象：`chusanApp.exe` SHA-256 `28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`；保存后的 IDB SHA-256 `0229B02CB4932EFE4576BE5AE7EC9E46ABA7470B9B430585FB5D858B238EC672`。

## selector 槽位 9

`ceylon_initialize_shader_selectors_and_post_effects` (`0x6217F0`) 使用同一个 `g_sea_shader_manager` 的首个 selector vector：

1. `0x621A60..0x621C54` 循环 9 次构造 Default selector，并在 `0x621C40` 调用 registry insert；
2. `0x621C5A..0x621D78` 构造 Simple selector；
3. `0x621DC2` 把 Simple 对象插入同一个 registry。

`sea_shader_selector_registry_insert` (`0x61D350`) 先检查重复对象和空槽；registry 初始为空且上述对象均为新分配，所以每次都追加到 vector 尾部，返回值是追加前的长度。由此严格得到 Default 槽位 `0..8`、Simple 槽位 `9`。lookup (`0x61F4A0`) 使用 `requested_index % current_count` 取 vector 元素。

`ceylon_create_shape_environment` (`0x670680`) 构造 `sea::PrimitiveDummyShape` 后，在 `0x6706D2..0x6706DF` 执行：

```text
push 1
push 9
mov  ecx, shape
call sea_shape_set_shader_selector
```

`sea_shape_set_shader_selector` (`0x6AFC70`) 用 selector 字节查 registry，并把它写入 `Shape+0xA8`；第二个字节写入 `Shape+0xB0`。cache hit 时 `ceylon_select_shape_environment` 返回此前保存的同一 shape，随后 Ceylon 绘制入口通过该 shape 的 virtual `+0x2C` 执行。因此 SRD ShapeEnv 使用 Simple selector 是实际对象与调用链结论，不是依据 shader 文件名推断。

## 18 字节紧凑键

Simple selector 的 virtual `+0x0C` 是 `sea_simple_shader_selector_build_compact_key` (`0x660120`)；完成 71 位 bitset 后调用 `sea_simple_shader_encode_compact_key` (`0x65EB40`)。编码循环每次读取连续四个 feature position，低位在前：

```text
nibble = bit[p+0] * 1
       | bit[p+1] * 2
       | bit[p+2] * 4
       | bit[p+3] * 8
key_byte = ASCII('A') + nibble
p += 4
```

循环覆盖到 `p == 72`，因此输出恰好 18 字节；有效位置只有 `0..70`，最后一个字符的 bit 3 对应 padding position 71，编码时恒为零。

Simple selector 的 virtual `+0x10` 先由 `sea_simple_shader_decode_compact_key` (`0x65EA80`) 读取恰好 18 个字符。每个字符执行 `byte - 'A'`，只检查其低四位，并且只写入 `< 71` 的位置；没有十六进制文本解析步骤。

## 完整 71 项 descriptor

`sea_simple_shader_selector_construct` (`0x65ED50`) 在 `0x65EE3A` 把 `0x189CD58` 的 71 个、每项 16 字节的记录注册到公共 define manager。记录包括 position、逐位名称、可选共享参数名称和共享值的 bit index。共享名称为空时，逐位名称本身就是 define 名称。注册器把实际 define 名称转为大写作为有序 map key，所以最终前缀输出的是下表 `define` 列的大写形式。

| pos | descriptor | define | value bit |
|---:|---|---|---:|
| 0 | `SSF_None` | `SSF_None` | 0 |
| 1 | `SSF_UserShader` | `SSF_UserShader` | 0 |
| 2 | `SSF_2DTransform` | `SSF_2DTransform` | 0 |
| 3 | `SSF_Vertex_Normal` | `SSF_Vertex_Normal` | 0 |
| 4 | `SSF_Vertex_NormalUByte4N` | `SSF_Vertex_NormalUByte4N` | 0 |
| 5 | `SSF_Vertex_Tangent` | `SSF_Vertex_Tangent` | 0 |
| 6 | `SSF_Vertex_TangentUByte4N` | `SSF_Vertex_TangentUByte4N` | 0 |
| 7 | `SSF_Vertex_Binormal` | `SSF_Vertex_Binormal` | 0 |
| 8 | `SSF_Vertex_BinormalUByte4N` | `SSF_Vertex_BinormalUByte4N` | 0 |
| 9 | `SSF_Vertex_ColorBit0` | `SSF_Vertex_Color` | 0 |
| 10 | `SSF_Vertex_ColorBit1` | `SSF_Vertex_Color` | 1 |
| 11 | `SSF_Vertex_TexcoordBit0` | `SSF_Vertex_Texcoord` | 0 |
| 12 | `SSF_Vertex_TexcoordBit1` | `SSF_Vertex_Texcoord` | 1 |
| 13 | `SSF_Vertex_BlendWeight` | `SSF_Vertex_BlendWeight` | 0 |
| 14 | `SSF_Vertex_BlendIndices` | `SSF_Vertex_BlendIndices` | 0 |
| 15 | `SSF_Vertex_PointSize` | `SSF_Vertex_PointSize` | 0 |
| 16 | `SSF_OutColor_ModeBit0` | `SSF_OutColor_Mode` | 0 |
| 17 | `SSF_OutColor_ModeBit1` | `SSF_OutColor_Mode` | 1 |
| 18 | `SSF_OutDistance_Color0A` | `SSF_OutDistance_Color0A` | 0 |
| 19 | `SSF_OutDistance_Color0RD2G` | `SSF_OutDistance_Color0RD2G` | 0 |
| 20 | `SSF_NoUpdateDistance` | `SSF_NoUpdateDistance` | 0 |
| 21 | `SSF_OutVelocityBit0` | `SSF_OutVelocity` | 0 |
| 22 | `SSF_OutVelocityBit1` | `SSF_OutVelocity` | 1 |
| 23 | `SSF_PixelAlphaTest` | `SSF_PixelAlphaTest` | 0 |
| 24 | `SSF_AlphaBlend` | `SSF_AlphaBlend` | 0 |
| 25 | `SSF_SoftEdge` | `SSF_SoftEdge` | 0 |
| 26 | `SSF_ParticleShader` | `SSF_ParticleShader` | 0 |
| 27 | `SSF_ReductionMode` | `SSF_ReductionMode` | 0 |
| 28 | `SSF_Fog_ModeBit0` | `SSF_Fog_Mode` | 0 |
| 29 | `SSF_Fog_ModeBit1` | `SSF_Fog_Mode` | 1 |
| 30 | `SSF_Vtf_Fog_ModeBit0` | `SSF_Vtf_Fog_Mode` | 0 |
| 31 | `SSF_Vtf_Fog_ModeBit1` | `SSF_Vtf_Fog_Mode` | 1 |
| 32 | `SSF_LightEffectModeBit0` | `SSF_LightEffectMode` | 0 |
| 33 | `SSF_LightEffectModeBit1` | `SSF_LightEffectMode` | 1 |
| 34 | `SSF_LightParallelBit0` | `SSF_LightParallel` | 0 |
| 35 | `SSF_LightParallelBit1` | `SSF_LightParallel` | 1 |
| 36 | `SSF_BaseMap` | `SSF_BaseMap` | 0 |
| 37 | `SSF_MultiTexMap0` | `SSF_MultiTexMap0` | 0 |
| 38 | `SSF_MultiTexMap1` | `SSF_MultiTexMap1` | 0 |
| 39 | `SSF_RefractionMapBit0` | `SSF_RefractionMap` | 0 |
| 40 | `SSF_RefractionMapBit1` | `SSF_RefractionMap` | 1 |
| 41 | `SSF_BlendModeBit0` | `SSF_BlendMode` | 0 |
| 42 | `SSF_BlendModeBit1` | `SSF_BlendMode` | 1 |
| 43 | `SSF_BlendModeBit2` | `SSF_BlendMode` | 2 |
| 44 | `SSF_BlendModeBit3` | `SSF_BlendMode` | 3 |
| 45 | `SSF_BlendModeBit4` | `SSF_BlendMode` | 4 |
| 46 | `SSF_BlendModeBit5` | `SSF_BlendMode` | 5 |
| 47 | `SSF_MultiTex0BlendModeBit0` | `SSF_MultiTex0BlendMode` | 0 |
| 48 | `SSF_MultiTex0BlendModeBit1` | `SSF_MultiTex0BlendMode` | 1 |
| 49 | `SSF_MultiTex0BlendModeBit2` | `SSF_MultiTex0BlendMode` | 2 |
| 50 | `SSF_MultiTex0BlendModeBit3` | `SSF_MultiTex0BlendMode` | 3 |
| 51 | `SSF_MultiTex1BlendModeBit0` | `SSF_MultiTex1BlendMode` | 0 |
| 52 | `SSF_MultiTex1BlendModeBit1` | `SSF_MultiTex1BlendMode` | 1 |
| 53 | `SSF_MultiTex1BlendModeBit2` | `SSF_MultiTex1BlendMode` | 2 |
| 54 | `SSF_ShadowParallel` | `SSF_ShadowParallel` | 0 |
| 55 | `SSF_ShadowMapBit0` | `SSF_ShadowMap` | 0 |
| 56 | `SSF_ShadowMapBit1` | `SSF_ShadowMap` | 1 |
| 57 | `SSF_ShadowMapEdgeHide` | `SSF_ShadowMapEdgeHide` | 0 |
| 58 | `SSF_ShadowMapColorShadowBit0` | `SSF_ShadowMapColorShadow` | 0 |
| 59 | `SSF_ShadowMapColorShadowBit1` | `SSF_ShadowMapColorShadow` | 1 |
| 60 | `SSF_ShadowMap0_ModeBit0` | `SSF_ShadowMap0_Mode` | 0 |
| 61 | `SSF_ShadowMap0_ModeBit1` | `SSF_ShadowMap0_Mode` | 1 |
| 62 | `SSF_ShadowMap0_ModeBit2` | `SSF_ShadowMap0_Mode` | 2 |
| 63 | `SSF_ShadowMap1_ModeBit0` | `SSF_ShadowMap1_Mode` | 0 |
| 64 | `SSF_ShadowMap1_ModeBit1` | `SSF_ShadowMap1_Mode` | 1 |
| 65 | `SSF_ShadowMap1_ModeBit2` | `SSF_ShadowMap1_Mode` | 2 |
| 66 | `SSF_ShadowMap2_ModeBit0` | `SSF_ShadowMap2_Mode` | 0 |
| 67 | `SSF_ShadowMap2_ModeBit1` | `SSF_ShadowMap2_Mode` | 1 |
| 68 | `SSF_ShadowMap2_ModeBit2` | `SSF_ShadowMap2_Mode` | 2 |
| 69 | `SSF_DepthWrite` | `SSF_DepthWrite` | 0 |
| 70 | `SSF_Debug` | `SSF_Debug` | 0 |

`sea_simple_shader_append_define_prefix` (`0x65EA10`) 先把所有已注册 define 的值清零，再扫描全部 71 个位置。每个置位 descriptor 向其 define 累加 `1 << value_bit`。`sea_append_shader_defines` (`0x6C5160`) 随后按大写 map key 的字典序输出所有 define，包括值仍为零的 define：

```text
#define <UPPERCASE_REGISTERED_NAME> <signed_decimal_value>\n
```

## 完整 integer selector parameter 映射

`sea_simple_shader_selector_construct` 建立了 16 个 parameter ID 到 position vector 的精确映射。下表是完整集合，不是只列出当前 SRD direct key 会触发的子集：

| integer parameter ID | positions | emitted parameter |
|---:|---|---|
| 1 | `2` | `SSF_2DTransform` |
| 6 | `41..46` | `SSF_BLENDMODE`，6 位 |
| 7 | `47..50` | `SSF_MULTITEX0BLENDMODE`，4 位 |
| 8 | `51..53` | `SSF_MULTITEX1BLENDMODE`，3 位 |
| 9 | `16..17` | `SSF_OUTCOLOR_MODE`，2 位 |
| 11 | `19` | `SSF_OutDistance_Color0RD2G` |
| 22 | `27` | `SSF_ReductionMode` |
| 23 | `25` | `SSF_SoftEdge` |
| 24 | `39..40` | `SSF_REFRACTIONMAP`，2 位 |
| 30 | `26` | `SSF_ParticleShader` |
| 40 | `34..35` | `SSF_LIGHTPARALLEL`，2 位 |
| 52 | `28..29` | `SSF_FOG_MODE`，2 位 |
| 53 | `30..31` | `SSF_VTF_FOG_MODE`，2 位 |
| 61 | `32..33` | `SSF_LIGHTEFFECTMODE`，2 位 |
| 66 | `69` | `SSF_DepthWrite` |
| 68 | `70` | `SSF_Debug` |

`sea_simple_shader_apply_parameter_positions` (`0x65E950`) 对每个 parameter value 从 bit 0 开始检查；置位的 value bit 设置对应 vector 元素指向的 feature position。Rust 的 `CEYLON_SIMPLE_SELECTOR_PARAMETERS` 与 `set_selector_parameter_value` 保存这张完整映射，但不自行假定 provider 激活。此前 ShapeEnv 模块调用链已经证明 Blend、MultiTex0、MultiTex1 分别写 parameter ID `6/7/8`，base SoftEdge/Refraction/DepthWrite 写 `23/24/66`，可选 ShapeEnv2D 写 `1`；其余 ID 的 SRD context 可达性继续单独取证。

parameter ID `10` 是这张表之外的特殊分支。`sea_simple_shader_selector_build_compact_key` (`0x660120`) 在 ID `10` 非零且 shape alpha blend 关闭时直接设置 position `18` (`SSF_OutDistance_Color0A`)。唯一注册类 RTTI 为 `sea::AppendParamWriteDistanceTarget`；`sea::PassEnvWriteDistance` 的应用方法把该值提升为 `1`。其构造函数唯一调用点位于 `sea::FilterSrcMsaa` 的内嵌 pass 对象，因此它不是 ShapeEnv 64-bit cache key 的直接贡献，而是特定 render-pass context。Rust 以 `set_write_distance_target_contribution` 显式暴露该分支，但 SRD direct-key 方法不默认启用它。

## `LightShadowParallel` 与 parameters 43..48

parameters `43..48` 同样不是上述 16 项普通 position vector，而是 `sea_simple_shader_selector_build_compact_key` (`0x660120`) 的专用 shadow 分支。其 provider 已由 executable 闭环到 RTTI `sea::LightShadowParallel`：

1. `sea_append_param_util_shadow_parallel_register` (`0x681360`) 的 RTTI 为 `sea::AppendParamUtilShadowParallel`，把 local slots `1/2/3` 注册为 integer IDs `43/44/45`，把三个 cascade 的 local slots `8/13/18` 注册为 IDs `46/47/48`；其余 slots 是 `lightShadow*`、`textureShadow{0..2}`、`lightShadowSize{0..2}`、`lightShadowMatrix{0..2}` named resources；
2. `sea_light_shadow_parallel_construct` (`0x681B60`) 注册的 authored property 索引精确为：`9 = Softness`（默认 0、范围 0..4）、`10 = EdgeHide`（默认 false）、`11 = ColorShadow`（默认 0、范围 0..3）、`12 = CascadeNum`（默认 2、范围 1..3）；
3. `sea_light_shadow_parallel_rebuild_cascades` (`0x684B00`) 读取 property `12`，按 `0 -> 1`、`>3 -> 3` 归一化后把 live cascade-node vector 调整为 1..3 项；
4. `sea_light_shadow_parallel_append_parameters` (`0x687920`) 仅在其 pass 参数为 0 时写入 provider。它把 `EdgeHide != 0` 覆盖到 ID `44`，把 `ColorShadow` 覆盖到 ID `45`；遍历每个 live cascade 时，以 1-based cascade index 逐次提升 ID `43`，因此最终 ID `43` 等于 live cascade count；
5. 同一循环把首级 `Softness` 原值写入 ID `46`。后一级模式为 `max(min(previous, 2) - 1, 0)`：只有存在第二个 cascade 且首级 `Softness >= 2` 时 ID `47 = 1`；第三个 cascade 的 ID `48 = 0`。

Simple selector 对这六个值的消费顺序也已精确闭环：

- ID `43 == 0` 时 positions `54..68` 全部不由 shadow 分支设置；
- ID `43 != 0` 时 position `54` (`SSF_ShadowParallel`) 无条件设置；
- 只有 shape `+0x90` bit `3` 非零时才继续设置 positions `55..68`。`ceylon_create_shape_environment` 证明该位来自 ShapeEnv cache key low bit `6`；
- ID `43` 先取 `min(value, 3)`，其 bits 0/1 写 positions `55/56`；ID `44 != 0` 写 position `57`；ID `45` 先取 `min(value, 3)`，写 positions `58/59`；IDs `46/47/48` 分别先取 `min(value, 7)`，再写 `60..62`、`63..65`、`66..68`。

Rust 因而提供 `CeylonShadowParallelParameters::from_light_shadow_parallel`、`set_shadow_parallel_contribution` 与 `srd_simple_shader_with_shadow_parallel`。最后一个 API 只在调用者明确提供场景级 shadow provider 时组合该上下文；它不会因为 key low bit `6` 单独存在就虚构 `LightShadowParallel`。完整 data 语料中只有 `acroarts/st_effect_use_list.afb`、`A000/stage/stage000058/st_00058.afb`、`A000/stage/stage000063/st_00063.afb` 含有 `LightShadowParallel` 类名，证明该 graph node 确实存在于发布资源，但类名语料不被反向当作任意 SRD draw 激活它的证据。

例如 MultiTex0 variant `9` 的二进制值为 `1001b`，因此设置 positions `47` 与 `50`。在紧凑键中 position 47 是第 12 个字符的 bit 3，position 50 是第 13 个字符的 bit 2，局部编码恰为 `I`、`E`。

## 两套参数表与 `ShapeEnv2D` 的 position 2 来源

`sea_simple_shader_selector_construct` (`0x65ED50`) 还精确注册了 parameter ID `1` 到 position `2`，即 `SSF_2DTransform`。这是 selector 自身的静态映射，不依赖 shader collection XML。

游戏的 parameter manager 与 parameter set 各自维护两套互不混用的表：

- named resource parameter registry 位于 manager `+0xB4`，parameter set 的资源指针 vector 位于 `+0xC0`。`sea_shader_parameter_registry_intern` (`0x630FD0`)、`sea_append_param_util_register_slot` 与 `sea_append_param_util_append_slot` 操作这套表；
- integer selector parameter registry 位于 manager `+0xC0`，parameter set 的整数 value vector 位于 `+0xCC`。`sub_65CAC0(local_slot, integer_id)` 把 utility local slot 映射到指定的固定整数 ID，`sea_shader_parameter_set_get` (`0x6BBF10`) 与 `sub_6BC540` 读取/写入这套表。

因此 named 参数的注册顺序不会产生 integer selector parameter ID。此前把 `AllEnvBasic` 的名称与 parameter ID `1` 联系起来的解释不成立；它的 7 个逻辑名称全部属于 named resource 表，不能直接设置 Simple feature position。

对 executable 中所有 `sub_65CAC0` 调用点的完整枚举显示，固定整数 ID `1` 只有一个注册点：`sea_shape_env_2d_register_selector_parameter` (`0x6CF4B0`) 将 `sea::ShapeEnv2D` 的 local slot `0` 映射到 ID `1`。其 RTTI、构造和应用链进一步闭环为：

1. `ceylon_environment_manager_construct` (`0x66E4D0`) 构造 RTTI 为 `sea::ShapeEnv2D` 的对象并保存到 manager `+0x180`；
2. `ceylon_create_shape_environment` (`0x670680`) 仅在 cache key low bit `3` 非零时把 `manager+0x180` 应用到 shape；
3. `sea_shape_apply_environment_module` (`0x6AFD10`) 调用模块 virtual `+0x18`，落到 `sea_shape_env_2d_apply` (`0x6CF560`)；
4. 该方法通过 `sea_append_param_util_raise_selector_value` (`0x65D320`) 写 local slot `0` 的值 `1`。后者先读取当前整数 selector value，再写入 `max(current, 1)`；最终目标就是 parameter set `+0xCC` 的 ID `1` 项；
5. Simple selector 将 ID `1` 的 value bit `0` 映射到 position `2`，得到 `SSF_2DTransform`。

base environment vector 是另一条独立路径。构造器只按顺序加入 `null/SoftEdge/Refraction/Refraction2/DepthWrite` 五项，对应 variants `0..4`；不存在 base variant `5 = ShapeEnv2D`。因此 Rust 已删除旧的 variant `5 -> position 2` 映射，并拒绝 base variants `5..7`。position `2` 的已证直接 key 来源只有 low bit `3` 的可选 `ShapeEnv2D` 模块。

SRD quad 的 ShapeEnv cache key 能直接证明的 Simple contributions 已单独实现：format 14 的 color/texcoord 位、实际 texture slot 数、alpha blend、反向的 `NoUpdateDistance`、base environment `0..4`、low bit `3` 的 `ShapeEnv2D`，以及 ShapeEnv parameter `6/7/8`。shader collection 中“只差 position 2”的键仍只作语料诊断，不能反向证明某次运行时 draw 启用了 `ShapeEnv2D`。

## 完整 data 目录的独立校验

完整游戏数据 `D:\sdhd\assets\data\A000\shader\shadercollect.xml` 包含 `SimpleShaderVSSimpleShaderPS_-382718031` 组。该组有 82 个键，全部严格为 18 字节；Rust 对每个键执行二进制同构解码再编码，82 个均逐字节回到原值。

按 positions `47..50` 解码，82 个 Simple shader key 的 MultiTex0 variant 分布为：`0:58`、`6:1`、`9:1`、`10:1`、`11:10`、`12:11`。其中唯一的 variant `9` 键是 `AAEBABBAADIIEAAAAA`，其 `I/E` 位型精确得到数值 `9`。这份 XML 只作为完整资源语料对二进制算法的独立校验；selector 选择、position 语义和编码规则仍由上述 executable 调用链决定。

## 当前实现边界

Rust 已实现：

- selector registry 的 9 个 Default + 1 个 Simple 类型映射；
- Simple 的完整 71 项 descriptor；
- 18 字节键的逐位编解码；
- 所有 uppercase define 的清零、累加和有序前缀输出；
- 全部 16 个 integer selector parameter ID 到 Simple positions 的映射 API；
- parameter `10` 的 non-blended write-distance pass 特殊分支；
- `LightShadowParallel` authored properties、cascade 构造、parameters `43..48` provider、Simple positions `54..68` 与 key low bit `6` gate；
- base environment `0..4` 与 low bit `3` 的 `ShapeEnv2D -> SSF_2DTransform` 映射，并拒绝未构造的 base `5..7`；
- 完整 data 中 82 个 Simple key 的回归。

`SimpleShaderVS.cg/SimpleShaderPS.cg` 的原始 source、include 闭包、双 UV/顶点色公式、完整 collection 的无 D3DX bytecode及其 runtime key 表均已闭环，见 [`render-shader-source.md`](render-shader-source.md) 与 [`render-shader-bytecode.md`](render-shader-bytecode.md)。尚未闭环的是其余 shape/context feature 输入如何在 SRD 的每一种运行时状态下形成全部 positions。shadow provider 现在可以显式组合，但不能在没有场景 graph 证据时默认附加到独立 SRD 预览。

编辑器原生渲染器不再把 selector key 作为 draw contract，也不以 collection 中是否存在某个键来决定能否绘制。已从 Cg/bytecode 与 D3D9 像素回归闭环的可见 SimpleShader 子集现收敛为 `SimpleShaderProfile`：format 13/14 declaration、2D/3D transform、0/1/2 个连续纹理槽、MultiTex0 `0/6/9/10/11/12`、surface mode `0/9` 与 render preset `0..21/33..61`。其中影响 shader 程序的字段作为 WebGPU pipeline constants；矩阵、材质数值、alpha reference 与 sampler 仍为 draw 数据。未闭环的 selector positions、第三纹理槽和 render preset `22..32` 被显式拒绝，不以通用分支猜测。
