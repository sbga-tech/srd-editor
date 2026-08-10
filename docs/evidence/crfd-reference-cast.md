# CRFD、SrRefCast 与动画通道 23 证据

本页记录引用 CAST 的数据记录、运行时指针链和通道 `23` 的已闭环行为。分析对象为：

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`；
- 保存后的 IDB SHA-256：`F75E8EB4A4E6437E9D17745C255AE6B9DCEB6BC2C46853AD6131CBCEB3B3E175`。

## CRFD 记录

`srd_parse_crfd` (`0xAA2DE0`) 为每个 `CRFD` 分配并清零 `0x608` 字节，再按属性直接写入：

| 属性 | 记录偏移 | 已证明用途 |
| --- | --- | --- |
| `0x51` | 不存入记录 | NODE 下标；解析结束后把记录指针写入该 NODE `+0x50` |
| `0x80` | `+0x000`，`char[0x200]` | 引用资源名 |
| `0x81` | `+0x200`，`char[0x200]` | 引用层名 |
| `0x82` | `+0x400`，u32 | 非零时允许通道驱动被引用动画 |
| `0x83` | `+0x404`，`char[0x200]` | 被引用动画名 |
| `0x84` | `+0x604`，f32 | 通道值为有序负数时使用的默认帧 |

`srd_resolve_reference_cast_resource` (`0xADB550`) 用 `+0x000` 与当前 SRD 的运行时 SCN 场景名逐项作完整字节比较，匹配后用 `+0x200` 查找其中的层并建立引用运行时对象。因此前两个字符串不是根据样本名称推断出的标签。场景表来源和两级首次匹配见 [`project-scene-reference.md`](project-scene-reference.md)。

## NODE 到 SrRefCast 的指针链

`srd_build_runtime_layer` (`0xAC00B0`) 以 `0x5C` 为步长遍历 NODE，调用 `srd_create_runtime_cast_for_node` 后再调用新 CAST 的虚表 `+0x04` 初始化器。所有当前 CAST 类型在该槽使用 `srd_init_runtime_cast_from_node` (`0xAD3F10`)；其中：

```text
runtime CAST +0x04 = parsed NODE pointer
runtime CAST +0x0C = parsed NODE +0x50
```

所以 NODE 类型 `3` 建立的 SrRefCast `+0x0C` 精确指向上述 CRFD 记录。`srd_apply_reference_animation_frame` 随后读取的 `+0x400/+0x404/+0x604` 均来自同一记录，不依赖结构相似性推断。

## 虚表槽边界

`srd_apply_cast_animation_channels` (`0xAD5370`) 对通道 `23` 先以 `0xBF800000` 初始化局部四字节值，再调用公共标量求值器，最后调用主 CAST 虚表 `+0x7C`。

构造函数写入的实际虚表证明：SrCast、SrNullCast、SrImageCast、SrSliceCast、SrTextCast 和 SrNumberCast 的该槽都指向 `j_nullsub_7188` (`0x445E2B`)；只有 SrRefCast 虚表 `0x190E310` 的 `+0x7C`（`0x190E38C`）指向 `srd_apply_reference_animation_frame` (`0xADB420`)。因此通道 `23` 对非 RefCast 是空操作，不应被赋予统一 CAST 字段。

## 引用动画帧应用

`srd_apply_reference_animation_frame` 严格按下列顺序执行：

1. SrRefCast `+0x1F4` 的引用层运行时对象为空时返回；
2. CRFD `+0x400` 为零时返回；
3. 传入 f32 与零作有序比较；仅有序负数替换为 CRFD `+0x604`，非负数和 NaN 原样保留；
4. 用 CRFD `+0x404` 在引用层的动画数组中作完整名称匹配；未找到时返回；
5. `srd_set_animation_frame_raw` (`0x71EEB0`) 把四个结果字节写入动画运行时对象 `+0x1C`；
6. 以参数 `0` 调用 `srd_apply_animation_motion_set` (`0xAD52B0`)，立即把该动画的公共通道和 CAST 专属通道应用到引用层 CAST。

公共标量求值器不按目标字段转换。因此 Rust 的 `ReferenceDefinition::animation_request` 同样从 `0xBF800000` 开始，保留 f32、i32 或四字节求值结果的原始位型，并使用同一负数回退条件。
因此通道 `23` 的 TRK **只提供 frame**；动画名、允许开关和负值回退帧分别来自目标 RefCast 的 CRFD `+0x404/+0x400/+0x604`。父动画每次采样都会覆盖该引用实例中 child ANIM 的 raw frame 并立即求值，但不会选择 child scene 的 ANMS、改变 layer gate、加载资源或按 child 自己的时钟额外推进。每个 RefCast 持有独立 copied layer，所以同一 authored layer 的兄弟引用实例互不改写；child 动画再命中另一个 RefCast 的通道 `23` 时按同一规则递归。

## 样本验证与实现边界

53 个本地 SRD 共解析并按 NODE 连接 1090 个 CRFD。语料中的 111 条通道 `23` 全部目标为 NODE 类型 `3`，全部使用 f32 Key20 format `0x13`；在每个 key 和相邻中点共完成 247 次引用动画帧请求求值。

Rust 当前已经实现：

- CRFD `0x51/0x80..0x84` 的默认值、类型和 512 字节字符串上限；
- CRFD 到 NODE 的连接及 NODE 类型回归；
- 通道 `23` 的原始位求值、非零 gate、负值默认帧和动画名请求；
- 顶层 Project/Scene/Layer 与 copied reference layer 两种父级到正确独立子实例的绑定；
- 首次同名动画查找、raw frame 保存、公共通道、SrImage 专用通道，以及子 RefCast 的递归请求。

`source_name/layer_name` 现在已经按游戏的同文件 `PROJ -> SCN  -> LAYR` 表解析，53 个样本中的 1090 个 CRFD 全部命中。独立引用层的建立顺序、父实例关系、世界矩阵、颜色、gate 和独立 ANIM 状态见 [`reference-runtime-recursion.md`](reference-runtime-recursion.md)。

完整语料从 3099 个顶层动画入口执行时，恰好触发 111 次通道 `23` 请求，并使动画层调用总数增加到 3210；这 111 次请求全部找到了绑定的独立引用实例和具名动画。最终 D3D9 draw submission 仍未完成。
