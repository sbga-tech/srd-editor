# SRD 四顶点格式与 D3D9 primitive 提交证据

分析对象：

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`；
- 保存后的 IDB SHA-256：`1B780E16652216CA5021F0A31EDCD7EDBBD46B16D39EF0C06810875D4BE906E9`。

## 36 字节顶点格式

`srd_render_image_cast` (`0xAD77D0`) 在 `0xAD7C75..0xAD7D16` 连续写四个顶点，每次把目标指针增加 `0x24`。`srd_render_slice_cast` 和 `srd_render_number_glyph` 使用相同步长和字段偏移。格式如下：

| 顶点偏移 | 大小 | 已证明来源 |
| --- | --- | --- |
| `+0x00` | 12 | 三个 f32 位置分量 |
| `+0x0C` | 4 | primary packed color |
| `+0x10` | 4 | secondary packed color |
| `+0x14` | 8 | UV 通道 0，两个 f32 |
| `+0x1C` | 8 | UV 通道 1，两个 f32 |

Image、Slice 和 Number 都按四顶点顺序写入。Image/Number 的 primary 为描述符顶点色乘 CAST multiplicative tint；secondary 先把 CAST additive tint 转为 `[R*A/255, G*A/255, B*A/255, 0]`。Slice 的 secondary 是 CAST additive tint 与 SLIC `0x33` 的逐通道饱和加法，不走 Image/Number 的 alpha 预乘路径。

Rust 的 `#[repr(C)] SrdRenderVertex` 固定上述字段顺序，并以 `size_of`/`offset_of` 测试验证 36 字节布局。`ImageDefinition::build_render_quad` 已把动画后的 size/origin、第一份描述符顶点色、两份最终 UV 和两种世界 tint 组合成同布局四顶点；`build_slice_render_quad` 使用 Slice 已证明的颜色链，并把同一最终 UV 复制到两个通道；`NumberDefinition::build_glyph_render_quad` 则使用已排版 glyph 的四个位置和 Number 的双描述符状态建立同格式顶点。

## 格式 14 的 D3D9 vertex declaration

全局顶点格式注册器在 `sub_671D30` 的 case `14` 精确追加五个元素。每个元素保存内部 semantic、type 和 usage index；`sub_1319690` 再累计同一 stream 的 byte offset，并通过两个映射表写成 8 字节 `D3DVERTEXELEMENT9`。type 表为恒等映射 `0..16`；semantic 表在 `0..8` 后把内部 `9..12` 映射为 D3D9 usage `10..13`。

格式 `14` 最终声明为：

| Stream | Offset | Type | Usage | Index |
| --- | --- | --- | --- | --- |
| 0 | 0 | `D3DDECLTYPE_FLOAT3` (`2`) | `POSITION` (`0`) | 0 |
| 0 | 12 | `D3DDECLTYPE_D3DCOLOR` (`4`) | `COLOR` (`10`) | 0 |
| 0 | 16 | `D3DDECLTYPE_D3DCOLOR` (`4`) | `COLOR` (`10`) | 1 |
| 0 | 20 | `D3DDECLTYPE_FLOAT2` (`1`) | `TEXCOORD` (`5`) | 0 |
| 0 | 28 | `D3DDECLTYPE_FLOAT2` (`1`) | `TEXCOORD` (`5`) | 1 |
| `0xFF` | 0 | `D3DDECLTYPE_UNUSED` (`17`) | 0 | 0 |

`sub_1319690` 以设备虚表 `+0x158` 调用 `IDirect3DDevice9::CreateVertexDeclaration`，之后提交路径以 `+0x15C` 调用 `SetVertexDeclaration`。Rust 的 `SRD_D3D9_VERTEX_DECLARATION` 直接保存上述六条记录，可交给后续 D3D9 后端创建声明对象。

## packed color 与 shader 语义边界

`D3DDECLTYPE_D3DCOLOR` 把 little-endian 顶点内存的 `B,G,R,A` 字节送入 shader 的语义 `R,G,B,A`。远端 trace 提取并保留的 `diagnostics/parity/analysis/common-background-native-vb.bin` 中，最终帧 format-14 顶点 `2136` 的 primary 原始字节为 `[FB, FF, AC, FF]`；shader 看到的颜色因此是 `[AC, FF, FB, FF]`。这与同一 draw 的 WebGPU format-14 顶点逐分量一致，不能在 WGSL 中再次交换红蓝。

