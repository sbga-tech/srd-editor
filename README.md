# SRD Editor

SEGA Surfride `.srd` 文件的离线解析、预览与编辑工具。

## 已确认的项目目标

- 使用 Rust 解析 SRD，并只对已证明可安全修改的既有 TRS2/TRS3 字段做原位写回；未知字节保持不变。
- 使用 `iced 0.14` 的 `wgpu` renderer 构建跨平台桌面编辑界面，不保留 tiny-skia CPU fallback。
- macOS、Windows 与 Linux 使用同一原生 WebGPU SRD/Fennel 后端；UI 不持有 device、surface 或 GPU resource。
- 按构建宿主机的原生指令集发布，不把编辑器绑定到原游戏的 32 位 x86 架构。
- 不依赖 D3D9、D3DX、DXVK、Cg 或运行时 shader 翻译。纹理由独立库解码，GPU 只接收原生语义 draw state，并执行一份 pipeline-specialized Ceylon SimpleShader WGSL 模块。
- 支持纹理图集、裁剪、节点层级、动画、文本与常见 CAST 类型。
- 保留原始字节和未知字段，避免编辑时破坏尚未还原的结构。
- 通过离线样本和 IDA 数据库验证，不启动游戏或 `amdaemon`。

旧 Python/PySide6 项目位于相邻的 `WORK/srd_editor`，只能用于提供调查线索、样本路径和待核对问题。旧实现与旧文档中的字段含义、动画公式和渲染规则都不能直接移植；本仓库只接受能够由游戏二进制及样本闭环证明的逻辑。

## 当前状态

已经实现并验证：

