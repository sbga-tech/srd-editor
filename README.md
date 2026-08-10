# SRD Editor

SEGA Surfride `.srd` 文件的离线解析、预览与编辑工具。

## 已确认的项目目标

- 使用 Rust 解析 SRD，并只对已证明可安全修改的既有 TRS2/TRS3 字段做原位写回；未知字节保持不变。
- 使用 `iced 0.14` 的 `wgpu` renderer 构建跨平台桌面编辑界面，不保留 tiny-skia CPU fallback。
- macOS、Windows 与 Linux 默认使用原生 WebGPU SRD/Fennel 后端；Windows/Linux 仍可通过 `SRD_PREVIEW_BACKEND=d3d9` 显式选择同一提交契约的 DXVK D3D9 参考后端。UI 不持有 device、swap chain 或 GPU resource。
- 按构建宿主机的原生指令集发布，不把编辑器绑定到原游戏的 32 位 x86 架构。
- 不依赖或调用 D3DX；纹理解码使用独立库，GPU 后端分别上传到 WebGPU 或 D3D9，着色器采用不依赖 D3DX 的、经二进制行为验证的方案。
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
- 编辑器 DDS 后端已实装：D3D9 路径让 7,490 个原生兼容文件保持 A8R8G8B8/DXT1/DXT5 surface 布局，经 SYSTEMMEM staging 和 `UpdateSurface` 上传；7,204 个 NPOT DXT fallback 由 `image_dds 0.7.2` 的纯 Rust BC1/BC3 decoder 转为 RGBA8。WebGPU 路径使用同一后端无关 source loader，把支持的 DDS mip 解码为 RGBA8 GPU texture；A4R4G4B4 字体页也显式展开为 RGBA8。
- TEX `0x62` 到 Wrap/Clamp、Linear/Point 双包装对象及最终 D3D9 sampler state 的完整绑定链。
- CAST `CATL/CATR` 通用属性列表、`ExtParamData` 12 字节运行时结构、blend preset 覆盖和继承式层级键。
- SrSliceCast 两个 packed vertex color 的解析零默认、双线性 CSLI 插值、逐通道乘法和饱和加法组合器；未证明的 CAST tint 保持为显式输入。
- `surfride::SrPlayer -> SrPlayer::Impl -> SrRenderer` 对象链、CAM 到 Camera 的运行时传递、精确 4x4 乘法、Width/Height 视口矩阵以及 2D/3D CAST 最终屏幕 X/Y 映射。
- 完整游戏 `surfboard` 下 91 个 SRD 的结构解析回归。
- 91 个样本中的 263 个 SCN 和 1299 个 CRFD 引用目标回归。
- 91 个样本中的 983 个 CSLI 和 25 个实际父级单元索引关系回归。
- avatar 样本 MOT target、公共 rotation Z 通道和游戏三次曲线结果回归。
- 36 字节 D3D9 顶点声明、四顶点非索引 triangle strip、混合、alpha/depth/stencil 与 scissor 状态提交。
- 绘制包到 64 位 ShapeEnv shader cache key 的全部位来源、CREF/CRE1 到 D3D9 texture stage 0/1 的映射、vertex-format/blend/multi-texture 模块索引，以及完整 `surfboard` 语料 19,484 个初始 image node 的实际 key 回归。
- ShaderSelector 注册顺序、SRD/ShapeEnv 对 Simple 槽位 9 的实际选择、Simple 的 18 字节键与 71 项表、Default 的 46 字节键机制、嵌入式 Cg source 的精确 dword 解码/include 闭包、format 14 的双 UV/双顶点色公式，以及 stage 0/1 到 pixel/vertex Shader resource 的映射。
- 默认及逐 packet 的 VS `c0..c9`、PS `c0` 常量提交，以及选定无贴图 fixture 的 VS `c10..c13 = Projection*View` provider。
- draw packet 到 D3D9 cull/fill/color-write 的精确覆盖：首个 fixture 为 `CULL_NONE`、`SOLID`、四通道写入，不依赖编辑器侧显示性兜底。
- 首个证据完整的 CPU draw list：`CHU_UI_System_00_v10.srd` 的 scene 0/layer 0/node 1 `C_fill` 在显式 identity `FirstCalcMatrix` 和 1920 宽目标下生成唯一无贴图 ImageCast draw，包含精确四顶点、世界色、packet、Simple key、固定常量、blend/raster/depth。原游戏 XML 的完整 82-key Simple collection 与两个 Fennel 精确 key 均已嵌入；TEXT、显式纹理 override、特殊 CAST 矩阵分支和未注册 key 不会被伪装为已支持。
- 首个真实 D3D9Ex SRD draw submission：创建 format 14 顶点声明与动态 DEFAULT-pool 顶点缓冲，上传已验证 VS/PS、VS `c0..c13`、PS `c0` 和精确 blend/raster/depth 状态，执行非索引 `D3DPT_TRIANGLESTRIP`；不可见 smoke 在 `EndScene` 后通过 `GetRenderTargetData` 回读 identity-host fixture 内部像素，并在强制 `ResetEx` 后重复验证。外部 material scissor 作为显式 context 输入，不从 SRD 猜测。
- 隔离 x86 取证工具已对完整 XML 的 82 个 Simple key 生成原版 Cg assembly，经 `D3DCompiler_47!D3DAssemble` 得到 164 份无 D3DX D3D9 bytecode，并全部由 D3D9 HAL device 成功创建 shader 对象；生成器将其确定性打包为 14 个唯一 VS、24 个唯一 PS 和完整精确 key 映射，编辑器发布物不依赖 Cg。
- 可运行的 `iced` 桌面编辑器：顶栏、可调整四面板工作区、按 NODE `firstChild`/`nextSibling` 关系展开且每个 CAST 分支可独立折叠的 Assets 树、Composition 视口、按 CAST 类型切换的 Transform/CAST payload Inspector 与单一 Animation Timeline 均由原生 widget 和 canvas 构成。RefCast 在原位展开目标 LAYR；循环引用会截断，搜索结果保留祖先路径。
- UI 状态集中在 `EditorModel`：场景/可选动画预设/层/CAST 选择、显隐与锁定、播放帧、CAST 折叠状态、工具模式、运行时文本输入、撤销/重做和脏状态均通过显式 `EditorAction` 更新；Preview target 不再是可变编辑器状态。
- SCN 的 ANMS 是 Timeline 中独立的动画预设，不是 Assets 层级的子资产。`Base pose · no animation preset` 是一等预览状态，直接使用序列化 CAST 状态；前景与 Common background 可分别选择 Base Pose。
- Transform Inspector 在动画预设生效时显示当前帧求值后的 runtime TRS/tint/alpha，并禁用写回；切换到 Base Pose 后恢复序列化 TRS 编辑。CAST 选择框来自通过 pass/filter 后实际提交的 quad/Fennel 几何，合并所选 CAST、结构子孙和 RefCast 实例后裁剪到 composition，而不是使用固定编辑器矩形。
- `PreviewCoordinator` 在三平台默认创建 WebGPU；初始化失败时报告 `GPU preview unavailable`，不再降级到 CPU renderer。Windows/Linux 上只有显式设置 `SRD_PREVIEW_BACKEND=d3d9` 才加载 DXVK 参考后端。
- Composition 支持网格、safe-area、透明度提示、50%–200% 实际画布缩放、当前 backend/能力状态及显式 Common background 下层 SRD。交互编辑器固定使用 MainScene 路由（内部沿用已验证的 AdvertiseLogo pass/filter/layer 常量），不再暴露 `Preview target` 选择器；该路由是编辑器宿主上下文，不伪装成 SRD 文件属性。前景与背景始终各自按当前 SCN composition aspect 使用其项目内嵌 `PROJ -> CAM `；缺少 `CAM ` 时使用 position/target/angle/near/far 全零的精确 fallback。两层尺寸不一致时返回错误，不擅自缩放。
- 文档保存以源文件为基线，只原位 patch 已证明的 TRS2/TRS3 position、rotation、scale；Save/Save As、原子替换、100 项撤销历史与 dirty 状态均有回归测试。材质颜色、opacity、anchor/skew 和运行时宿主文本不会伪装成已序列化字段。
- 首个二维单贴图 runtime draw 已接入：CAST 二维标志精确进入 packet `+0x60` bit 7，选择 `EAEBABBAABGAAAAAAA`/`ShapeEnv2D` VS，并把显式 target screen size 的半宽半高上传为 `c10 screenParam`。实际 AdvertiseLogo fixture 在强制 `ResetEx` 前后稳定覆盖 `(346,194)..(1573,885)`；runtime 仍不依赖 Cg、D3DX 或 D3DCompiler。
- Chusan `AdvertiseLogoObject` 的成员、属性与 identity `FirstCalcMatrix` 已闭环；property 2 字符串保持为空。Scene map 空构造、唯一插入入口、18 个注册点和完整 293 个 AFB 类型键审计证明该精确 lookup 返回 null，packet 进入全局队列。
- Chusan `CommonBackGroundObject` 的嵌入式 SrPlayer、identity 根节点、启用生命周期与 `2DLayer=6` 已闭环。命名 target 成功解析时，Rust 现已复现 `SrRenderer+0x24C..+0x258`、四角屏幕投影和 inclusive AABB Image/Fennel 剔除；null target 时原二进制无条件读取从未初始化的这四个 f32，Rust 为可重复预览显式跳过该项剔除。Common/MainScene 的全屏 D3D9Ex 回归哈希保持不变。
- AdvertiseLogo 的实际六阶段 SrCtrl identity 表已解码到 SRD 的 ANMS 下标；`AS_warning_in`、`AS_movie_in` 等页面现在按 SANM gate 和命名动画生成 draw。原先全白输出已由 D3D9Ex 回读定位并修复：页面 smoke 的 2,073,600 个 changed pixels 中 `white_pixels=0`，ResetEx 前后哈希一致。

