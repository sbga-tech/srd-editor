# 待闭环问题

本页只记录已经有明确二进制边界、但证据尚未闭合的问题。这里的候选解释不得直接进入渲染实现。

## 空 `TargetScene` 的未初始化可见性矩形

Advertise/Common 的空 property 2 已闭环为 null lookup：Scene 管理器构造时 map 为空，唯一插入入口是已审计的注册链；18 个静态注册调用点已逐个审计，正常运行时名称均为非空，`Scene List` Create 来源属于调试路径；`star::SglScene` 的工厂类型键 `0x005B916A` 在本地完整 293 个 AFB 中零命中，而同一原始序列化路径的正向对照 `SglInstancingModel` 键 `0x005B9172` 有 35 次命中。详见 [`evidence/render-visibility-culling.md`](evidence/render-visibility-culling.md)。

剩余未闭环的是原二进制自身的未初始化状态：`SrRenderer+0x24C..+0x258` 只在 target 非空时由 Width/Height 写入，构造器和 `_aligned_malloc` 分配链都不初始化它；`sub_AC6660` 却在每个 ImageCast 前无条件读取该矩形。Rust 的 null-target 路径目前确定性地跳过此剔除，不会伪造默认矩形；这项宿主策略不能冒充原二进制对任意堆历史的逐位复现。

## Chusan target 的后续生命周期

仍未闭合的是游戏模式切换之后 MainScene/BgScene Enable、manager current-target、Camera/ProjectionView 后续写入、其他专用 target 的注册/移除和当帧组合时序。当前 profile 只描述已证明的构造完成状态，不外推到所有运行阶段。`PlayLinkedVerseGateObject` 的资源 id 84、空 TargetScene、identity FirstCalc、DrawMask 与 `2DLayer=70` 已闭环；其 3D TextCast CPU/packet 路径也有真实 ANMS fixture。但 MainScene 构造相机会把该文字投到 1920x1080 画面下方，并会把 Common background 的中央内容放大裁切；用户提供的实机画面对照明确否定了把构造相机当作运行时默认值。最终像素基线必须等待 Camera 后续写入链闭环。

## 原生 shader parity 边界

当前 WebGPU surface contract 已执行 render preset `0..21` 与 `33..61`。动态路径直接复现 `EXSSF_PS_BLEND` 的 Photoshop blend、加法、两种 Gaussian、super-Gaussian、RG refraction 和 white-fill 公式；`33..60` 在每个 draw 前复制有序 composition color attachment，`61` 使用预设 3 的固定 blend。完整游戏语料中的 CATR override `34..58`、`60` 均可编译；shader ExtParam fixture 编译出的 54 组真实 material/texture 状态在替换未闭环的宿主 transform、排成确定性 gallery 后已提交，独立硬件像素 oracle 另验证精确公式。

仍未闭环的是 preset `22..32` 的 shader/provider 语义，以及其他 scene/pass context 如何形成每帧完整 18 字节 Simple 键。只有这些状态继续在 CAST draw 编译点返回带 `SCN/LAYR/NODE` 来源的错误；设备初始化失败仍显示 `GPU preview unavailable` 与原始错误。未知 provider 不得降级成某个已实现 target-color mode，preview 也不会在剩余宿主边界闭环前标记为 reference-accurate。证据见 [`evidence/render-blend-state.md`](evidence/render-blend-state.md)、[`evidence/render-shader-source.md`](evidence/render-shader-source.md) 与 [`evidence/runtime-shader-state-audit.md`](evidence/runtime-shader-state-audit.md)。

## 编辑器交互与性能（按用户要求后置）

当前先完成 SRD 核心解析/渲染行为，以下编辑器问题暂不打断主线：

