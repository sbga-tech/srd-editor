# SRD 格式与游戏逻辑证据账本

旧 Python 编辑器、旧报告和会话摘要只提供待调查线索，不构成格式或运行时语义证据。本文件中的每项结论必须最终附带游戏二进制地址、反编译/汇编依据和对应样本验证；未完成闭环的内容一律标记为“待验证”，不能直接进入 Rust 实现。

具体的跨模块待闭环问题见 [`TODO.md`](TODO.md)。

## 证据等级

- **已证明**：游戏二进制中的读取/写入/分派代码与至少一个实际样本能够互相印证。
- **部分证明**：已经定位二进制代码，但结构字段、调用上下文或样本行为尚未闭环。
- **待验证线索**：仅来自旧实现、旧文档、字符串、标签名或样本相关性。
- **未知**：没有足够证据，不赋予语义。

已完成证据闭环的内容：

- VTBF 读取逻辑：[`evidence/vtbf-reader.md`](evidence/vtbf-reader.md)
- 标签分派图：[`evidence/tag-dispatch.md`](evidence/tag-dispatch.md)
- 动画记录布局：[`evidence/animation-records.md`](evidence/animation-records.md)
- SCN `ANMS/SANM` 场景动画集、layer gate 与 AdvertiseLogo 阶段 identity：[`evidence/scene-animation-sets.md`](evidence/scene-animation-sets.md)
- LAYR/NODE/TRS、公共动画通道和矩阵链：[`evidence/scene-transform.md`](evidence/scene-transform.md)
- CSLI/SLIC 网格、NODE `0x32` 与父级单元偏移：[`evidence/csli-layout.md`](evidence/csli-layout.md)
- SrSliceCast active 单元局部顶点、CREF 选择与最终 UV：[`evidence/slice-geometry.md`](evidence/slice-geometry.md)
- CIMG、CREF/CRE1 双通道与 SrImageCast：[`evidence/cimg-image-cast.md`](evidence/cimg-image-cast.md)
- CNUM 解析、SrNumberCast 初值与 glyph 映射：[`evidence/cnum-number-cast.md`](evidence/cnum-number-cast.md)
- SrImage 尺寸 `11/12`、顶点色 `13..16` 与双坐标描述符 `17/20` 动画：[`evidence/image-coordinate-animation.md`](evidence/image-coordinate-animation.md)
- TEXT、PROJ FONT/CHAR、SrTextCast 初始化与外部 RFZ BinaryLZW 边界：[`evidence/text-font-records.md`](evidence/text-font-records.md)
- RFZ BinaryLZW、通用 YABX、Ruhuna 字形对象、AVTS 目录与内嵌 DDS 图集：[`evidence/ruhuna-font-archives.md`](evidence/ruhuna-font-archives.md)
- CRFD、SrRefCast 与引用动画帧通道 `23`：[`evidence/crfd-reference-cast.md`](evidence/crfd-reference-cast.md)
- PROJ/SCN 同文件场景表与 CRFD 的 SCN/LAYR 两级解析：[`evidence/project-scene-reference.md`](evidence/project-scene-reference.md)
- TEX `0x62`、双采样包装对象、SrSliceCast 选择与 D3D9 采样状态：[`evidence/texture-binding.md`](evidence/texture-binding.md)
- TEXL/TEX/CROP 记录、归一化矩形与外部 DDS 路径：[`evidence/texture-table.md`](evidence/texture-table.md)
- `air::TextureResource`、DDS 描述符、mip/cube/palette 布局及 D3D9/D3DX9_43 创建分支：[`evidence/dds-resource-loading.md`](evidence/dds-resource-loading.md)
- CAST `CATL/CATR`、`ExtParamData`、preset 覆盖与继承层级键：[`evidence/cast-extended-parameters.md`](evidence/cast-extended-parameters.md)
- SrImage render-preset 选择、62 项混合表、draw packet 编码与 D3D9 blend state：[`evidence/render-blend-state.md`](evidence/render-blend-state.md)
- Ceylon shader cache key、SRD vertex format 与 ShapeEnv 模块索引：[`evidence/render-shader-key.md`](evidence/render-shader-key.md)
- SRD 实际 SimpleShaderSelector、18 字节键与 71 项 feature 表：[`evidence/render-simple-selector.md`](evidence/render-simple-selector.md)
- Simple Cg source/公式、canonical assembly 与无 D3DX D3D9 bytecode：[`evidence/render-shader-source.md`](evidence/render-shader-source.md)、[`evidence/render-shader-bytecode.md`](evidence/render-shader-bytecode.md)
- 已替换的 D3D9Ex/ImGui 编辑器前端之历史 device、HiDPI 与 ResetEx 证据（当前 UI 架构见根目录 README）：[`evidence/editor-d3d9-backend.md`](evidence/editor-d3d9-backend.md)
- 首个 stage-0 贴图 draw、完整 Composition 回读及宿主 `FirstCalcMatrix` 边界：[`evidence/render-first-textured-draw.md`](evidence/render-first-textured-draw.md)
- Chusan `AdvertiseLogoObject` 的实际 SrPlayer common-init 与 identity 根节点：[`evidence/chusan-advertise-logo-player.md`](evidence/chusan-advertise-logo-player.md)
- Chusan `PlayLinkedVerseGateObject` 的资源 id 84、`2DLayer=70` 与 null-target 宿主：[`evidence/chusan-linked-verse-gate-player.md`](evidence/chusan-linked-verse-gate-player.md)
- SrImage 原始 alpha/stencil packet、深度 flags、枚举映射与最终 D3D9 状态：[`evidence/render-alpha-depth-stencil.md`](evidence/render-alpha-depth-stencil.md)
- Draw/material scissor 来源选择、RenderState 传递与 D3D9 `SetScissorRect`：[`evidence/render-scissor-state.md`](evidence/render-scissor-state.md)
- 首个 fixture 的证据完整 CPU draw list 与明确排除边界：[`evidence/render-first-draw.md`](evidence/render-first-draw.md)

