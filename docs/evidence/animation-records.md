# 动画记录读取证据

状态：`ANIM → MOT → TRK → KEY` 的记录分配、完整 runtime `SrAnimation` 布局、名称查找、选择/采样时序、三因子时钟、format 分派、时间区间处理、公共及 CAST 专属通道均已闭环。尚未命名的部分明确列于文末。

所列主要函数已与当前 `chusanApp.exe` 逐字节核对。

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`
- 保存后的 IDB SHA-256：`CDADC82B2932CF09122A1C73E7753446F7AAA3F7176A2DEE4F43902BBE725E7F`

## 层级和记录大小

### `ANIM`：`0xA9FD60`

目标记录大小为 84 字节。该函数读取：

| 属性 code | 读取方式 | 目标偏移 |
|---:|---|---:|
| `0x03` | type 2 字节串复制，最多 64 字节 | `+0x00` |
| `0x50` | 无符号标量 | `+0x48` |
| `0x56` | 有符号标量 | `+0x50` |
| `0x5F` | 无符号标量 | `+0x44` |

`+0x48` 非零时分配 `value * 8` 字节，并把指针存到 `+0x4C`。游戏先把每个槽初始化为：

```text
target = -1
track_count = 0
track_pointer = null
```

随后每个直接 `MOT ` 子块顺序填充一个槽。`CATR` 写入 ANIM `+0x40` 的辅助属性表，不消耗 MOT 槽。若直接 MOT 数少于 `0x50`，尾部槽保持 target `-1`；运行时应用函数明确跳过这些槽。因此 `0x50` 是 MOT 槽数，不是必须与直接 MOT 子块数相等的结构校验字段。

上述表只描述读取行为，不给 code 命名。

### `MOT `：`0xAA2290`

8 字节记录：

| 属性 code | 读取方式 | 目标偏移 |
|---:|---|---:|
| `0x51` | 有符号标量，截为 16 位 | `+0x00` |
| `0x52` | 无符号标量，截为 16 位 | `+0x02` |

`+0x02` 非零时分配 `count * 20` 字节，并把指针存到 `+0x04`；每个 `TRK ` 子块消耗一条 20 字节记录。

### `TRK `：`0xAA3FF0`

20 字节记录：

| 属性 code | 读取方式 | 目标偏移 |
|---:|---|---:|
| `0x53` | 无符号标量，截为 16 位 | `+0x00` |
| `0x57` | 无符号标量，截为 16 位 | `+0x02` |
| `0x54` | 无符号标量 | `+0x04` |
| `0x58` | 有符号标量 | `+0x08` |
| `0x59` | 有符号标量 | `+0x0C` |

`+0x02` 是 KEY 记录数量；`+0x04` 控制记录格式；分配结果指针写到 `+0x10`。

## KEY format 分派

`0xAA3FF0` 使用：

```text
low = format & 3
family = format & 0x70
```

已证明的组合：

| low | family | 单条大小 | 解析函数 |
|---:|---:|---:|---|
| `0` 或 `1` | `0x10` | 8 | `0xAA1D60` |
| `0` 或 `1` | `0x40` | 8 | `0xAA1B30` |
| `0` 或 `1` | `0x50` | 8 | `0xAA1CB0` |
| `3` | `0x10` | 20 | `0xAA1DF0` |
| `3` | `0x20` | 20 | `0xAA1EE0` |
| `3` | `0x40` | 20 | `0xAA1BC0` |

其他组合把 KEY 数据指针设为 null。旧报告中“KEY 固定 20 字节”的结论不成立。

## 8 字节 KEY 记录

所有解析器都以 code `0x5A` 开始一条新记录并写入 `+0x00`：

- family `0x10` (`0xAA1D60`)：`0x5A` → i32；`0x5B` → f32 位于 `+0x04`。
- family `0x40` (`0xAA1B30`)：`0x5A` → i32；`0x5B` → i32 位于 `+0x04`。
- family `0x50` (`0xAA1CB0`)：`0x5A` → i32；`0x5B` 的四个原始字节以顺序 `[1,2,3,0]` 写入记录 `+0x04..+0x07`。

family `0x50` 的字节重排必须保留为事实描述，不能先命名为 RGBA/ARGB。

## 20 字节 KEY 记录

每条记录：

```text
+0x00 code 0x5A: i32
+0x04 code 0x5B: value
+0x08 code 0x5C: u32
+0x0C code 0x5D: f32
+0x10 code 0x5E: f32
```

family `0x10` 的 `+0x04` 使用 f32；family `0x20` 和 `0x40` 的 `+0x04` 使用 i32。其余字段读取方式相同。

运行时已经证明这三项如何参与 20 字节记录求值，见下文；业务命名仍以实际公式为准。

## 运行时总分派

总入口 `0x129A0E0`：

1. KEY 数量为 0 时返回失败。
2. 调用 `0x129B330` 处理时间区间。
3. 按 `format & 3` 分派：

| low | 行为 |
|---:|---|
| `0` | 调用 `0x129A620` |
| `1` | 调用 `0x129B140`；该函数当前与 `0x129A620` 行为一致 |
| `2` | 返回成功，但不写目标值 |
| `3` | 调用 `0x129ABB0`，使用 20 字节记录 |

### 时间区间：`0x129B330`

若 `format & 0x300 == 0`，输入时间原样返回。

否则使用 TRK `+0x08` 和 `+0x0C`：

```text
start = track.+0x08
end   = track.+0x0C
```

时间处于 `[start, end)` 时原样返回；区间外按长度 `end - start` 循环折返到该半开区间。该函数只判断两位是否有任一置位，没有区分 `0x100` 与 `0x200`。

如果没有启用该区间循环，各关键帧求值器在首关键帧之前保持首值，在末关键帧及之后保持末值。没有“start 之前回退 CAST base 值”的逻辑。

NaN 帧也由比较方向闭环：各 Key8/Key20 求值器先执行有序比较 `first_frame < frame`。NaN 使该比较为假，因此直接返回第一关键帧值；它不会进入区间搜索或产生越界下标。Rust 已复现该分支。

## 运行时 `SrAnimation` 对象

`srd_construct_runtime_animation` (`0xAD4D40`) 为每个 parsed ANIM 建立一个独立 `0x8C` 字节对象：

| runtime 偏移 | 已证明内容 |
| ---: | --- |
| `+0x00` | 虚表 |
| `+0x04` | `std::string` ANIM 名称 |
| `+0x1C` | raw f32 frame，初始 `0.0` |
| `+0x20` | runtime f32 duration |
| `+0x24/+0x28/+0x2C` | 三个播放因子，初始均为 `1.0` |
| `+0x30` | runtime flags |
| `+0x34` | parsed ANIM 指针 |
| `+0x38` | owning runtime layer |
| `+0x3C..+0x88` | 从 CATR 派生的 vector/string/integer 字段；正式语义尚未闭环 |

parsed `0x56 >= 0` 时直接转换成 f32 duration；为负时，从 `0.0` 开始按 MOT/TRK 顺序对所有 TRK `range_end` 做 `fmaxf`，结果写入 `+0x20`。

构造时 flags 为 `0x9`；parsed ANIM flags 位 0 非零时再置 runtime 位 1，得到 `0xB`。已证明的 runtime 位行为为：

| mask | 行为 |
| ---: | --- |
| `0x02` | 循环播放 |
| `0x04` | 到达完成端点 |
| `0x08` | 允许推进 frame |
| `0x10` | 本 tick 发生循环折返；每 tick 开始时清除 |

初始化位 `0x01` 的正式名称仍未恢复。

### 时钟推进：`0x71F020`

允许推进时，核心运算严格为：

```text
delta = f32(+0x24) * f32(+0x28) * f32(+0x2C)
frame = frame + delta
```

非循环动画把 frame clamp 到 `[0,duration]`，并在运动方向的端点设置完成位。循环动画在有效正 duration 上执行浮点余数折返并设置 `0x10`；负乘积可使时间反向。上游 layer 更新 `0xABE3F0` 把本次调度标量写到 active ANIM `+0x2C`，再调用 `srd_apply_animation_motion_set` (`0xAD52B0`) 的推进分支。调度标量的正式时间单位尚未恢复，不能据此声称固定 FPS。

`0xAD52B0(false)` 是“不推进、只采样”分支：它临时令乘法链不产生 delta，然后按当前 raw frame 应用轨道。通道 23 用此分支递归采样引用层动画。

### 选择、激活与采样边界

`srd_find_animation_by_name` (`0xABF940`) 从 runtime layer `+0x28..+0x2C` 的动画向量开头遍历，以完整长度、区分大小写比较，返回第一个同名对象。

场景动画集选择 `0xAC2FD0` 只写入各已解析 ANIM 的 raw frame 并加入 active-animation 列表；它不调用 `0xAD52B0`。第一次 `MOT/TRK/KEY` 属性写入发生在下一次 layer 动画更新。直接 façade 或通道 23 则可以显式写 raw frame 后立即走“不推进、只采样”路径。

应用器按 ANIM `0x50` 槽顺序遍历：target 为负时跳过；否则用 target 索引 owning layer 的 runtime CAST 数组，对该 MOT 先运行全部公共通道，再运行全部 CAST 类型专属通道。完整 `0..23` 消费者表见 [`animation-channels.md`](animation-channels.md)；通道 23 的虚表边界、CRFD gate、默认 frame 和递归调用见 [`crfd-reference-cast.md`](crfd-reference-cast.md)。

Rust 当前解析并保存全部 ANIM 槽，保留尾部 target `-1` 项；每个 `ReferenceLayerRuntimeState` 独立复制目标层 CAST transform 和 runtime animation state。同名动画查找、raw frame、duration/flags、时钟推进、公共通道、SrImage 专属通道与通道 23 递归均已实现。53 个样本共 3099 个 ANIM、7053 个空 MOT 槽、2468 个自动 duration，并在 frame 0 成功应用 101922 条公共通道。

## low 0/1：8 字节记录求值

`0x129A620` 与 `0x129B140` 按 `format & 0x70` 分派：

| family | 运行时函数 | 行为 |
|---:|---|---|
| `0x10` | `0x129A890` | f32 线性插值 |
| `0x20` | `0x129AF40` | 保持区间左关键值 |
| `0x30` | `0x129A440` | 保持区间左关键值 |
| `0x40` | `0x129A1B0` | i32 经浮点线性插值后截断为整数 |
| `0x50` | `0x129A500` | 调用 `0x46DE30` 对四字节值插值 |

文件解析器对 low 0/1 只为 family `0x10`、`0x40`、`0x50` 分配 KEY 数据；运行时函数支持更多 family 不代表当前 SRD 读取路径会构造它们。

## low 3：20 字节记录求值

`0x129ABB0` 按 family 分派：

| family | 函数 | value 类型 |
|---:|---|---|
| `0x10` | `0x129A9A0` | f32 |
| `0x20` | `0x129B000` | i32 |
| `0x40` | `0x129A2D0` | i32 |

每段使用左关键记录的 `+0x08` 控制：

- family `0x10`：值 0 保持左值；值 2 使用三次曲线；其他值线性。
- family `0x20`：值 0 保持左值；其他值线性并截断为整数。
- family `0x40`：值 0 保持左值；值 2 调用 `0x129B400`；其他值线性并截断为整数。

### family 0x10 的三次公式：`0x129A9A0`

对当前记录 `k0` 和下一记录 `k1`：

```text
span = k1.frame - k0.frame
t = (frame - k0.frame) / span
delta = k1.value - k0.value
m0 = k0.+0x10
m1 = k1.+0x0C

