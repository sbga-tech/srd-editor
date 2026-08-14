# 首个证据完整 SRD draw list

本页把已经分别闭环的解析、运行时状态、矩阵、shader 和 D3D9 状态收束为第一个可以提交 GPU 的 draw。它不增加未经证明的游戏语义，也不把编辑器预览布局当作游戏输入。

## 选定 fixture

文件为 `surfboard/system/CHU_UI_System_00_v10.srd`。解析后的 scene 0/layer 0 中，唯一满足当前全部证据边界的对象是 node 1 `C_fill`：

- 它是 `SrImageCast`，不是带 `TEXT` 子块及 CIMG flags `0x100` 的 `SrTextCast`；
- 它不绑定纹理；
- Shape key 为 `00000000:00371F90`；
- layer 为二维，因此 Simple compact key 为 `EAEBABBAAAGAAAAAAA`，对应已经嵌入并由真实 D3D9 HAL device 创建验证的 VS/PS；
- 初始 packet 精确导出 `CULL_NONE`、`SOLID`、color-write `0xF`，且 Z disabled。

对应证据链分别见 [`cimg-image-cast.md`](cimg-image-cast.md)、[`render-shader-constants.md`](render-shader-constants.md)、[`render-shader-bytecode.md`](render-shader-bytecode.md)、[`render-raster-state.md`](render-raster-state.md) 与 [`render-alpha-depth-stencil.md`](render-alpha-depth-stencil.md)。

## FirstCalcMatrix 边界

`build_base_pose_image_draws` 要求调用者显式提供无默认值的 `WorldSnapshot`，其中分别保存 `FirstCalcMatrix`、可选的 renderer project target、最终接收 target Camera 的 `Projection*View` 与 target screen size。语料测试对命名 target 与接收 target 都传 identity、screen size 传 `1920x1080`，只是在独立宿主输入固定时验证确定结果；它不声称原游戏任意调用现场都使用这些值。对已审计的 Advertise/Common，空 `TargetScene` 精确 lookup 已证明产生 `renderer_project_target=None`；独立 SRD 的通用预览仍不能脱离宿主上下文自动选择接收 target。详见 [`render-visibility-culling.md`](render-visibility-culling.md)。

编辑器未来的 fit-to-view 属于 Composition 显示变换，必须与这个游戏根矩阵分层保存。HiDPI 只改变窗口/backbuffer 的物理像素和 ImGui 的逻辑到物理比例，也不能进入 SRD 的 `FirstCalcMatrix`。

## 当前生成结果

在 `FirstCalcMatrix = identity`、显式 target `Projection*View = identity` 时，Rust 生成一个 draw：

```text
scene/layer/node = 0/0/1
positions        = (0,0,0), (0,1080,0), (1920,0,0), (1920,1080,0)
primary color    = 00 00 00 FF（四顶点相同）
secondary color  = 00 00 00 00（四顶点相同）
primitive        = non-indexed triangle strip, 4 vertices
```

draw 同时携带精确 packet、VS `c0..c13`/PS `c0` 固定常量、blend、raster 和 depth 状态。世界矩阵使用 `FirstCalcMatrix * layer local` 后沿 CAST 层级执行 `parent_world * local`；世界色只在 NODE flags 明确要求时继承父级乘色、加色和可见性。

## 显式排除范围

当前 builder 只产出证据完整子集：

- 遇到 CAST 特殊矩阵 flags `0x0007_0000` 时返回错误；
- 排除 `SrTextCast`；
- 只产出已解析 CREF/CRE1 绑定与 sampler 的纹理 draw；
- 保留 packet 与 texture presence，即使由它们派生的 shader key 没有已注册 VS/PS，最终由 backend 在提交时拒绝；
- 本页旧的窄 builder 仍只描述首个 Image fixture；统一 runtime builder 已另外接入 CSLI、CNUM 与引用层递归，见各自证据页。

因此“没有产出”不等于对象不可渲染，只表示它仍在上述显式排除范围。本页 fixture 测试断言 draw 数量、节点、提交时派生的二维 shader key、`screenParam`、四顶点、两组顶点色、color-write 和深度状态。

## 实际 D3D9Ex 提交与像素验证

`src/d3d9_srd.rs` 已把这个 draw 接到真实 D3D9Ex device：

- 由 `SRD_D3D9_VERTEX_DECLARATION` 创建 format 14 vertex declaration；
- 在 DEFAULT pool dynamic/write-only vertex buffer 中按原始 36 字节布局写入四顶点；
- 选择嵌入式 key 对应 VS/PS，上传 VS `c0..c13` 与 PS `c0`；
- 提交已闭合 blend、cull、fill、color-write 和 depth state；
- 执行 `DrawPrimitive(D3DPT_TRIANGLESTRIP, 0, 2)`；
- 用 state block 恢复编辑器调用前的 D3D9 状态。

material scissor 与基础 alpha/stencil command 在二进制中是外部 context，而不是 SRD 属性，所以 renderer API 要求显式传入 `SrdDx9ExternalContext`。独立 smoke 明确使用 disabled scissor，并使用 Material 构造器已证明的 alpha test disabled、reference `0`、comparison `GREATER` 默认值；alpha-test preset 因而可以精确覆盖 enable 并保留这两个基础字段。packet stencil override 的逐提交 sequence 生命周期仍未闭环，当前 GPU 子集对此继续显式拒绝。

`--srd-draw-smoke` 在物理 backbuffer 上先清为 `0xFF202226`。未给出 game host 参数时，它只在 smoke flag 下显式构造“SRD CAM 作为 target Camera、SCN size 作为合成 screen”的诊断 host；该 host 不会用于普通编辑器预览，也不声称等于游戏 target。给出 `--advertise-logo-host` 时，target、present 与 screen source size 都由调用者显式指定。`EndScene` 后通过 `GetRenderTargetData` 验证 Composition 存在非清屏覆盖，并检查 backbuffer 诊断采样发生变化；强制 `ResetEx` 后第二帧重复验证。

Composition 现已创建与 SCN 尺寸一致、格式取自实际 backbuffer 的 DEFAULT-pool render-target texture，并把它注册到 ImGui texture table。面板按可用逻辑尺寸保持宽高比居中显示；HiDPI framebuffer scale 只作用于 ImGui 最终顶点与 scissor。`ResetEx` 前会先从 texture table 移除 COM 引用并释放 target，reset 后重建和重新注册。

smoke 在 `EndScene` 后回读 Composition texture 并诊断主 backbuffer；随后强制 reset 的第二帧重复通过。这里的 aspect-fit 是编辑器显示层变换，不会回写或替代 `FirstCalcMatrix`、target Camera 或 ShapeEnv2D `screenParam`。