Rust 的 backend-neutral CIMG/CNUM/CSLI 顶点色和 CAST multiply/additive tint 因而统一保存语义 RGBA。SRD 属性 `0x44` 的 packed `[A,R,G,B]` 在解析时转换，动画 KEY color 的 D3DCOLOR 字节在写入 runtime state 时转换；format-14 WGSL 原样传递颜色。Fennel 是另一条已证明边界：其 stride-28 CPU 输出仍明确保存 native `primary_color_bgra/secondary_color_bgra`，所以 format-13 vertex shader 才执行 `.zyxw`。这一区分防止修正 ImageCast 时反向破坏 TextCast。

CIMG/CNUM/CSLI 属性转换、公共颜色动画与 format-14 上传各有单元回归；四个远端 SRD 的 native/WebGPU 比较还证明把 format-14 全局再交换红蓝会显著恶化画面，而不是修复剩余几何差异。


## 游戏内部格式与 primitive 参数

`srd_render_image_cast`、每个 active Slice 单元和每个 Number glyph 都经 `sub_AC5320` 建立绘制包。该函数在 `0xAC54A1` 对顶点批次对象的虚表 `+0x08` 传入：

```text
vertex_format_id = 14
primitive_type   = 4
vertex_count     = 4
```

四顶点的几何顺序是左上、左下、右上、右下，正好形成 triangle strip。后端 `d3d9_map_internal_primitive_type` (`0xE5B490`) 使用表 `[1,2,3,4,5,6]`，所以内部类型 `4` 映射为 D3D9 primitive 值 `5`，即 `D3DPT_TRIANGLESTRIP`。`d3d9_primitive_count_from_vertex_count` (`0xE5B420`) 对内部类型 `4` 返回 `vertex_count - 2`，四顶点因此提交两个 primitive。

## `ceylon_enqueue_draw_packet` 的相邻合并

`ceylon_enqueue_draw_packet` (`0x670BE0`) 总是先建立新的 304-byte record；只有上一条 record 存在且 enqueue target 指针相同，才尝试与紧邻上一条合并。比较范围由逐 DWORD 循环直接给出：

| 新旧 record 比较区间 | 大小 | 当前已闭环含义 |
| --- | ---: | --- |
| `+0x00..+0x6B` | 108 | DrawPacket flags、深度/stencil、三个 texture wrapper 等前缀 |
| `+0x70..+0x7B` | 12 | vertex format、primitive type、固定零字段 |
| `+0x80..+0x8B` | 12 | DrawMask、command flags、完整 renderer layer key；其最低字节也承载 stencil sequence |
| `+0xA0..+0xDB` | 60 | 仅当新 record `+0x9E != 0` 时比较 packet current matrix 的前 15 个 f32 |

任一区间不相等即保留独立 record。矩阵 setter `0x6DF2D0` 证明：2D/null matrix 清零 `+0x9E`，因此不比较矩阵；3D/non-null matrix 设置 `+0x9E=1`，因此不同 world matrix 会阻止合并。

比较全部通过后，仅 primitive type `0/1/3/6` 和 `4` 允许合并：

- type `0/1/3/6`：上一条 `vertex_count += 新条 vertex_count`；Fennel 的 type 3 triangle list 属于此分支；
- type `4`：先在 vertex buffer 中写入两个退化连接顶点，再执行 `上一条 vertex_count += 新条 vertex_count + 2`；SRD quad triangle strip 属于此分支；
- 其他 type 不合并。

新建 record 随后从 vector 尾部移除并析构；因此 target queue 看到的是合并后的单个 command，不是两个 command 的后期视觉批处理。

type 4 的两个复制顺序已由 `0x670E19..0x670E3D` 逐参数闭环。第一段 `memcpy(dest = next_first - stride, src = next_first, size = stride)` 把新 strip 首顶点复制到前一个保留槽；第二段 `memcpy(dest = next_first - 2*stride, src = next_first - 3*stride, size = stride)` 把旧 strip 末顶点复制到后一个保留槽。因此边界四项精确为 `last,last,first,first`，CPU materializer 使用“旧流 + 旧末顶点 + 新首顶点 + 新 quad”，不是任选两个退化点。