- VTBF/SRFF 结构解析与受约束的无损写回：未知头字段、未编辑属性及其原始编码保持不变；只修改已经存在且可按原类型编码的 TRS2/TRS3 translation、rotation、scale 属性。
- `ANIM → MOT → TRK → KEY` 记录读取。
- `SCN -> ANMS -> SANM` 场景动画集：逐 LAYR enable gate、命名 ANIM、初始 frame 与 runtime duration；正常 Composition 只提交显式选择的动画集，不再同时绘制互斥页面。
- CIMG/TEXT 与 PROJ FONT/CHAR 记录解析、TEXT 到同一项目 FONT 下标解析，以及 SrTextCast 的实际构造/字符串初始化。完整语料的字体来自 `A000/font/*.rfz`；BinaryLZW `YS` v2、YABX schema/CRC/对象 ID、`RHFONTDB` Database/Glyph/TextureResource 和 AVTS 目录均已按游戏读取逻辑实现。六套字体的全部 glyph 引用与内嵌 DDS 页已闭环；不会用系统字体伪装游戏字形。
- 字体 AVTS 不重写贴图解码：目录中的 `.svo` 是 Stevia YABX 元数据，其余条目是完整 DDS。当前六套字体共 15 个 A4R4G4B4 atlas 页，尺寸、末页高度、mip、数据长度与 Ruhuna Database 全部交叉验证，并直接复用现有 DDS 解析、独立解码与 GPU 上传路径。
- RFZ/Fennel 已实现游戏 UTF-8、当前完整语料所需控制 token，以及二进制已闭合的 `$s/$S` effect toggle 和 `$C/$c` 四角颜色状态、128 字节 runtime glyph 到 116 字节 layout record、默认静态 `sub_7C1F90` 的 auto-fit/自动断行/固定字符表/空格候选/对齐/垂直 `-254` 截止、`sub_7C0D40` 的记录过滤与 17 桶 atlas batch 前向链顺序、`sub_7C7F90` normal/effect glyph 的 bearing origin/effective scale、2D CPU matrix/3D packet matrix、effect RGB 替换与 alpha 相乘/effect-first buffer 分段，以及 `sub_7C10B0` 的无裁剪和局部矩形裁剪/UV 重映射两套 28 字节 format 13 glyph 顶点。CATR `FontParamData` 已按原记录顺序解析，实际首帧 mode `0/2/4`、post-mode 低位、monospaced `0x200`、clip size、24 字节 FontObject style 到 RFZ record flags、`$[0]..$[7]`/`$D/$L` 文本预处理、scroll 状态、`sub_7BFAB0` mode-0 几何测量、`sub_7C04F0` 循环位移/fit guard 和 shadow effect 均已闭合；draw-list 已按最终原串/循环串/二次排版对象状态接线，并提供显式 per-node runtime API 接收 8 个替换槽、default D、repeat-space count 和当前 `F4`。右侧 Properties 已能按 TextCast 保存这些手动输入，并明确保持 `F4` 与 timeline frame 独立。project layer/Cast vector/RefCast 已按引用 NODE 位置立即递归，copied layer 的独立 Image/Text draw、runtime gate、混合 CAST 调用序列和 target planner 均已接线；手动文本输入以独立 runtime owner/node 为键。point/style 对 RFZ glyph lookup 的实际非消费链也已闭合。mode-6 显式 setter 按 `0xADA300..0xADA348` 固化，但尚无游戏内部调用点。原始 shader/DrawPacket/sampler 与 D3D9 triangle-list renderer、SrTextCast 初始 world/color、零颜色跳过门控和 ShapeEnv material 的 `D3DCULL_CW` 基础状态也已接入 Composition。特殊 CAST matrix 已进入同一 runtime world composition；完整语料构造初态产生 557 个可见 2D Fennel draw、35466 个顶点，而 `LinkedVERSE_Gate` 的 `ANMS[10] frame 1` 已通过真实可达 3D TextCast fixture，产生 498 个字体顶点。其实际宿主已闭环为 SurfFile id 84、空 TargetScene、identity FirstCalc、`2DLayer=70`/root key `0xC680`；最终像素仍等待 MainScene 运行时 Camera 写入链。Advertise 样本没有触发新增状态，强制 `ResetEx` 前后仍为 40920 个 changed pixels、bbox `(651,396)..(1271,683)`、`white_pixels=0` 且哈希一致；没有用系统字体、通用 Unicode 断行或启发式布局替代。
- 游戏标量轨道的时间区间、端点、线性、保持和三次曲线求值。
- LAYR、NODE、TRS2/TRS3 记录读取、2D/3D flags 分派和首子/同级层级构建。
- `SRFF -> SRCK -> PROJ -> SCN  -> LAYR` 项目场景表，以及 CRFD 在同文件 SCN/LAYR 表中的首次完整名称解析。
- `PROJ -> CAM ` 的 position/target/angle/near/far、`SCN 0x40/0x41` composition 尺寸、全局 `sea::Camera` 的 RH View/Perspective/`Projection*View`，以及 Simple shader `mtxPrjView` 常量 provider。
- 公共空间动画通道、游戏自有 sin/cos 近似、局部 3x4 仿射矩阵和 `parent_world * local` 乘法。
- CSLI/SLIC 记录、`surfride::SrSliceCast` 链接、尺寸/origin 计算、网格单元生成、NODE `0x32` 父级单元索引及其完整偏移链。
- SrSliceCast active 单元的局部四顶点、2D/3D Y 轴分支、36 字节游戏顶点顺序、CSLI CREF 选择、flags flip/order 与两个相同最终 UV 通道。
- TEXL/TEX/CROP 的 540 字节记录、外部 DDS 基础路径、纹理尺寸、16 字节归一化矩形表，以及 CREF 到实际矩形的解析。
- SRD 路径到 `air::TextureResource` 的资源工厂链、DDS header/格式/mip/cube/palette surface 布局、D3D9 原生创建、游戏中的 D3DX9_43 回退分支和二维 SYSTEMMEM staging/`UpdateSurface` 参数；旧 97 文件、完整 `surfboard` 的 360 个 DDS，以及整个游戏 `data` 的 14,694 个 DDS 均已全量回归。全游戏语料只有 A8R8G8B8/DXT1/DXT5，7,204 个文件会走原游戏 D3DX 回退；编辑器自身不链接或调用 D3DX。
- 编辑器 DDS 后端使用后端无关 source loader，把支持的 DDS mip 解码为 RGBA8 WebGPU texture；`image_dds 0.7.2` 的纯 Rust BC1/BC3 decoder 处理原游戏走 D3DX fallback 的 NPOT DXT 文件，A4R4G4B4 字体页也显式展开为 RGBA8。
- TEX `0x62` 到 Wrap/Clamp、Linear/Point 双包装对象及最终 D3D9 sampler state 的完整绑定链。
- CAST `CATL/CATR` 通用属性列表、`ExtParamData` 12 字节运行时结构、blend preset 覆盖和继承式层级键。
- SrSliceCast 两个 packed vertex color 的解析零默认、双线性 CSLI 插值、逐通道乘法和饱和加法组合器；未证明的 CAST tint 保持为显式输入。
- `surfride::SrPlayer -> SrPlayer::Impl -> SrRenderer` 对象链、CAM 到 Camera 的运行时传递、精确 4x4 乘法、Width/Height 视口矩阵以及 2D/3D CAST 最终屏幕 X/Y 映射。
- 完整游戏 `surfboard` 下 91 个 SRD 的结构解析回归。
- 91 个样本中的 263 个 SCN 和 1299 个 CRFD 引用目标回归。
- 91 个样本中的 983 个 CSLI 和 25 个实际父级单元索引关系回归。
- avatar 样本 MOT target、公共 rotation Z 通道和游戏三次曲线结果回归。
- 原游戏的 36 字节顶点、triangle strip/list、混合、alpha/depth/stencil、scissor 和 sampler 行为已分别编译为 `SrdVertex`、`DrawTopology`、`SrdPipelineState`、`SrdMaterial` 与 `SrdQueueState`。
- ShapeEnv、SimpleShaderSelector、draw-packet bitfield 和 D3D9 常量寄存器仅保留在证据文档中；运行时 draw 不携带 compact key、packet、寄存器数组或 D3D9 枚举。
- 原始 selector 对纹理数、双纹理模式、surface mode 与 Fennel 的已实现可见语义收敛为 `SimpleShaderProfile`，其中真正影响 shader 程序的字段进一步形成 pipeline-specialized `SimpleShaderProgram`；原生 surface contract 只接受已证明的 render preset `0..21` 与 `33..61`，未证明的 `22..32` 不会落入通用 WGSL fallback。
- GPU 按原 Cg 运算次序分别接收 `mtxWorld`、`mtxPrjView`、`screenParam` 和数值 material 参数；2D half-pixel 分支与 3D `mtxPrjView * (mtxWorld * vertex)` 都在 SimpleShader 内执行，纹理数、双纹理模式、transform mode 与 blend mode 则在 pipeline 创建时固化为 WGSL overrides。
- blend/raster/depth/stencil 以语义枚举进入 WebGPU pipeline cache；alpha-test 是显式 material 参数，不再构造或反解 D3D9 packet。
- CPU draw stream 保留来源、顺序、quad/Fennel geometry、语义状态与纹理绑定；target filter/pass 排序和相邻合并后再物化 strip connector 或 triangle list。
- WebGPU 后端使用 arena 复用 uniform/vertex buffer，并在 composition 编码前准备、缓存 pipeline/sampler/bind group。普通 draw 保持在连续 render pass 中；需要 `textureTargetColor` 的动态 draw 会结束当前 pass、复制当时的 RGBA attachment，再以 `Load` 恢复同一 color/depth-stencil attachment，因而严格保留 draw 顺序且不会形成读写反馈环。内建硬件回归逐项验证 Cg `multiTexBlned` 的 13 个 mode、target-color mode `33..61`、连续 backdrop snapshot 及普通 draw/readback；设置 `GAME_DATA_CORPUS` 后的 smoke 另把 shader ExtParam fixture 编译出的 54 组真实 material/texture 状态排成确定性 gallery 后提交。
- 历史离线 extractor 曾从游戏恢复并验证 Cg/SM3 shader collection；结果仍作为语义证据，但 extractor、SM3/SPIR-V asset、82-key lookup、build-time translator 与 D3D9 runtime 均已从编辑器删除。
- 可运行的 `iced` 桌面编辑器：顶栏、可调整四面板工作区、按 NODE `firstChild`/`nextSibling` 关系展开且每个 CAST 分支可独立折叠的 Assets 树、Composition 视口、按 CAST 类型切换的 Transform/CAST payload Inspector 与单一 Animation Timeline 均由原生 widget 和 canvas 构成。RefCast 在原位展开目标 LAYR；循环引用会截断，搜索结果保留祖先路径。
- UI 状态集中在 `EditorModel`：场景/可选动画预设/层/CAST 选择、显隐与锁定、播放帧、CAST 折叠状态、工具模式、运行时文本输入、撤销/重做和脏状态均通过显式 `EditorAction` 更新；Preview target 不再是可变编辑器状态。
- SCN 的 ANMS 是 Timeline 中独立的动画预设，不是 Assets 层级的子资产。`Base pose · no animation preset` 是一等预览状态，直接使用序列化 CAST 状态；前景与 Common background 可分别选择 Base Pose。
- Transform Inspector 在动画预设生效时显示当前帧求值后的 runtime TRS/tint/alpha，并禁用写回；切换到 Base Pose 后恢复序列化 TRS 编辑。CAST 选择框来自通过 pass/filter 后实际提交的 quad/Fennel 几何，合并所选 CAST、结构子孙和 RefCast 实例后裁剪到 composition，而不是使用固定编辑器矩形。
- `PreviewCoordinator` 在三平台创建 WebGPU；初始化失败时报告 `GPU preview unavailable`，不降级到 CPU、D3D9 或软件 adapter。
- Composition 支持网格、safe-area、透明度提示、50%–200% 实际画布缩放、当前 backend/能力状态及显式 Common background 下层 SRD。交互编辑器固定使用 MainScene 路由（内部沿用已验证的 AdvertiseLogo pass/filter/layer 常量），不再暴露 `Preview target` 选择器；该路由是编辑器宿主上下文，不伪装成 SRD 文件属性。前景与背景始终各自按当前 SCN composition aspect 使用其项目内嵌 `PROJ -> CAM `；缺少 `CAM ` 时使用 position/target/angle/near/far 全零的精确 fallback。两层尺寸不一致时返回错误，不擅自缩放。
- 文档保存以源文件为基线，只原位 patch 已证明的 TRS2/TRS3 position、rotation、scale；Save/Save As、原子替换、100 项撤销历史与 dirty 状态均有回归测试。材质颜色、opacity、anchor/skew 和运行时宿主文本不会伪装成已序列化字段。
- 2D 与 3D runtime draw 均已接入原生 `SrdTransform`：CPU 保留独立 world、projection-view 与 target screen 参数，shader 按恢复的 Cg 运算顺序执行 2D half-pixel 或 3D 矩阵分支；运行时不再读取 ShapeEnv key 或 D3D9 常量槽。
- Chusan `AdvertiseLogoObject` 的成员、属性与 identity `FirstCalcMatrix` 已闭环；property 2 字符串保持为空。Scene map 空构造、唯一插入入口、18 个注册点和完整 293 个 AFB 类型键审计证明该精确 lookup 返回 null，packet 进入全局队列。
- Chusan `CommonBackGroundObject` 的嵌入式 SrPlayer、identity 根节点、启用生命周期与 `2DLayer=6` 已闭环。命名 target 成功解析时，Rust 现已复现 `SrRenderer+0x24C..+0x258`、四角屏幕投影和 inclusive AABB Image/Fennel 剔除；null target 时原二进制无条件读取从未初始化的这四个 f32，Rust 为可重复预览显式跳过该项剔除。Common/MainScene 的全屏 D3D9Ex 回归哈希保持不变。
- AdvertiseLogo 的实际六阶段 SrCtrl identity 表已解码到 SRD 的 ANMS 下标；`AS_warning_in`、`AS_movie_in` 等页面现在按 SANM gate 和命名动画生成 draw。原先全白输出已由 D3D9Ex 回读定位并修复：页面 smoke 的 2,073,600 个 changed pixels 中 `white_pixels=0`，ResetEx 前后哈希一致。

