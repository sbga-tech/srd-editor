# Simple Cg assembly 与无 D3DX D3D9 bytecode 证据

本页记录不启动游戏的离线 shader 取证链。它使用原版 `cg.dll` 仅生成 canonical D3D assembly，再通过系统 `D3DCompiler_47!D3DAssemble` 产生 D3D9 bytecode；编辑器运行时和发布物不依赖 Cg 或 D3DX。

分析对象：`chusanApp.exe` SHA-256 `28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`；保存后的 IDB SHA-256 `0229B02CB4932EFE4576BE5AE7EC9E46ABA7470B9B430585FB5D858B238EC672`。

## 原版 profile 与编译参数

`sea_shader_selector_get_or_create_shader_pair` (`0x6BE2A0`) 对两个 stage 都把 profile descriptor `0` 传入 resource。`ceylon_shader_profile_to_backend_code` (`0xE9CEF0`) 保留低字节 `0`，`tea_cg_profile_from_shader_model` (`0x1319F90`) 因 stage 分别得到：

- vertex：Cg profile `6157`，即 `vs_3_0`；
- pixel：Cg profile `6165`，即 `ps_3_0`。

`tea_cg_create_program` 以 `CG_SOURCE (0x1010)`、entry `main`、compiler args null 调用 `cgCreateProgram`。离线输出头进一步独立显示 Cg `3.1.0013`、`-q -no_uniform_blocks -profile vs_3_0/ps_3_0 -entry main`，与二进制参数闭环一致。

## 历史隔离取证流程

现已删除的独立 x86 probe 曾执行：

1. 扫描 PE32 code 中的 shader-manager 注册调用形态，按共享 call target 自动恢复 resource records；
2. 直接从每条 record 读取文件名、长度、encoded VA 与 hash，再按 `encoded XOR hash XOR 0x59634649` 解码；已知 MATE 2.50 executable 自动发现并导出全部 138 个 Cg source/include；
3. compile 子命令复用 Rust 的 18 字节键 decoder 和完整 define 表；
4. 递归展开 include，并消除与游戏 include guard 等价的重复 include；
5. 动态加载游戏 `cg.dll`，用上述精确参数取得 D3D assembly；
6. 动态加载系统 `d3dcompiler_47.dll` 并调用 `D3DAssemble`；
7. 创建不可见 Win32 窗口和 windowed D3D9 HAL device（software vertex processing）；
8. 对每份 bytecode 调用 `CreateVertexShader` 或 `CreatePixelShader`，成功后立即释放对象；
9. 写出 `.cg/.asm/.bin` 与含精确 HRESULT 的 `manifest.tsv`。

该工具的 PE import table 没有 Cg、D3DCompiler 或 D3DX 静态依赖，所有取证 DLL 都显式动态加载；import table 对 `D3DX` 的匹配数为零，源码中也没有 D3DX 调用。工具与其生成物在原生 renderer 完成行为对照后一起删除，本页保留其可审计结果。

## 完整 Simple collection 结果

以 `D:\sdhd\assets\data\A000\shader\shadercollect.xml` 为输入，一次运行取得：

- 82 个 Simple key；
- 82 份 VS assembly/bytecode；
- 82 份 PS assembly/bytecode；
- Cg compile failure `0`，D3DAssemble failure `0`。

所有 VS bytecode 首 token 为 `0xFFFE0300` (`vs_3_0`)，所有 PS 首 token 为 `0xFFFF0300` (`ps_3_0`)，全部以 `0x0000FFFF` 结束。82 个键因部分 feature 只影响单一 stage，最终折叠为 14 个不同 VS bytecode hash 和 24 个不同 PS bytecode hash。

这套离线流程曾将完整 collection 去重为 14 个 VS 与 24 个 PS，并以 vkd3d-shader 2.0 生成 stripped SPIR-V，再由 Naga 验证 WebGPU 翻译。它证明了 shader 公式、vertex interface 与 82-key collection 的行为，不再是编辑器 build 输入。

当前编辑器已删除 SM3/SPIR-V asset、key-to-stage lookup、build-time translator 与 D3D9 runtime。运行时直接构造语义化 `SrdDrawState`，并使用 `src/renderer/wgpu/shaders/simple.wgsl` 这一份 canonical module；其两个 vertex declaration entrypoint 分别接收 format 14 与 format 13，随后共享同一 fragment program，profile feature 在 pipeline 创建时固化为 overrides。因此正常 Cargo build 不读取本页离线产物。

三维无贴图 sibling `AAEBABBAAAGAAAAAAA` 的 VS 为 332 bytes、SHA-256 `86669F24505A70D6DB560C6B2838EBA7D262B0206825BA3927658AB5A7112D61`；PS 为 216 bytes、SHA-256 `B7D50CF8DAC3A981DB13F2B5C3C7CAF8935FC4392B385B516620F7584EC2E53F`。