下述尚未附带该等级证据的 SRD 语义仍按“待验证线索”处理。

## 容器（待验证线索）

文件头：

```text
VTBF magic
unknown_04: 4 bytes
format: [u8; 4]，通常为 SRFF
block_count: u16
unknown_0e: 2 bytes
blocks...
```

块：

```text
sub_sig: [u8; 4]，常见 vtc0
block_size: u32
tag: [u8; 4]
child_count: u16
prop_count: u16
properties...
children...
```

游戏读取器已经证明魔数、format、顶层块数和块布局；`unknown_04` 与 `unknown_0e` 在已定位入口中没有被读取，不能沿用旧解析器的 `version`/`header_extra` 命名。写入规则与未知字节语义尚未证明。

## VTBF 属性（待验证线索）

属性的边界编码、类型大小表、count/mult 扩展和字符串长度前缀已经由游戏读取器证明，详见证据文档。字符串内容在该层仅按字节复制；是否要求 UTF-8 并未由读取器证明。

- 长度小于 127 字节时使用单字节长度。
- 长度为 127 至 32767 字节时使用双字节长度。
- 该读取器的两字节形式只能表达至 `0x7FFF`。

属性类型的业务语义仍需由各标签的消费代码逐项证明；不能仅凭宽度给 type code 命名。

当前 53 个样本使用旧 Python 解析器进行未编辑往返时，有 5 个文件并非字节一致。已观察到其中至少一种原因是短字符串也可能使用双字节长度前缀，而旧序列化器会改写为单字节形式。这只证明“原始编码必须保留”，并不证明完整字符串规则。

## 动画

当前二进制已经重新证明解析链：

```text
sub_A9FD60              Animation
  -> sub_427183         thunk
  -> sub_AA2290         TRK
  -> sub_457D1A
  -> sub_AA3FF0         KEY
```

但 KEY 记录不是固定 20 字节；游戏根据 TRK format 分派为 8 或 20 字节。20 字节分支之一的布局为：

```text
+0x00 frame
+0x04 value
+0x08 tangent_type
+0x0C tangent_in
+0x10 tangent_out
```

具体分派、时间折叠、端点、线性、保持和三次曲线公式已经由运行时求值器证明，详见证据文档。公共空间/visibility/颜色通道与 CAST 专属 `11..17/20/23` 均已证明并接入顶层及独立引用层运行时；未知目标仍不得猜测。

完整语料审计固定了 shipped 数据边界：91 个 SRD 的 150327 条 `TRK` 只使用 `0x13/0x23/0x43/0x51/0x113/0x123/0x143/0x151`，全部解析为已证明的 `Key20F32/Key20I32/Key8Bytes4`，没有真实 `Unsupported` KEY 布局；同一语料的 68511 条 CATR value 全部为已实现的 type code `2`，没有真实未知 CATR value 类型。Rust 保留拒绝未知输入的分支，但不会把 shipped 数据中不存在的格式伪造成新语义。

## 场景变换

LAYR flags 位 0、NODE/TRS2/TRS3 记录、公共运行时变换、游戏三角函数、局部 3x4 仿射矩阵以及 `parent_world * local` 世界组合顺序已经完成二进制闭环，详见 [`evidence/scene-transform.md`](evidence/scene-transform.md)。

NODE `0x3C/0x3D` 的首子/同级链以及根节点选择已经闭环并实现。NODE `0x32` 在父节点 `surfride::SrSliceCast` 时索引父级 CSLI 生成单元；尺寸、origin mode、自定义 origin、显式单元累计、越界、active、2D/3D 分支和中心偏移均已闭环并实现。

## 投影与视口