尚未实现：任意 SRD 结构的完整写回（当前只原位 patch 既有 TRS2/TRS3 的 position、rotation、scale）、公共 packed color/alpha 通道、CNUM 宿主数值事件与 transition delta、shipped 数据未出现的 CNUM mode `2..8` effect、TEXT 动态 mode `2..5` 的写入源及 mode 6 的真实调用点、texture batch rehash（当前完整语料不触发）、完整语料未出现的 DDS 内部格式转换/cube request、显式纹理 override、未证明的 render preset `22..32`，以及尚未闭环的 scene/pass host context。preset `33..61` 的 target-color 公式、attachment snapshot 与固定 pipeline 状态已经实现；仍不把未知 provider 或 `22..32` 映射为近似 shader variant。CNUM mode 1 的双 history 对齐、方向、标点插值、上下擦除及仅第一套 UV 裁剪已实现为显式运行时计划；未知宿主输入不会由 timeline 猜测。贴图像素解码由独立库完成，全程不依赖 D3DX。

## Editor architecture

依赖方向固定为：

```text
main.rs
  -> editor/mod.rs                   应用、窗口、快捷键、文件对话框、pane-grid、截图
       -> editor/ui.rs               纯 iced widget/canvas 视图
       -> editor/model.rs            action、选择、播放、history、editor-only runtime 输入
       -> document.rs                SRD 加载、源字节基线、已证明字段的原位写回
       -> renderer/mod.rs            PreviewCoordinator 与平台无关 request/frame 契约
            -> renderer/preview/     runtime evaluation、资源生命周期、Composition 编排
            -> renderer/pipeline/    原生 geometry、shader/material/pipeline/queue 状态
            -> renderer/backend/     GPU resource 与 draw submission 边界
            -> renderer/resources/   后端无关 DDS/Fennel source loading
            -> renderer/wgpu/        WebGPU pipeline/cache/upload/readback
```