尚未实现：任意 SRD 结构的完整写回（当前只原位 patch 既有 TRS2/TRS3 的 position、rotation、scale）、公共 packed color/alpha 通道、CNUM 宿主数值事件与 transition delta、shipped 数据未出现的 CNUM mode `2..8` effect、TEXT 动态 mode `2..5` 的写入源及 mode 6 的真实调用点、texture batch rehash（当前完整语料不触发）、完整语料未出现的 DDS 内部格式转换/cube request、显式纹理 override，以及 ShapeEnv 剩余 scene/pass context 到每次实际 Simple 键的映射。CNUM mode 1 的双 history 对齐、方向、标点插值、上下擦除及仅第一套 UV 裁剪已实现为显式运行时计划；未知宿主输入不会由 timeline 猜测。贴图像素解码由独立库完成，全程不依赖 D3DX。

## Editor architecture

依赖方向固定为：

```text
main.rs
  -> editor/mod.rs                   应用、窗口、快捷键、文件对话框、pane-grid、截图
       -> editor/ui.rs               纯 iced widget/canvas 视图
       -> editor/model.rs            action、选择、播放、history、editor-only runtime 输入
       -> document.rs                SRD 加载、源字节基线、已证明字段的原位写回
       -> renderer/mod.rs            PreviewCoordinator 与平台无关 request/frame 契约
            -> renderer/wgpu/        默认 WebGPU SRD/Fennel submission 与 RGBA readback
            -> renderer/d3d9/        Windows/Linux 可选 D3D9 参考 submission 与 RGBA readback
                 -> dxvk.rs          DXVK library、Direct3DCreate9Ex、Windows/SDL3 WSI
            -> renderer/backend.rs   两个 GPU 后端共享的资源、Composition、draw 提交边界
            -> renderer/assets.rs    后端无关 DDS/Fennel source loading
```