`PROJ` 直接子块 `CAM ` 的 position、target、angle units、near/far 已闭环到解析后的 `SrProject+0x58`，并继续闭环到全局 `sea::Camera` 的 RH View、Perspective、`Projection*View` 与 shader `mtxPrjView` provider。`SCN 0x40/0x41` 也已证明为 scene width/height。完整证据和 Rust 实现见 [`evidence/projection.md`](evidence/projection.md)。

游戏在构造 `SrRenderer+0x48` 时已经乘入 `[0,width] x [0,height]` 的像素视口映射和 Y 反转，因此三维 CAST 在除以 W 后得到最终屏幕 X/Y，不存在另一个尚未执行的 NDC-to-viewport 步骤。全局 Camera 输入已命名并实现；另一个 backend context 的逆/组合变换仍待追踪。

## 图像与特殊 CAST

- CSLI/SLIC 的解析、NODE 链接、type 2 分派、网格生成、父级偏移、active 单元局部四顶点以及用于 packed color 插值的单元归一化坐标已经闭环，详见 [`evidence/slice-geometry.md`](evidence/slice-geometry.md)。
- CSLI CREF 每条记录的两个 signed i16、SLIC `0x46` 选择、运行时图像/矩形下标、flags flip/order 以及两个相同最终 UV 通道已经闭环。
- CSLI/SLIC packed color 的零默认、双线性插值、逐通道乘法和饱和加法公式已经闭环；CAST 两种 tint 的来源仍未闭环。
- TEXL/TEX/CROP 到运行时 16 字节归一化矩形表以及 `.dds` 路径构造已经闭环；外部路径又已闭环到 `air::TextureResource`、DDS surface 描述符、D3D9 原生创建、D3DX9_43 回退和二维 SYSTEMMEM staging/`UpdateSurface`，详见 [`evidence/dds-resource-loading.md`](evidence/dds-resource-loading.md)。36 字节双 UV 顶点、四顶点 triangle strip 和最终 `DrawPrimitive` 参数也已闭环，详见 [`evidence/render-vertex-submission.md`](evidence/render-vertex-submission.md)。SrImage 的完整 render-preset 选择、特殊序列副作用、62 项 Ceylon blend 表、packet 低六位编码及最终 D3D9 blend/alpha-enable 状态也已闭环，详见 [`evidence/render-blend-state.md`](evidence/render-blend-state.md)。SrImage 原始 `+0x10/+0x14/+0x18` 分支、packet alpha/stencil 覆盖、深度 flags、comparison/stencil-op 表以及最终 D3D9 alpha/depth/stencil states 已闭环，详见 [`evidence/render-alpha-depth-stencil.md`](evidence/render-alpha-depth-stencil.md)。draw/material scissor 的两个独立 override 位、命令传递和最终 D3D9 `SCISSORTESTENABLE`/`SetScissorRect` 也已闭环，详见 [`evidence/render-scissor-state.md`](evidence/render-scissor-state.md)。DDS 内部格式转换、cube request、失败/设备丢失生命周期、基础 alpha ref/function、depth-bias、scissor 输入的更上游来源和其余 draw state 尚未闭环。
- CIMG 的 CREF/CRE1 双表保存、选择器、TEXL/CROP 坐标解析、坐标偏移、Point/Linear 选择与 Image/Text cast 分类已经闭环；TEXT 内部仍未完成。
- CNUM 的结构布局、CREF/CRE1 双表、NumberCast 初值、完整静态格式化、数字及四种特殊字符的 glyph 映射、逐字符 quad 排版和每 glyph 纹理坐标已经闭环；历史 glyph 动画与最终绘制状态仍未完成。
- CAST 专属通道 `11/12` 的尺寸/origin、`13..16` 的 packed vertex color、`17/20` 的 selector/CREF/CRE1，以及 SrRefCast 专属 `23` 的 CRFD 动画帧请求已经闭环并接入独立状态。CRFD 的 `source_name/layer_name` 已证明是在同一 SRD 的 `PROJ -> SCN  -> LAYR` 表内两级匹配，不是外部 SRD 路径；每个 RefCast 的独立复制层、顶层/复制层通道 `23` 递归动画、世界状态和递归绘制调用关系已经闭环，但实际 D3D9 draw submission 尚未实现。
- DDS 图集裁剪、runtime-bound 资源、数字排版与 sliced sprite 均不得依据旧预览器的表现直接实现。

## 待验证问题

1. VTBF/SRFF 文件头、块长度、子块布局和全部属性编码。
2. 各标签的构造/解析分派函数及运行时对象类型。
3. 引用层根 CAST 的附加抑制条件，以及递归绘制进入实际 draw submission 后的完整状态恢复。
4. 投影屏幕矩阵中另一个 backend context 的逆/组合变换，以及 3D 深度、裁剪提交逻辑。
5. CNUM 历史 glyph 动画、TEXT 与 shader/固定管线中的双 UV 消费流程。
6. CNUM 历史 glyph 动画、CAST tint/default color 与最终 D3D9 绘制语义；ANIM/TRK 已证明没有独立的 Number 数值目标。