- `editor/ui.rs` 不解析 SRD、不打开文件、不持有 renderer。每个可交互控件只发送 `Message` / `EditorAction`。
- `editor/model.rs` 区分序列化项目状态与 editor-only 状态。只有项目字节可表达的 TRS 修改会令文档变脏；layer visibility、Common background、runtime text substitution 和预览工具不会污染 SRD。
- `renderer/mod.rs` 构造前景与可选 Common background 两个独立 runtime layer。两层分别借用 document、scene/可选 ANMS/frame、hidden-layer 集合和 runtime text input，不复制整个文档；未选 ANMS 时构造序列化 Base Pose，提交顺序固定为 background → foreground。
- `renderer/pipeline/state.rs` 是唯一 draw vocabulary；编译器只产生语义状态，WebGPU adapter 不读取游戏 packet、selector key 或 D3D9 register layout。
- renderer 直接在 iced 主线程运行；资源 cache 以 document revision 为键，UI 只接收回读后的 `PreviewFrame`。预览使用独立的 `wgpu` device，不向 editor model 泄漏 GPU 类型。

### Native WebGPU backend

- macOS、Windows 与 Linux 运行同一实现；wgpu 按平台选择 Metal、Vulkan 或其他可用原生 API。
- 单一 `simple.wgsl` 通过 `vs_format_14` 与 `vs_format_13` 两个 vertex declaration entrypoint 分别接收 Surface 36-byte 与 Fennel 28-byte layout，再进入同一 `fs_main`；fragment 路径执行已证明的 texture count、13 个精确双纹理公式、surface mode `0/9`、alpha-test，以及 target-color mode `33..61`。`33..60` 读取逐 draw 的 composition snapshot；`61` 输出保留 source alpha 的白色并使用预设 3 的固定 blend。
- Pipeline key 包含 pipeline-specialized `SimpleShaderProgram`、vertex layout、topology、blend、raster、depth 与 stencil；alpha-test、矩阵、数值 material 参数与 sampler 保留在 uniform/bind group。固定 alpha blend 不进入 shader program identity，但仍由 blend state 区分 pipeline。
- WebGPU 初始化失败时显示 `GPU preview unavailable` 与具体原因；不切换到 CPU rasterizer、D3D9、DXVK 或软件 adapter。
- Iced UI 与 SRD preview 各自持有独立 `wgpu` device；这是清晰的所有权边界，不是文件格式语义。


不传路径时编辑器以空项目状态启动；也可以显式加载一个文件：

```text
cargo run
cargo run -- /path/to/project.srd
```


验证本机与 Windows 编译、模型/renderer 契约以及格式：

```text
cargo check --all-targets
cargo check --all-targets --target x86_64-pc-windows-msvc
cargo test
cargo fmt --all --check
```

依赖游戏数据语料的回归统一从 `GAME_DATA_CORPUS` 读取完整 `data` 根目录；未设置时这些语料测试会跳过，不会探测仓库外的相邻目录：

```text
GAME_DATA_CORPUS=/path/to/data cargo test
```

设置 `SRD_EDITOR_SCREENSHOT=/path/to/output.png` 后启动编辑器，会在窗口稳定后保存实际 iced framebuffer，用于布局 smoke/视觉回归。

运行时对象关系与生命周期见 [`docs/runtime-architecture.md`](docs/runtime-architecture.md)；SRD 格式和渲染证据索引见 [`docs/srd-format.md`](docs/srd-format.md)；尚未闭环且不得进入实现的问题见 [`docs/TODO.md`](docs/TODO.md)。