三维单贴图 sibling `AAEBABBAABGAAAAAAA` 也已按精确值嵌入：VS 复用上述 332-byte bytecode；PS 为 248 bytes、SHA-256 `066761E3FE149084A9526FDD1A091138B9DC894EAC29FA707D71992E4ED4E23F`。

恢复 CAST 二维标志到 ShapeEnv key 后，二维 sibling `EAEBABBAAAGAAAAAAA` 与 `EAEBABBAABGAAAAAAA` 也已按精确值嵌入。两者共享 384-byte VS，SHA-256 `A3E0CA2EFA3452A529DDE7E92EB638FAAD4A230DF5A5537D2D50BE53BC045BD4`；像素 stage 分别复用上述无贴图/单贴图 PS。该 VS 的 `SSF_2DTRANSFORM=1`，读取 `c10 screenParam`，不读取 `c10..c13 mtxPrjView`。完整 runtime 选择与像素闭环见 [`render-shape-env-2d.md`](render-shape-env-2d.md)。

完整 SliceCast runtime 语料又暴露四个精确 selector，均用同一离线原版 Cg 链编译并由隐藏 D3D9 HAL device 的 `CreateVertexShader/CreatePixelShader` 接受：

| compact key | VS | PS |
| --- | --- | --- |
| `EAEBABBAABCBAAAAAA` | 复用上述 2D VS | 296 bytes，SHA-256 `23CAC67F21338BC63A62D66DF9D5502AA0ED349C414D9AF73670BD8ADDCDCE65` |
| `EAEBABBAABIAAAAAAA` | 复用上述 2D VS | 复用单贴图 PS `066761E3FE149084A9526FDD1A091138B9DC894EAC29FA707D71992E4ED4E23F` |
| `AAEBABBAABIAAAAAAA` | 复用上述 3D VS | 复用同一单贴图 PS |
| `EAEBABBAAACBAAAAAA` | 复用上述 2D VS | 252 bytes，SHA-256 `B674C9D61CBE8B051742BB59C2C42618E027EA03DFE9D1F90839793CBBDDE290` |

Rust 为完整 collection 的全部精确 key 注册对应 bytecode；相同 bytecode 的 selector 仍分别保留，避免把 selector 语义折叠成猜测。编辑器 runtime 仍不加载 Cg 或 D3DX。

collection 中唯一的 MultiTex0 mode 9 键 `AAEBABBAADIIEAAAAA` 得到：

| stage | assembly bytes | bytecode bytes | SHA-256 |
| --- | ---: | ---: | --- |
| VS | 1879 | 348 | `FE98C9AEA8E461489C33DA284D6AEFACCA89042DE66420C09B0DAD698D3074F6` |
| PS | 2107 | 296 | `F111680C04362565C0B98D158716B3AA90C41A48345B2A692B3500CF1BF4C0A6` |

同一输入重复执行后两个 hash 均保持不变。其 PS assembly 直接显示 stage 0/1 sampler、MultiTex0 mode 9 的 alpha epsilon 公式、COLOR0 乘法、COLOR1.rgb 加法和最终异常高亮度清零分支，与解码 source 逐项一致。

## D3D9 HAL device 验证

探针通过动态加载 `d3d9.dll` 调用 `Direct3DCreate9(D3D_SDK_VERSION=32)`，随后以 adapter `0`、`D3DDEVTYPE_HAL`、`D3DCREATE_SOFTWARE_VERTEXPROCESSING` 和不可见的 1x1 windowed swap chain 创建一次 device。该次 `IDirect3D9::CreateDevice` 返回 `0x00000000`。

同一 device 对完整 collection 的 164 份 bytecode 逐个执行真实 COM 创建调用：

| stage | vtable index | count | shader-create HRESULT |
| --- | ---: | ---: | --- |
| VS | 91 (`CreateVertexShader`) | 82 | 全部 `0x00000000` |
| PS | 106 (`CreatePixelShader`) | 82 | 全部 `0x00000000` |

失败数为 `0`；每个成功创建的 shader 对象都立即经 IUnknown vtable index 2 释放。由此不只证明 token/profile 结构可解析，还证明完整结果被当前 D3D9 HAL 驱动实际接受。

重新检查探针 PE import table，只包含 `KERNEL32.dll`、`USER32.dll`、系统 CRT/NT DLL；没有 `cg.dll`、`D3DCompiler_47.dll`、`d3d9.dll` 或任何 D3DX 静态 import。后三类取证组件仍全部显式动态加载。

## 边界

已经证明：原版 Cg profile/参数、完整 collection 的 canonical assembly、无需 D3DX 的确定性 D3D9 bytecode 生成、bytecode profile/end token，以及全部 164 份 bytecode 的 D3D9 HAL shader 对象创建。

范围边界：自动 extractor 覆盖该 executable 内发现的全部 138 个 Cg source/include；checked runtime asset pipeline 目前只消费编辑器实际选择的 82-key Simple collection（外加两个 byte-identical Fennel aliases）。其余 Cg shader family 已可提取，但尚未成为 SRD renderer 的 runtime contract。