- `editor/ui.rs` 不解析 SRD、不打开文件、不持有 renderer。每个可交互控件只发送 `Message` / `EditorAction`。
- `editor/model.rs` 区分序列化项目状态与 editor-only 状态。只有项目字节可表达的 TRS 修改会令文档变脏；layer visibility、Common background、runtime text substitution 和预览工具不会污染 SRD。
- `renderer/mod.rs` 构造前景与可选 Common background 两个独立 runtime layer。两层分别借用 document、scene/可选 ANMS/frame、hidden-layer 集合和 runtime text input，不复制整个文档；未选 ANMS 时构造序列化 Base Pose，提交顺序固定为 background → foreground。
- 默认 WebGPU renderer 与可选 D3D9 renderer 都直接在 iced 主线程运行；资源 cache 以 document revision 为键，UI 只接收回读后的 `PreviewFrame`。
- `wgpu`、DXVK、D3D9、Vulkan/Metal、SDL window 与 GPU resource 类型分别封装在 `renderer/wgpu/` 和 `renderer/d3d9/` 内，不泄漏到 editor model。

### Optional DXVK reference runtime

默认 backend 不需要 DXVK。设置 `SRD_PREVIEW_BACKEND=d3d9` 后，Windows 和 Linux 使用同一份 D3D9 renderer，差异仅限于 DXVK WSI：Windows package 提供 `d3d9.dll` 与 `HWND`，Linux native package 提供版本化的 `libdxvk_d3d9.so.0.30002` 并以隐藏的 SDL3 Vulkan window 作为 `HWND` 等价物。`Direct3DCreate9Ex` 由 `libloading` 显式解析；Windows **不会**静默绑定系统 `d3d9.dll`。

