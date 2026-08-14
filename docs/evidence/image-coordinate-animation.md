# SrImage CAST 动画通道证据

本页记录 CAST 专属动画通道到 SrImage 尺寸、顶点色和两份 48 字节坐标描述符的已闭环路径。分析对象为：

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`；
- 保存后的 IDB SHA-256：`F75E8EB4A4E6437E9D17745C255AE6B9DCEB6BC2C46853AD6131CBCEB3B3E175`。

## CAST 专属通道分派

`srd_apply_animation_motion_set` (`0xAD52B0`) 在公共通道之后调用 `srd_apply_cast_animation_channels` (`0xAD5370`)。后者已经证明下列直接目标：

| 通道 | 运行时目标 |
| --- | --- |
| `11/12` | SrImage 当前宽/高，求值后重新计算 size/origin |
| `13/14/15/16` | 四个 packed vertex color，目标顺序为记录偏移 `+0/+8/+4/+12` |
| `17` | 坐标描述符通道 0，即 CREF |
| `20` | 坐标描述符通道 1，即 CRE1 |
| `23` | CAST 虚表槽 `+0x7C`；只有 SrRefCast 覆盖该空操作以驱动引用动画帧 |

Rust 已实现 `11..17` 与 `20`；通道 `23` 的 SrRefCast 专属语义和 CRFD 来源见 [`crfd-reference-cast.md`](crfd-reference-cast.md)。

该 switch 覆盖 `11..23` 的完整整数范围；`18/19/21/22` 明确落入 default，因为它们已经由前一遍公共通道处理。不存在另一组 Text 或 Number 专用 TRK 目标。`srd_apply_animation_motion_set` 对每个 MOT 先完整调用公共通道，再完整调用本函数，因此 Rust 也保持这两个 pass 的边界。

## 尺寸通道 11/12

通道 `11` 和 `12` 分别复制当前 SrImage `[width,height]`，把公共标量求值器的四个结果字节原样写入 width 或 height，再调用 `srd_set_srimage_size_and_origin` (`0xAD2EB0`)。因此它们不进行目标类型转换，随后会同时更新：

- SrImage 当前尺寸；
- 两份坐标描述符中的尺寸副本；
- origin mode `0..8` 对应的 `factor * current_size`；
- mode 超出表范围时保留的自定义 origin。

`ImageGeometryState` 保存动画后的 size/origin；`apply_size_track` 与 `build_quad_with_geometry` 复现该写回和最终 Image/Number quad。样本中的通道 `11/12` 使用 format `0x13/0x113`，在 key 与相邻中点共完成 5712 次 CIMG/CNUM 尺寸求值。

## 顶点色通道 13..16

四个通道写入第一份坐标描述符的四个 packed color，但通道编号按二维网格而不是内存顶点顺序排列：

```text
13 -> vertex 0 (left/top)
14 -> vertex 2 (right/top)
15 -> vertex 1 (left/bottom)
16 -> vertex 3 (right/bottom)
```

样本使用 Key8 family `0x50`（format `0x51/0x151`）。`srd_eval_key8_bytes4_linear` (`0x129A500`) 在端点外保持端点值；区间内调用 `srd_lerp_packed_color_bytes` (`0x129BAA0`)。每个字节严格执行：

```text
left_unit  = f32(left)  * f32::from_bits(0x3B808081)
right_unit = f32(right) * f32::from_bits(0x3B808081)
out = cvtt_i32((left_unit * (1-t) + right_unit * t)
               * f32::from_bits(0x437F0000))