value = (((((m1 + m0) * span - 2*delta) * t
          + (3*delta - (2*m0 + m1) * span)) * t
          + m0 * span) * t)
          + k0.value
```

这证明 `+0x10` 是当前段起点使用的斜率，`+0x0C` 是下一关键帧作为段终点使用的斜率。游戏没有旧预览器中的范围检查、2% 偏差限幅或线性回退。

### family 0x40 的三次公式：`0x129B400`

该分支先使用单精度常量 `0.0054931640625`（地址 `0x190D824`，原始字节 `00 00 B4 3B`）将整数 value 缩放到浮点域，按同样的三次多项式计算，再乘以 `-182.04443359375`（地址 `0x19A7488`，原始字节 `60 0B 36 C3`），并按 x86 `cvttss2si` 语义向零截断。公式中的 2 和 3 也来自单精度常量 `0x181C3AC` 与 `0x18E401C`。

该缩放是 family `0x40` 的真实二进制行为，不能用普通整数 Hermite 或线性插值代替。

## avatar 样本闭环

样本：`CHU_UI_Common_Avatar_Position_00.srd`

- SHA-256：`764B3F65CB878F6A1A9DF1A398A2B7A176BBDE9B4480ABC2304F8592EDC29A6F`
- `ANIM` 名称字节为 `001_Default_loop`，块偏移 `0x2005E`。
- `MOT ` 偏移 `0x206A1`，code `0x51` 原始值为 61。
- `TRK ` 偏移 `0x20765`，属性原始值：
  - `0x53 = 5`
  - `0x57 = 4`
  - `0x54 = 0x00000143`
  - `0x58 = 0`
  - `0x59 = 200`
- `KEY ` 块偏移 `0x2078F`，四条记录为：

```text
[frame=0,   value=0,    mode=2, +0x0C=0.0481231697, +0x10=0.0481231697]
[frame=60,  value=364,  mode=2, +0x0C=0,            +0x10=0]
[frame=140, value=-364, mode=2, +0x0C=0,            +0x10=0]
[frame=200, value=0,    mode=2, +0x0C=0.0516922809, +0x10=0.0516922809]
```

`0x143 & 3 == 3` 且 `0x143 & 0x70 == 0x40`，所以 frame 50 的闭环路径是：

```text
0x129A0E0 → 0x129ABB0 → 0x129A2D0 → 0x129B400
```

frame 50 位于 `[0, 200)`，时间不被折返。按游戏的单精度运算和向零截断，结果为整数 `349`。旧预览器的 `303` 线性回退和旧报告中的其他 Hermite 结果均不符合当前游戏二进制。

## 尚未证明

- format 的 low/family 位在引擎中的正式名称。
- `format & 0x300` 两个位是否在其他调用路径存在不同含义。
- 特殊低位分支 `0x129A7A0` 对引用/复合数据的完整业务语义。
- runtime `SrAnimation +0x3C..+0x88` 的 CATR 派生字段的正式结构名和全部用途。
- 三个播放因子的正式字段名、runtime flags `0x01` 的正式名称，以及上游调度标量的时间单位；三因子乘法、frame 更新和其到轨道求值器的调用链本身已经闭环。
