# LAYR、NODE、TRS 与运行时矩阵证据

本页只记录从游戏二进制闭环得到的结论。分析对象为：

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`
- 本轮保存后的 IDB SHA-256：`5ACFF2969D10D0840FEA516DCB20699324E2286DDB00E6C404E862F77B695074`

## 原始记录

`srd_parse_layr` (`0xAA1FD0`) 写入的 LAYR 字段：

| 属性 | 解析后偏移 | 已证明行为 |
| --- | ---: | --- |
| `0x03` | `+0x00` | 最多复制 64 字节 |
| `0x20` | `+0x44` | unsigned 标量 flags |
| `0x21` | `+0x48` | unsigned 节点数 |
| `0x22` | `+0x54` | unsigned 动画数 |
| `0x23` | `+0x5C` | 逐项读取 unsigned，最多 64 项 |

`srd_parse_cast` (`0xAA0130`) 按节点数分配 `92` 字节 NODE 记录。若 LAYR flags 位 0 清零，则分配 `36` 字节 TRS2；若置位，则分配 `48` 字节 TRS3。属性 `0xFE` 在三个解析器中分别把当前写指针推进 `92`、`36`、`48` 字节，因此它是游戏使用的记录分隔符。

`srd_parse_node` (`0xAA06F0`) 的已证明布局：

| 属性 | 偏移 | 读取方式 |
| --- | ---: | --- |
| `0x03` | `+0x00` | 最多 64 字节 |
| `0x30` | `+0x48` | unsigned 标量；运行时工厂读取低字节作为 CAST 类型分派码 |
| `0x32` | `+0x4C` | signed 标量；父节点为 type 2 时索引父级 CSLI 生成单元 |
| `0x3C` | `+0x54` | signed 标量后截断为 16 位；首子节点下标 |
| `0x3D` | `+0x56` | signed 标量后截断为 16 位；下一同级节点下标 |
| `0xA0` | `+0x58` | signed 标量 |

`0x32` 的完整限定条件、越界/active 分支和二维中心偏移公式已闭环，详见 [`csli-layout.md`](csli-layout.md)。它不是层级 parent 字段；`0x3C/0x3D` 的层级语义由下述最终构建函数闭环证明。

### NODE 初始 CAST Active 状态

`srd_init_runtime_cast_from_node` (`0xAD3F10`) 在 `0xAD3F7D..0xAD3F93` 读取解析后 NODE `+0x48`，把序列化 bit `0x100` 精确映射到运行时 `SrCast+0x4C` bit 2。这个位不是 CAST type 的一部分；type 分派只读取 NODE `+0x48` 的低字节。

公共绘制包装器 `0xAD45E0` 先在 `0xAD4609` 检查派生可见性 `SrCast+0xC4`，再在 `0xAD4616..0xAD461E` 检查 `SrCast+0x4C` bit 2；任一条件不成立都不调用实际 CAST 绘制函数。因此 NODE `0x100` 是序列化的初始逐 CAST **Active** 状态，运行时实现为 draw gate；它独立于 TRS `0x3B` 可见性和 LAYR/ANMS/SANM layer gate。

runtime layer 按扁平 CAST vector 逐项调用公共绘制包装器，所以父 CAST 的 bit 2 清零不会自动关闭普通子 CAST。只有已有的父可见性条件或 RefCast owning gate 会沿各自路径传播。

`srd_parse_trs2` (`0xAA0970`) 与初始化器 `0xA9F120`：

| 属性 | 原始偏移 | 已证明行为 |
| --- | ---: | --- |
| `0x34` | `+0x00` | 2 个标量转换成 f32 |
| `0x35` | `+0x08` | signed i32 |
| `0x36` | `+0x0C` | 2 个标量转换成 f32 |
| `0x3A` | `+0x14` | 4 字节按 `[1,2,3,0]` 重排 |
| `0x33` | `+0x18` | 4 字节按 `[1,2,3,0]` 重排 |
| `0x3B` | `+0x1C` | signed i32 |
| `0x3E` | `+0x20` | unsigned 后截断为 u16 |
| `0x3F` | `+0x22` | unsigned 后截断为 u16 |

矩阵相关默认值为位置 `[0,0]`、旋转 `0`、缩放 `[1,1]`、`0x3B = 1`。初始化器没有显式写原始 `+0x18` 和 `+0x22`，因此当前实现不制造这两个字段的默认值。

`srd_parse_trs3` (`0xAA0C10`) 与初始化器 `0xA9F190`：

| 属性 | 原始偏移 | 已证明行为 |
| --- | ---: | --- |
| `0x37` | `+0x00` | 3 个标量转换成 f32 |
| `0x38` | `+0x0C` | 3 个标量转换成 i32 |
| `0x39` | `+0x18` | 3 个标量转换成 f32 |
| `0x3A` | `+0x24` | 4 字节按 `[1,2,3,0]` 重排 |
| `0x33` | `+0x28` | 4 字节按 `[1,2,3,0]` 重排 |
| `0x3B` | `+0x2C` | signed i32 |

向量转换函数 `0x12990E0` 证明了整数、浮点和 `cvttss2si` 分支；Rust 解析器逐类型复现这些转换。矩阵相关默认值为位置和旋转全零、缩放全一、`0x3B = 1`。

## 运行时公共变换

`srd_build_runtime_layer` (`0xAC00B0`) 在 `0xAC0159` 写入：

```text
runtime_layer.is_2d = (parsed_layer.flags & 1) == 0
```

同一函数使用 NODE stride `92`，并在 2D 时使用 TRS2 stride `36`、3D 时使用 TRS3 stride `48` 调用 CAST 工厂。因此 flags 位 0 的 2D/3D 含义不是标签名推测。

CAST 基类构造器 `0xAD3020` 把公共局部变换放在 CAST `+0x5C`，世界矩阵放在 `+0x8C`。初始化器 `srd_init_runtime_transform` (`0xAD6F70`) 以及复制函数 `0xAD6E10`、`0xAD6EA0` 证明公共空间字段：

```text
+0x00 translation.x f32
+0x04 translation.y f32
+0x08 translation.z f32
+0x0C rotation.x i32
+0x10 rotation.y i32
+0x14 rotation.z i32
+0x18 scale.x f32
+0x1C scale.y f32
+0x20 scale.z f32
+0x24 multiplicative color，内部四分量字节
+0x28 additive color，内部四分量字节
+0x2C visibility low byte；动画求值器可向此处写完整 4 字节
```

TRS2 复制把二维位置放入 X/Y、旋转放入 Z、缩放放入 X/Y，其余空间分量分别补 `0/0/1`。TRS3 三组分量直接复制。两者都把原始 `0x3B != 0` 归一成 visibility 字节 `0/1`。

## 动画通道

`srd_apply_animation_motion_set` (`0xAD52B0`) 使用 MOT 属性 `0x51` 作为运行时 CAST 指针数组下标，然后调用公共和 CAST 专属通道消费者。这证明 MOT target 是 CAST/节点下标。

`srd_apply_common_animation_channels` (`0xAD5580`) 证明：

| target | 目标 |
| ---: | --- |
| `0..2` | translation X/Y/Z |
| `3..5` | rotation X/Y/Z |
| `6..8` | scale X/Y/Z |
| `10` | visibility 的 4 字节存储 |

标量求值函数直接向目标地址写入 4 字节；它不会按目标字段类型转换。因此 Rust 的公共位置、旋转、缩放和 visibility 通道应用保留 `f32`、`i32` 或四字节结果的原始位型。

`srd_copy_trs2_to_runtime_transform` (`0xAD6E10`) 和 `srd_copy_trs3_to_runtime_transform` (`0xAD6EA0`) 将 parsed `0x3A/0x33` 交给 `srd_unpack_packed_color`，分别写入 runtime transform `+0x24/+0x28`。随后 `srd_compose_cast_world_state` 对 `+0x24` 调用 `srd_multiply_color_u8`，对 `+0x28` 调用 `srd_add_color_saturating_u8`；最终图像、切片和数字 CAST 也把两条结果分别作为 primary multiplicative tint 与 secondary additive tint 消费。因此两字段语义已经由解析、运行时组合和最终顶点颜色三端闭环。

其余公共通道为：

| target | 目标与转换 |
| ---: | --- |
| `9` | 求值结果内存字节 `2,1,0` 写入 multiplicative color 分量 `0,1,2`；分量 3 不变 |
| `19` | 同样写入 additive color 分量 `0,1,2` |
| `21` | f32 从 `1.0` 初值求值，按 SSE 比较/max 路径限制到 `[0,1]`，NaN 变为 `0`，乘 `255` 后 `cvttss2si`，写 multiplicative 分量 3 |
| `22` | 同样写 additive 分量 3 |

Rust 已复现上述字段默认值、属性字节顺序和四条动画通道。53 个样本包含 7224 个非全 `255` multiplicative transform color、22595 个非零 additive transform color；通道 `9/19/21/22` 分别出现 `3541/4153/14995/609` 条。CAST 专属通道 `11..17` 与 `20` 详见 [`image-coordinate-animation.md`](image-coordinate-animation.md)；`23` 详见 [`crfd-reference-cast.md`](crfd-reference-cast.md)。

本地 avatar 样本已闭环验证：`001_Default_loop` 的 MOT target `61` 选择第 61 个 CAST，target `5` 在 frame `50` 求值得到 `349`，并写入该 CAST 公共变换的 rotation Z。

## 局部矩阵与世界矩阵

`srd_build_local_affine_matrix` (`0xABEAB0`) 使用三行四列、行主序、隐式第四行 `[0,0,0,1]` 的仿射矩阵。

- 位置先加调用方提供的二维 offset；runtime layer `+0x131` 非零时再翻转 Y。
- i32 旋转乘精确 f32 位型 `0x38C90FDB`，即每单位约为 `2π/65536`。
- 2D 分支只读取 rotation Z，并在转换为角度前执行整数取负。
- 3D 分支按 Z、Y、X 的顺序更新矩阵。
- 最后按列乘 scale X/Y/Z，并写入平移列。

游戏没有调用 CRT `sinf/cosf`。`srd_game_cos_f32` (`0x6688B0`) 和 `srd_game_sin_f32` (`0x669C40`) 使用 f64 范围折叠与多项式近似，随后在调用点存回 f32。Rust 版本使用从 EXE 读取的精确 IEEE-754 位型，并保持乘加的汇编顺序。

`srd_mul_affine_3x4` (`0x609C20`) 对每一行执行：

```text
(left.w * [0,0,0,1] + left.z * right.row2)
+ (left.y * right.row1 + left.x * right.row0)
```

`srd_compose_cast_world_state` (`0xABF350`) 以 parent world 为左操作数、local 为右操作数调用它。`srd_update_cast_tree` (`0xAC0F80`) 在父 CAST 后递归子 CAST；没有父 CAST 时使用 layer 根矩阵。故世界组合顺序严格为：

```text
world = parent_world * local
```

`srd_build_cast_hierarchy` (`0xAC1AC0`) 与 `srd_attach_child_cast` (`0xAD3500`) 证明层级来源：对每个当前节点，读取 NODE `+0x54`（属性 `0x3C`）作为首子节点下标；若不是 `-1`，把对应 CAST 挂到当前 CAST，并反复读取该子节点 NODE `+0x56`（属性 `0x3D`）取得下一同级下标，直到 `-1`。挂接函数同时写 child CAST `+0x30` 的 parent 指针并追加到 parent CAST `+0x34` 子列表。最终 parent 指针仍为空的 CAST 按原节点顺序加入 layer `+0x64` 根列表。

Rust `Layer::build_hierarchy` 已按该首子/同级链构建 parents、children 和 roots，并通过全部 53 个样本回归。

特殊矩阵 flags 的 parent inverse、SRD Camera inverse View、2D parent-axis scale
保留、3D switch、translation 恢复与递归后 Z 清零已经闭环，详见
[`special-cast-matrix.md`](special-cast-matrix.md)。

## 尚未闭环

- `srd_compute_parent_csli_cell_offset` 的 SrSliceCast 尺寸、origin、显式单元累计、2D flags 分支与公式均已闭环并由 Rust 从 SRD 数据自行计算。
- TRS packed color、公共通道 `9/19/21/22` 及父子颜色组合已经实现。
- CAST 专属动画通道 `11..17/20/23`、引用资源递归实例化、特殊矩阵 flags 及投影/视口映射已闭环；剩余工作是完整 layer animation 对象和最终 D3D9 状态提交。