固定 runtime 为官方 [DXVK v3.0.2](https://github.com/doitsujin/dxvk/releases/tag/v3.0.2)：

| 平台 | 官方 archive | SHA-256 | 本项目识别的主要路径 |
|---|---|---|---|
| Windows | [`dxvk-3.0.2.tar.gz`](https://github.com/doitsujin/dxvk/releases/download/v3.0.2/dxvk-3.0.2.tar.gz) | `9c538924110a7cdef871ca36dee218c0774124374ffdeb38af4b76be55bdf7c2` | `runtime/dxvk/x64/d3d9.dll`，或原样解包后的 `runtime/dxvk/dxvk-3.0.2/x64/d3d9.dll`（32 位使用 `x32`） |
| Linux | [`dxvk-native-3.0.2-steamrt-sniper.tar.gz`](https://github.com/doitsujin/dxvk/releases/download/v3.0.2/dxvk-native-3.0.2-steamrt-sniper.tar.gz) | `4a11ccf93b2d325f44d710ae86eb630721d3c232a3c333e76581adc4913112d3` | 原样解包后的 `runtime/dxvk/usr/lib/libdxvk_d3d9.so.0.30002`（32 位为 `usr/lib32`） |

显式选择 D3D9 时，runtime discovery order 固定为：

1. `SRD_EDITOR_DXVK_LIBRARY` 指定文件或已解包目录；
2. executable-relative `runtime/dxvk/`；
3. working-directory-relative `runtime/dxvk/`，供 `cargo run` 使用。

Linux D3D9 还需要 SDL3。可用 `SRD_EDITOR_SDL3_LIBRARY` 指定 `libSDL3.so.0`；否则依次检查 app-local runtime 与系统 loader。进程在 iced 启动前设置 `DXVK_WSI_DRIVER=SDL3`，随后在主线程初始化 SDL video/window 和 DXVK。默认 Cargo feature `dxvk-native` 只编译该可选路径；`--no-default-features` 明确移除它，不影响默认 WebGPU。

Backend policy:

- 默认：macOS、Windows、Linux 都先初始化 WebGPU，并运行同一组已翻译的 14 个 VS / 24 个 PS、状态映射、DDS/Fennel upload 和 Composition readback。
- WebGPU 初始化失败时显示 `GPU preview unavailable` 与具体原因；不切换到 CPU rasterizer 或软件 adapter。
- 显式 D3D9：Windows/Linux 缺少 DXVK、SDL3 或 device 时显示完整 discovery/device error；不回退到系统 D3D9，也不伪装为 reference output。

Iced UI 使用 `wgpu`，SRD preview 使用独立的 `wgpu` device；Windows/Linux 的显式 DXVK 路径只替换离屏 SRD preview backend。

不传路径时编辑器以空项目状态启动；也可以显式加载一个文件：

```text
cargo run
cargo run -- /path/to/project.srd
```

Windows/Linux 上显式选择 DXVK D3D9 参考后端：

```text
SRD_PREVIEW_BACKEND=d3d9 cargo run -- /path/to/project.srd
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
