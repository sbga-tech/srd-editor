# 编辑器 D3D9Ex、HiDPI 与 Dear ImGui 后端

> **历史实现记录。** 本页的 D3D9Ex device/readback 证据仍有效，但其中 Dear ImGui 前端与对应源码已由 `iced` 四面板编辑器替换。当前应用与 preview-adapter 边界见根目录 [`README.md`](../../README.md#editor-architecture)。

本页记录编辑器基础设施的实际实现和验证边界。它不作为 Surfride 游戏渲染语义的证据；游戏侧的顶点、状态、shader 和资源结论仍必须分别由二进制调用链闭环。

## 架构与依赖边界

编辑器按 Rust 构建宿主机的原生指令集发布，不绑定原游戏的 32 位 x86 ABI。当前 preview renderer 通过 `windows-rs` 使用 D3D9 COM ABI，但 provider 由 [`renderer/d3d9/dxvk.rs`](../../src/renderer/d3d9/dxvk.rs) 通过 `libloading` 显式提供，不静默绑定系统 `d3d9.dll`：

- Windows 加载官方 DXVK package 的 `d3d9.dll`，以 desktop `HWND` 创建 device；
- Linux 加载官方 DXVK Native `libdxvk_d3d9.so.0.30002`，以主线程创建的隐藏 SDL3 Vulkan window 作为 WSI handle；
- 两个平台调用同一个 `Direct3DCreate9Ex(D3D_SDK_VERSION)`、`IDirect3D9Ex::CreateDeviceEx`、submission 与 RGBA readback 实现；
- 默认 adapter、`D3DDEVTYPE_HAL`、windowed discard swap chain；
- 优先 hardware vertex processing，创建失败后退到 software vertex processing；
- `D3DFMT_D24S8` 自动 depth/stencil；
- `D3DPRESENT_INTERVAL_ONE`。

运行时不链接、不加载也不调用 D3DX 或 NVIDIA Cg。Cg 仅存在于隔离的离线 shader 取证流程中，详见 [`render-shader-bytecode.md`](render-shader-bytecode.md)。DXVK runtime 的固定版本、校验和、discovery order、Linux SDL3 依赖与平台失败策略见根目录 [`README.md`](../../README.md#dxvk-runtime)。

初始化还会把首个证据闭环 fixture 的嵌入式 `vs_3_0/ps_3_0` token 直接交给同一个 `IDirect3DDevice9Ex::CreateVertexShader/CreatePixelShader`。创建失败由 preview adapter 边界报告为 unavailable error；不会降级到 CPU，也不会伪装成 reference-accurate output。

## Dear ImGui renderer

`src/imgui_dx9.rs` 是针对 imgui-rs draw data 的原生 Rust D3D9 renderer，接收 D3D9Ex device 的 `IDirect3DDevice9` 基接口，行为以 Dear ImGui 官方 DX9 backend 为基础。实现包括：

- D3D9 fixed-function pipeline，不需要 UI shader；
- ImGui 顶点到 XYZ、diffuse、UV 动态 vertex buffer 的转换；
- 16/32 位动态 index buffer；
- display position、framebuffer scale、每条命令的 vertex/index offset 和 scissor；
- RGBA 字体图集到 `A8R8G8B8` 的 BGRA 字节转换；
- 半像素偏移的正交投影；
- state block 与 world/view/projection 的保存、恢复；
- texture ID 注册表，现已用于 Composition 的 D3D9 render-target texture 显示。

这一后端只负责编辑器 UI。它的 fixed-function 状态不能代替 SRD 的游戏 shader 与 draw packet 状态。

## D3D9Ex 状态与 ResetEx

窗口 resize 会先记录新的物理 backbuffer 尺寸并标记 reset pending。没有待处理 resize 时，每帧通过 `IDirect3DDevice9Ex::CheckDeviceState` 检查目标窗口状态；实现不再调用传统 D3D9 的 `TestCooperativeLevel`：

1. device lost 时不提交或 `PresentEx`；
2. resize 或 DPI 变化需要 reset 时，先释放 ImGui 的 DEFAULT-pool vertex/index buffer 与字体纹理；
3. 调用 `IDirect3DDevice9Ex::ResetEx`；
4. 重新创建字体纹理，其他动态缓冲在下一帧按需创建；
5. 成功后继续 `Clear`、`BeginScene`、ImGui draw、`EndScene`、`PresentEx`。

后续 SRD GPU 资源接入同一生命周期时，也必须按其 pool 和游戏证据分别注册失效/重建，不能仅假设 ImGui 的处理足够。

## HiDPI

窗口初始尺寸使用 winit 逻辑单位，D3D9Ex backbuffer 始终使用 `Window::inner_size()` 返回的物理像素。`imgui-winit-support` 的 `HiDpiMode::Default` 维护逻辑 `display_size`、实际 `display_framebuffer_scale`、鼠标坐标和 `ScaleFactorChanged`。

默认字体不是由 GPU 放大低分辨率图集：编辑器以 `13 * dpi_factor` 的实际像素尺寸重新栅格化字体，并以 `font_global_scale = 1 / dpi_factor` 保持 13 个逻辑单位的版面高度。跨显示器 DPI 变化时，事件处理会：

1. 释放当前 ImGui font/VB/IB DEFAULT-pool 资源；
2. 用新 DPI 清空并重建字体图集；
3. 记录窗口新的物理 backbuffer 尺寸；
4. 在下一帧执行 `ResetEx` 并重新上传字体纹理。

面板宽度、timeline 固定列和 ImGui style 数值保持逻辑单位，由 framebuffer scale 映射到物理像素，因此不会再额外 `scale_all_sizes` 造成双重缩放。

Windows 上 winit 优先调用 Per-Monitor V2 DPI awareness，系统不支持时依次降级到 Per-Monitor V1/旧 API。不可见 smoke 每帧还实际断言：winit window scale、`WinitPlatform` scale 与 ImGui `display_framebuffer_scale` 相等，并且 `display_size × framebuffer_scale` 在一像素容差内等于 D3D9Ex 物理 backbuffer 尺寸。因此 HiDPI 契约不是仅凭配置项推断。

## 工作区与真实文档路径

编辑器启用 Dear ImGui docking，默认布局为：

- 中央 `Composition`；
- 左侧 `Project` 与 `Scene & Status`；
- 右侧 `Properties`；
- 下方 `Layers & Timeline`，固定的层/状态列和可横向滚动的时间轴位于同一张 table，因此共享垂直滚动和行选择。

命令行第一个非选项参数作为 SRD 路径。文档加载使用当前 Rust `SrdFile`、`Project` 和 `TextureList` 解析器；面板按 NODE 的 `firstChild`/`nextSibling` 关系显示真实 scene、ANMS/SANM、layer/CAST 层级，变换、纹理与动画 frame 仍来自解析结构。Properties 的 ANMS 选择器和下方 frame slider 会重建当前运行时页面；Composition 不再把同一 SCN 的所有互斥 LAYR 同时提交。普通预览重建生成统一 Image/SliceCell/NumberGlyph/Fennel runtime draw，并经 target filter、scene pass planner 与相邻 merge planner 后在同一个 D3D9Ex Composition pass 内按 source 顺序提交。交互编辑器固定使用 MainScene 路由，内部沿用已验证的 AdvertiseLogo pass/filter/layer 常量，不再暴露 `Preview target` 选择器；reference/corpus 测试仍可显式传入其他已还原 profile。Composition 始终用当前 SRD 项目内嵌 `PROJ -> CAM ` 覆盖固定 host context 的 target `Projection*View`，缺少 `CAM ` 时精确保留全零 camera fallback。

Properties 还可显式加载第二个 Common background SRD，分别选择其 SCN/ANMS/frame。双文档各自构造 DDS、RFZ atlas、runtime stream 与 target submission，Composition 只清屏一次并先提交 Common、再提交前景；ResetEx 同时重建两组资源。编辑器不会根据前景文件名自动猜背景，且两个 SCN 尺寸不一致时直接拒绝未经证明的缩放。对象列表证据和边界见 [`chusan-common-foreground-composition.md`](chusan-common-foreground-composition.md)。

Composition texture 使用当前 D3D9Ex backbuffer 的实际格式创建，属于编辑器集成选择，不被表述为 SRD 文件格式语义。面板以 ImGui 逻辑坐标计算保持宽高比的居中显示矩形，renderer 再使用 `display_framebuffer_scale` 把顶点和 scissor 转为物理像素。因此跨显示器 DPI 变化只改变 UI 栅格化与显示尺寸，不改变固定 SCN 像素纹理、CAM 或 `FirstCalcMatrix`。

## 本机验证

2026-08-04 在 `aarch64-pc-windows-gnullvm` Rust host 上执行：

```powershell
cargo run -- --d3d9ex-smoke
cargo run -- --d3d9ex-smoke "D:\sdhd\assets\data\surfboard\system\CHU_UI_System_00_v10.srd"
cargo run -- --srd-draw-smoke "D:\sdhd\assets\data\surfboard\system\CHU_UI_System_00_v10.srd"
cargo run --release -- --srd-texture-smoke --advertise-logo-host=MainScene@1080x1920@1920x1080 "D:\sdhd\assets\data\surfboard\advertise\CHU_UI_Advertise_00_v10.srd"
cargo run -- --srd-fennel-smoke "D:\sdhd\assets\data\surfboard\advertise\CHU_UI_Advertise_00_v10.srd"
cargo run -- "D:\sdhd\assets\data\surfboard\advertise\CHU_UI_Advertise_00_v10.srd" --srd-runtime-smoke=0,0,24 --advertise-logo-host=MainScene@1080x1920@1920x1080
cargo run -- "D:\sdhd\assets\data\surfboard\common\time\CHU_UI_Time_00_v10.srd" --srd-runtime-smoke=0,0,0 --common-background-host=MainScene@1080x1920@1920x1080
cargo run -- "D:\sdhd\assets\data\surfboard\advertise\CHU_UI_Advertise_00_v10.srd" --srd-runtime-smoke=0,0,24 --advertise-logo-host=MainScene@1080x1920@1920x1080 --common-background-layer="D:\sdhd\assets\data\surfboard\common\commonBackGround\CHU_UI_Common_BK_00_v11.srd"
```

这些命令均通过 `Direct3DCreate9Ex`/`CreateDeviceEx` 成功创建真实 D3D9Ex HAL device，并由该 device 成功创建嵌入式 SRD VS/PS；随后生成并以 `PresentEx` 提交第一帧 ImGui draw data，再显式触发 resize/`ResetEx` 路径，重建 DEFAULT-pool UI 资源并成功提交第二帧。AdvertiseLogo 动画集 draw smoke 在 reset 前后均得到 2,073,600 个 changed pixels、`white_pixels=0` 和 FNV-1a `97D30483E5DD6325`；二维贴图 smoke 得到 849,776 个一致像素、`white_pixels=0` 和 `09DF61BBE19B88A5`；独立 RFZ/Fennel smoke 得到 40,920 个一致像素、`white_pixels=0`、bbox `(651,396)..(1271,683)` 和 `7B466FC4B4E0EC9A`。截图对应的正常 runtime smoke（scene 0 / ANMS 0 / frame 24 / AdvertiseLogo MainScene）得到 22 个 source、2 个 target group、20 个 Fennel source/3072 字体顶点；把 Fennel source 关闭后前后两帧均有 73,588 个 RGB 像素变化。Time runtime smoke（scene 0 / ANMS 0 / frame 0 / CommonBackground MainScene）得到 10 个 draw、4 个 target group、3 个 NumberGlyph source；关闭 NumberGlyph 后前后两帧均有 2,369 个 RGB 像素变化。双层 smoke 中 Common `yellow_loop` 单独改变 2,073,600 个 RGB 像素；warning 前景完全遮住它，因此最终可见差分为 0，但三次独立回读证明两条 stream 都已执行，ResetEx 前后相同。动画集证据见 [`scene-animation-sets.md`](scene-animation-sets.md)，2D shader/常量见 [`render-shape-env-2d.md`](render-shape-env-2d.md)，字体路径见 [`text-font-records.md`](text-font-records.md)，数字路径见 [`cnum-number-cast.md`](cnum-number-cast.md)。

同次检查后继续加入动画集、TEXT/FONT、证据完整 draw-list、贴图 draw、SliceCell、NumberGlyph、Fennel draw、特殊 CAST matrix、完整 DDS 解码路径和 reference runtime 世界/Image/Text draw 语料测试；当前测试集为 239 个单元测试及 39 个本地语料测试，其中完整游戏数据根为 `D:\sdhd\assets\data`。新增的跨字体专项以真实 RFZ 构造 `$F[n]` 切换和裸 `$F` 复位，验证 batch token 路由到各自 atlas；Fennel 专项还覆盖 `$[0]..$[7]`、`$D/$L`、辅助重复串、`sub_7BFAB0` mode-0 几何测量以及 `sub_7C04F0` 横纵位移/fit guard。reference 专项同时覆盖完整 91 文件中的 2365 个 copied layer 世界状态、命名-target 剔除和特殊矩阵合成后保留的 671 个 copied Fennel draw，以及 owner-aware Image/Text CAST 调用序列。Slice 专项覆盖 4,884 个 runtime cell draw、七个精确 shader key 和 Common 的 100 Image + 141 SliceCell 相邻 merge；Number 专项覆盖 2,696 个 glyph draw、2,223 个 copied glyph、mode 0/1 及三个精确单贴图 key。Common alpha-test 专项固定 `blue_in` frame 1 的十三个 Image draw、节点顺序及 `AAEBABBAABCBAAAAAA` key；对应 D3D9Ex smoke 提交 612 个完整 Image/Slice source 和 13 个 alpha-test source，ResetEx 前后均成功。Advertise 双纹理专项固定 `AS_title_in` 的十个节点、两个精确 key 及 stage 0/1 TEXL index。format-14 materializer 以精确顶点序列验证 `last,last,first,first`，Common 的 241 个 source 物理收束为 10 个 strip；format-13 materializer 把 Advertise 的 20 个字体 source 顺序拼成一条 3072 顶点 list。正常 runtime 路径分别有 73,588 像素的 Fennel 差分与 2,369 像素的 NumberGlyph 差分回归，物理合并后数值保持不变；双纹理独立提交在 ResetEx 前后均改变 2,073,600 个 RGB 像素。

## 当前边界

本页证明的是可运行编辑器窗口、D3D9Ex device、HiDPI ImGui draw、ResetEx 生命周期，以及首个无贴图 fixture 的实际 format 14 shader draw 和像素回读。它尚不证明完整 SRD 像素渲染完成。仍需在独立证据闭环后接入：

- ShapeEnv 剩余 scene/pass context 到每次实际 compact key 的精确选择；完整 82-key collection 与两个 Fennel key 的已验证 bytecode 已全部进入 runtime 表；
- sampler、带外部 base context 的 alpha/stencil/scissor 组合；
- 已闭合的 project-layer/Cast-vector/RefCast 递归顺序、copied-layer 世界/gate 与独立 Image/Text CAST 调用序列，已继续推进到 Advertise/Common 宿主根 key、type-1 target filter、Image/Slice/Number/Fennel 完整 layer-key 相邻 record merge、MainScene/BgScene 默认 rule/EntryInfo、首个 rule 匹配、per-pass 稳定队列、32-entry flush planner、统一 D3D9Ex source 顺序提交，以及 format-14 strip/format-13 triangle-list 的物理合并；仍需闭合后续 Enable/current-target 时序、其他 target、真正依赖 depth/order 的输入和 stencil/special-depth merge 状态。