```

Rust 的 `ScalarValue::Bytes4` 保留 KEY 解析后的四字节顺序；游戏把求值结果原样写成 D3DCOLOR 的 little-endian `B,G,R,A` 字节，而 WebGPU 顶点使用语义 `R,G,B,A`，所以 `apply_vertex_color_track` 在写入 backend-neutral `ImageCoordinateState` 时执行 `[2,1,0,3]` 转换。53 个样本在 CIMG/CNUM 上的 key 和相邻中点共完成 19424 次顶点色求值。

## 标量轨道

`srd_apply_image_coordinate_track` (`0xAD3620`) 检查 TRK format 的最低两位。结果不等于 `3` 时，它调用公共标量求值器，并把四个结果字节原样写到所选描述符：

```text
descriptor +0x18: signed i16 reference selector
descriptor +0x1A: signed i16 explicit image index
```

这与游戏的原始位写入规则一致，不执行数值类型转换。该路径不修改描述符 `+0x1C` 的显式矩形标志。

## 20 字节引用 key 轨道

format 最低两位等于 `3` 时，`srd_eval_image_reference_track` (`0xAD36A0`) 使用该通道的引用表指针和声明数量：CAST `+0x1A4/+0x1AC` 对应 CREF，`+0x1A8/+0x1B0` 对应 CRE1。表为空或 key 数量为零时返回 false，并清除显式矩形标志。

`srd_eval_image_reference_keys` (`0x129AC50`) 使用 20 字节 key：

```text
+0x00 i32 frame
+0x04 i32 reference selector（最终写入描述符时截断为 i16）
+0x08 u32 mode
+0x0C/+0x10 本路径不消费
```

时间首先经过已有的 TRK wrap 规则。单 key、时间位于端点外，或左 key mode 为 `0` 时，直接采用对应 key 的 selector。区间内且左 mode 非零时：

```text
t        = (frame - left.frame) / (right.frame - left.frame)
selector = cvtt_i32(left.selector * (1-t) + right.selector * t)
rectangle[i] = left.rectangle[i] * (1-t) + right.rectangle[i] * t
```

selector 最终写入描述符 `+0x18`。`srd_lookup_cref_rectangle` (`0x129B2C0`) 用 key selector 查询当前通道的 CREF/CRE1 记录，再经 TEX 记录步长 `0x21C` 和 CROP 步长 `0x10` 取得四个归一化 f32。有效查询还把记录的 image index 写入显式 image index。

区间插值会依次查询左、右 key，并共用同一个 image-index 输出。因此右 key 有效时，最终显式 image index 来自右 key；只有两端矩形都有效时才覆盖四个插值矩形。如果查询无效，游戏保留描述符中此前的矩形字节。Rust 复现这项有状态行为，但对负 selector 不执行游戏中未定义的数组前寻址。

求值函数返回 true 后，`srd_apply_image_coordinate_track` 把描述符 `+0x1C` 设为 true。后续 `srd_resolve_cref_texture_coordinates` 仍先检查插值后的 selector 是否落在声明数量内，然后才使用显式 image/rectangle；动画不能绕过 selector 边界。

## Rust 对应与样本验证

`RuntimeImageState` 保存 size/origin 和两份独立的 48 字节坐标描述符子状态。四个 packed vertex color 通道只写第一份描述符，第二份保留初始化时的副本，和 `srd_apply_cast_animation_channels` 的实际偏移一致。`apply_coordinate_track` 同时支持标量原始位写入和 20 字节引用 key，保留 wrap、端点、mode 0 hold、`cvtt` selector、左右查询顺序、显式 image 选择、矩形 f32 插值及无效查询时的旧矩形。

所有 CAST 构造时都先调用 `srd_srimage_construct`。Rust 因此为 Null、Image/Text、Slice、Ref 和 Number 的每个顶层 CAST及每个引用副本 CAST 都建立独立 SrImage 状态：未被类型初始化器覆盖时采用构造器的零颜色、零尺寸、双 selector `0` 和 origin mode `4`；Image/Text 从 CIMG 初始化，Slice 从 CSLI 初始化，Number 从 CNUM 初始化。

53 个本地 SRD 中，通道 `17` 只出现 format `0x23/0x123`，通道 `20` 同样只出现 `0x23/0x123`。对已连接到 CIMG/CNUM 的轨道，在每个 key 和相邻 key 中点共执行 183432 次求值，其中 183016 次得到能够继续通过 `resolve_coordinates` 的显式纹理引用。差额来自游戏允许返回显式状态、但 selector 或 image index 在该时刻不可绘制的情况。

把相同分派应用到 2087 个独立引用层实例的全部 11382 次动画调用时，共执行 371568 个公共通道和 102506 个 SrImage 专用通道；所有实际 CIMG、TextCast、CSLI、CNUM 以及构造器默认状态均走同一运行时路径。

## 仍未闭环

- 动画后的双 UV 在 shader/固定管线中的最终组合与 D3D9 draw 状态。
