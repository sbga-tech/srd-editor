# SRD 2D ShapeEnv、screenParam 与 D3D9Ex 像素闭环

分析对象为 `chusanApp.exe` SHA-256 `28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`。以下结论来自只读反汇编/反编译、原版 shader collection 与真实 D3D9Ex 像素读回；没有以视觉结果反推游戏逻辑。

## CAST 二维标志进入 shader key

二维状态的完整写入链为：

1. `srd_render_image_cast` (`0xAD7BF0`) 调用 CAST 虚表 `+0x30`；所有当前 ImageCast 使用 `srd_cast_is_2d` (`0xACA350`)，返回 `(CAST+0x4C & 2) != 0`。
2. `srd_render_image_cast` 在 `0xAD7BFC` 把该布尔值传给 `srd_begin_quad_draw` (`0xAC5320`)。
3. `srd_begin_quad_draw` 把它作为第五参数传给 vertex builder 虚表 `+8`；`ceylon_begin_vertex_batch` (`0x6DE970`) 保存到 builder `+0x1C`。
4. `ceylon_submit_vertex_batch` (`0x6DF020`) 在 `0x6DF027..0x6DF040` 把该值写入 draw packet `+0x60` bit 7。
5. ShapeEnv key 的 low bit 3 读取 packet `+0x60` bit 7；该位附加 `sea::ShapeEnv2D`，其 selector parameter ID `1` 设置 Simple feature position `2`，即 `SSF_2DTRANSFORM=1`。

Rust 的 `CeylonDrawPacketPresetState::set_srd_quad_is_2d` 逐位复现第 4 步，并在 compact key 生成前按 `Layer::is_2d()` 调用。二维无贴图与单贴图 key 因此分别为：

```text
EAEBABBAAAGAAAAAAA
EAEBABBAABGAAAAAAA
```

它们不再错误使用缺少 position 2 的 `A...` 三维 sibling。

## screenParam 的来源与寄存器

二维 VS assembly 把 `screenParam` 压缩到 `c10`，位置公式为：

```text
clip.x = (world.x - 0.5) / screenParam.x - 1
clip.y = (-world.y + 0.5) / screenParam.y + 1
```

provider 链为：

- `screenParam` 在 `sub_65BB60` 注册，resource 保存于 `PassEnvSrc+0x40`；
- `sub_65C3A0`（PassEnvSrc 虚表 `+0x34`）写入前两个分量，`sub_65C470`（`+0x38`）写入后两个分量；
- `sub_614AB0` 在 `0x614CD5..0x614D0F` 读取 `FilterSrcBasic+0x3BC` 同步描述符的前两个有符号尺寸，转换为 float，逐项乘精确常数 `0.5`，再调用 `PassEnvSrc+0x34`；
- 同一路径以零对调用 `+0x38`。

所以当前路径精确得到：

```text
screenParam = [source_width * 0.5, source_height * 0.5, 0, 0]
```

三维 sibling 的 `c10..c13` 则是 target Camera 的 `Projection*View`。D3D9Ex renderer 因而按 draw 的 `is_2d` 选择上传一个 `c10` screenParam 或四个 `c10..c13` matrix registers，不把二者混用。

独立 SRD 不能证明 filter source 尺寸必然等于窗口 present size 或 SCN 尺寸。`WorldSnapshot` 因此要求宿主显式提供 `target_screen_size`；编辑器 UI 和 smoke CLI 也把它与 Camera present size 分开输入，不进行静默推断。

## 精确 bytecode 与实机结果

二维 VS 为 384 bytes，SHA-256 `A3E0CA2EFA3452A529DDE7E92EB638FAAD4A230DF5A5537D2D50BE53BC045BD4`。二维单贴图 PS 与已验证的三维单贴图 PS 相同，为 248 bytes，SHA-256 `066761E3FE149084A9526FDD1A091138B9DC894EAC29FA707D71992E4ED4E23F`。两份二维 exact key pair 已嵌入 Rust；runtime 不调用 Cg、D3DX 或 D3DCompiler。

对 `CHU_UI_Advertise_00_v10.srd` 的 scene 0/layer 4/node 4 `C_movie_dummy`，显式宿主输入为：

```text
target        = MainScene
present       = 1080x1920
screen source = 1920x1080
FirstCalc     = identity（AdvertiseLogoObject 二进制证据）
scissor       = disabled（smoke 显式外部输入）
```

单 draw 在首帧及强制 `ResetEx` 后第二帧得到完全一致的 Composition 读回：

```text
changed pixels = 849776
bbox           = (346,194)..(1573,885)
FNV-1a 64      = 09DF61BBE19B88A5
```

当前所有证据完整初始 draw 的组合 smoke 同样两帧一致：

```text
changed pixels = 2073600
bbox           = (0,0)..(1919,1079)
FNV-1a 64      = D0EEE7805AF4C325
```

FNV 只作本机诊断，不定义跨 GPU 的格式规范。像素覆盖证明本页链路已进入真实 D3D9Ex rasterization；它不代表 TEXT、CNUM、引用层、动画后状态或尚未闭合的 shader context 已完成。
