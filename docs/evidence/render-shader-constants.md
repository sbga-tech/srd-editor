# SRD Simple shader 常量提交证据

分析对象为 `chusanApp.exe` SHA-256 `28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`；本轮 IDB SHA-256 `7BE4A919000CC62E435A8BCBD534916D14DC17991C9E0C6DBA4E7A97D8C767B8`。

## D3D9 常量调用

`ceylon_submit_default_fixed_shader_constants` (`0x671A00`) 和 `ceylon_submit_draw_packet_fixed_shader_constants` (`0x671B80`) 通过 D3D9 device 虚表 `+0x178/+0x1B4` 分别调用 `SetVertexShaderConstantF` 与 `SetPixelShaderConstantF`。

默认提交为：

```text
VS c0..c3 = identity
VS c4..c7 = identity
VS c8     = [0,0,0,0]
VS c9     = [0,0,0,0]
PS c0     = [0,0,1,0]
```

逐 draw packet 提交为：

```text
VS c0..c3 = packet+0xA0，packet+0x9E 为 0 时改用 identity
VS c4..c7 = packet+0xE0，未启用时改用同一 identity
VS c8     = [packet+0x1C, packet+0x20, packet+0x3C, packet+0x40]
VS c9     = packet+0x44..0x53
PS c0     = [packet+0x1C, packet+0x20, packet+0x24, 0]
```

packet 构造器 `0x6CD8A0` 初始化 `+0x1C/+0x20=0`、`+0x24=1`、`+0x3C/+0x40=0`、`+0x44..+0x50=0`。`srd_begin_quad_draw` (`0xAC5320`) 的二维分支调用矩阵 setter 时传 null，明确使 packet world matrices 走 identity 分支。

## 选定无贴图 fixture

`CHU_UI_System_00_v10.srd` 中证据闭环的非 TEXT 图像节点 `C_fill` 选择 Simple key：

```text
EAEBABBAAAGAAAAAAA
```

对应 VS 读取 position、两个 color、texcoord0，以及：

```text
c0..c3   mtxWorld
c8       fixedParam0
c10       screenParam = [source_width/2, source_height/2, 0, 0]
```

对应 PS 不读取 sampler，只读取两个 color 和 `c0.z`。同层的 `T_title/T_message` 虽然 NODE 低字节同为 type 1，但其 CIMG 同时具有 TEXT 子块和 flags `0x100`，游戏工厂会建立 `SrTextCast`；审计工具现将它们排除，不能拿 ImageCast 路径替代 TEXT。因而首个闭环 draw 是唯一的无贴图 `C_fill`。该 layer 为二维层，CAST 二维标志写入 packet `+0x60` bit 7 并附加 `ShapeEnv2D`；其 `c10` 来自 filter source 半宽/半高，而不是 target Camera matrix。完整链见 [`render-shape-env-2d.md`](render-shape-env-2d.md)。三维 sibling 才由 `sea::AllEnvBasic` 提供 `c10..c13 Projection*View`，详见 [`projection.md`](projection.md)。

Rust `CeylonSrdFixedShaderConstants::initial_for_target` 同时保存二维 `screenParam` 和三维 target `Projection*View`，renderer 按 draw 类型只上传其中一条。`WorldSnapshot` 没有 `Default`，并要求显式 target screen size，因此调用端不能静默使用 SRD CAM、present size 或 SCN size 代替。二维对应的 384-byte VS 和 216-byte PS 已按对齐 `u32` token 表嵌入；lookup 只接受精确 key，不会用近似 shader 代替。