- WebGPU 的 1920×1080 Debug readback 已移除逐像素 RGBA->BGRA->RGBA 往返。Apple M5 Max/Metal 隔离 submission+readback 从约 76 ms 降到约 3.3 ms；完整 Common background steady frame 为 10.4–14.1 ms（plan 6.1–7.6 ms、draw 2.3–3.9 ms、readback 1.9–3.2 ms），Advertise/Fennel 为 4.9–7.6 ms。剩余瓶颈集中在冷帧资源/字体 atlas/pipeline 建立（对应样本约 0.40 s 与 2.71 s）、iced 主线程同步全帧回读及上传到独立 `wgpu` device；后续优化应做缓存失效细化与 GPU 内直接交接，不得降低解析或绘制精度；
- `Play` 按钮当前不会按真实时间推进 timeline frame；
- 编辑器现已固定按 SCN composition aspect 使用项目内嵌 `PROJ -> CAM `，缺少 `CAM ` 时精确使用全零 fallback；Common background 的最终游戏 host Camera/viewport 修正只影响未来实机像素对照，不再作为编辑器可选相机模式。
- Layer 创建；
- Layer 删除；
- CAST 创建；
- CAST 删除；
- NODE 复制与粘贴；
- NODE 在层级或 Layer 之间移动；
- NODE 原地复制（duplicate）。

这些问题不得通过降低解析精度、跳过 draw 或伪造固定相机来“优化”。

## CNUM 宿主数值与 transition 时钟

shipped CNUM mode 1 的双 history 对齐、方向、标点位置插值、上下几何擦除和仅 CREF V 裁剪已经闭环并实现为显式运行时计划。仍缺的是游戏宿主何时调用 `srd_number_cast_set_value_parts`、传入哪些动态数值，以及每次 `srd_update_number_glyph_transition` 的精确 delta。普通预览在这些输入未知时继续使用 fresh 单 history，不把 timeline frame 猜成 transition 时钟。mode `2..8` 在 shipped SRD 中未出现，仍不得由 mode 1 外推。

## Fennel 行元数据与剩余 effect/crop 输入

静态 `sub_7C1F90` 的自动断行、两张固定表、空格候选、二次纵向 pass、独立 TextBox `+0x108`、`-254` 标记、fresh mode 1 的 `0x08` X/Y 联动 auto-fit、fresh mode 5/6 的 `0x4000` 垂直截止关闭、flags `0x200` 的固定 cell 度量/尾部 X 居中修正、`sub_7C90A0` 尾部 record-limit 返回值，以及 TextBoxObject `+0x12C/+0x34C` 的锁步行元数据已经实现；`sub_7C0D40` 的负记录过滤、`-254` 立即停止、静态 maximum=-1、texture token 分组、normal/effect 计数和初始 17 桶前向链顺序也已实现；`sub_7C7F90` normal/effect glyph 的 bearing origin、effective scale、2D CPU matrix/3D packet matrix 顺序，以及 `sub_7C10B0` 的无裁剪/裁剪 UV 重映射两套顶点同样已实现。CATR `FontParamData` 的顺序解析、实际 mode、低位 flags、monospaced、clip 与 shadow 已接入；特殊 CAST matrix 已并入相同 runtime world composition。完整 1292 条 RFZ TEXT 的实际 mode 为 `0:1173, 2:12, 4:107`，构造初态有 557 个可见 2D draw、35466 个顶点；`LinkedVERSE_Gate` 的 `ANMS[10] frame 1` 已固定真实可达 3D TextCast 的 498 顶点与 packet matrix。Advertise 像素回归仍为 40920 个 changed pixels。

尚未闭合：