Rust 的 `build_runtime_target_commands` 已对普通 Image、每个 active Slice cell、每个可绘制 Number glyph 和 Fennel 路径复现上述相邻比较、完整 renderer layer key、strip `+2` 与 triangle-list 直接累加，并保留每个合并 record 对应的逻辑 source 列表。不同宿主 `2DLayer`、CATR layer override、NODE `0xA0` 偏移或 RefCast 低字节结果都会阻止错误合并。它只接受当前能够完整构造比较键的路径：单一 SrPlayer/同一 enqueue target 状态、无显式 texture override、无 special-depth，且 stencil 关闭。Stencil 开启时逐提交 sequence 生命周期尚未作为独立 runtime 状态闭合，函数会报错而不是静默少合并。

普通编辑器 Composition 已按 planner 的 target group 与 source 顺序混合提交 Image/Slice/Number/Fennel。format-14/type-4 group 由 `materialize_runtime_srd_strip` 物理生成上述退化连接流，并由可按需扩到二次幂容量的 DEFAULT-pool dynamic vertex buffer 一次上传、一次 `DrawPrimitive(vertex_count - 2)`；`ResetEx` 时容量与 COM buffer 一起失效并按需重建。Common `yellow_loop` 的 241 个 source 因而真实提交为 planner 的 10 个 format-14 draw call，而不是 241 次独立 quad draw。

format-13/type-3 也已物理闭合。`materialize_runtime_fennel_list` 按 command source 顺序直接连接 batch vertices，并验证所有 source 的 packet、renderer layer key 与 texture token 一致；没有退化顶点或重排。Advertise `AS_warning_in` frame 24 的 20 个 Fennel source 因而物化为一条 3072 顶点 list，并以一次 `DrawPrimitive(D3DPT_TRIANGLELIST, 0, 1024)` 提交。

Common 实际样本 `yellow_loop` 第 0 帧现由统一 runtime 枚举产生 241 个普通 draw：100 个 Image 和 141 个 active SliceCell，全部携带 `0x8680`。相邻比较精确合并为 10 个 record，source 数依次为 `[2,1,179,1,4,2,6,2,36,8]`，含退化连接顶点后的 vertex count 依次为 `[10,4,1072,4,22,10,34,10,214,46]`；固定回归同时验证 241 个 source 没有丢失或重排成额外 target record。

## 绘制包到 IDirect3DDevice9

`ceylon_apply_draw_packet_state` (`0x6CEE30`) 同步渲染状态、纹理和采样器，再进入 `ceylon_submit_draw_packet` (`0x6CF110`)。后者完成以下设备调用：

- 设备虚表 `+0x15C`：`IDirect3DDevice9::SetVertexDeclaration`；
- 设备虚表 `+0x190`：`IDirect3DDevice9::SetStreamSource`；
- 设备虚表 `+0x198`：`IDirect3DDevice9::SetStreamSourceFreq`；
- 设备虚表 `+0x1A0`：`IDirect3DDevice9::SetIndices`；
- `d3d9_draw_nonindexed_primitive` (`0xE58930`) 的设备虚表 `+0x144`：`IDirect3DDevice9::DrawPrimitive`。

SRD 四顶点类型走非索引路径，最终参数等价于：

```text
DrawPrimitive(D3DPT_TRIANGLESTRIP, start_vertex, 2)
```

引擎也预建了 `(0,1,3, 1,2,3)` 的 quad index pattern，但那属于内部 primitive 类型 `6` 的独立 indexed 分支，不能替代 SRD 当前实际提交的类型 `4`。

## 仍未闭环

- draw packet 的 shader、blend、depth、stencil、scissor、cull、fill 与 color-write 已分别闭环；Image/Slice/Number format-14 与 Fennel format-13 的普通相邻 record 都已物理合并并接到编辑器 D3D9Ex。其余边界是 stencil sequence/special-depth 等尚未进入 runtime record 的状态。
- DDS 描述符、D3D9/D3DX9_43 创建参数和二维 SYSTEMMEM staging/`UpdateSurface` 已闭环，见 [`dds-resource-loading.md`](dds-resource-loading.md)；内部格式转换、cube request 和设备丢失/重建仍待闭环。