- FontResource 的 32 槽 lowest-free 分配、同资源缓存复用、最后引用释放后的精确槽回收、满表返回未注册 slot `0x20`，以及当前玩家 `SCN -> LAYR -> NODE` 中主字体后接 `rubyFont/rfzOutlineFont/rfzOutlineRubyFont` 的 CATR 请求顺序已经闭合并实现；copied reference layer 已证明不会追加请求，而是使用原始目标层已请求的共享资源。`$F[n]`/裸 `$F` 的全局 slot 解析与跨字体 atlas token 路由也已连接。仍待宿主提供的是加载当前 SRD 前仍存活的进程级 FontManager/renderer 资源状态；不得把“空 registry”或编辑器自行分配的 opaque texture handle 声称为任意原版进程的绝对映射；
- FontParam 初始 RFZ style 链已闭合：`pointX/pointY` 经 signed `max(value,1)` 和高字节清零成为 u8，但 `FontDriverRFO` 查找只读取字符码并只复制 style flags；outline/italic/bold 的 flags/value 也已复现，`faceId` 不被 `sub_AC6F50` 读取。`$[0]..$[7]` 顺序替换、`$D/$L` 大写优先/首个删除/可选括号 `atoi`、FontManager 默认 D=20 与 repeat-space count=3、`sub_AD8D00` 时钟、`sub_AC5740` 派生量、`sub_7BFAB0` mode-0 实际几何测量及完整 `sub_7C04F0` 横纵循环/fit-guard/二次排版均已实现，draw-list 已按最终 TextBox 内容接线，并有显式 per-node runtime API 接收替换槽/default/repeat/F4；动画集版本也复用 ProjectRuntime 的 layer enable、CAST transform/color 与 SrImage geometry。project layer/Cast-vector/RefCast 的结构递归顺序、完整世界/gate、copied ImageCast/TextCast 独立 draw 和两者混合的 runtime CAST 调用序列均已接通；copied 文本输入用 `(owner,node)` 作为 key，重复引用实例不会共享替换槽或 F4。target queue 的 type-1/SRD class、首个 rule 匹配、per-pass 稳定 vector、32-entry inclusive-range flush 和无 comparator 结论也已闭环并实现；MainScene/BgScene 共用的 5 项默认 BasePass 与 EntryInfo 映射也已闭环。普通 Image/Fennel packet 的 `flags_60 bit 0x2000` 已排除、`packet+0x64` 已闭环为零，默认 Back2DPass 对完整 u16 order 域恒真；逻辑 stream 现按一个 Image command/每个 Fennel texture-batch command 展平，并能在显式 MainScene/BgScene profile 下生成精确 target-local 顺序。type-1 target filter、Scene 初始 Enable/Attribute/DrawIndex 也已实现；Advertise/Common 构造完成状态均精确为 Main 接纳、Bg 拒绝，宿主根 key、CATR/NODE/RefCast 合成及普通相邻 packet 合并也已接线。尚缺后续 Enable/current-target 切换时序、其他 target、真正依赖 depth/order 的其他路径及 stencil/special-depth 合并，所以不会把该顺序冒充所有运行阶段的完整 Composition GPU packet 列表。右侧 Properties 已保存 project-layer 手动输入，并要求显式选择 Advertise/Common 宿主；initial API 碰到替换 token 会显式拒绝。仍未闭合的二进制行为是 `vertical=True` correction，以及 mode 6 非虚方法 `0xADA300..0xADA348` 的真实调用点；后者仍只有无调用引用的 jump-island thunk `0x45335F`，不得假定 SRD 首帧或动画会自动触发它；
- `sub_7C90A0` 的布局分派已闭合为 `0x20 -> sub_7C4070`、否则 `0x40 -> sub_7C5A20`、否则默认。fresh mode `1` 的 `0x000F` 默认排版、mode `2..4` 的 `0x0CA3/0x1CA3/0x2CA3` Flag20 排版、fresh mode `5/6` 的 `0x6C03/0x7C03` 默认排版，以及三个布局器共用的 flags `0x200` 固定 cell 分支均已实现。Flag20 包括相同的断行/截止/元数据状态机和 `+0x358` 严格最小 advance 下标。完整指令流已证明 `sub_7C5A20` 与 Flag20 的有效差异只有后者的 `+0x358` 初始化/两次更新，但组件内直接字段与 setter 审计仍找不到任何生成 `0x40` 的入口；因此 Flag40 的真实 flags 来源仍待闭合；
- `sub_7C8BE0` 的 hash rehash；当前完整字体最多 7 页、真实文本最多 6 个 batch，不会触发该分支。

在这些输入闭环前，不得自行补 vertical correction、滚动几何测量/宿主输入、mode-6 自动调用、`Flag40` 或 rehash 行为。
