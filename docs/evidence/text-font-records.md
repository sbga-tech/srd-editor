# TEXT、FONT/CHAR 与外部 RFZ 字体资源

状态：SRD 内 TEXT、FONT、CHAR 的记录布局，TEXT 到项目 FONT 下标解析，SrTextCast 建立/初始化，外部 RFZ/YABX/Ruhuna/AVTS/DDS 字体资源，Ruhuna Database/Glyph 到游戏 128 字节 runtime glyph 的转换，RFZ `TextBox` 路径与旧式 FONT/TEX/CROP 路径的运行时分流，实际游戏使用的 UTF-8 输入模式和当前完整语料所需的 Fennel token 子集，以及 format 13 字形 batch 的顶点声明、shader、DrawPacket、atlas sampler 和 D3D9 提交参数均已闭环。静态 mode-zero `sub_7C1F90` 的 auto-fit、自动断行、固定字符表、空格候选、对齐、垂直 `-254` 截止与 `+0x12C/+0x34C` 行元数据，fresh mode 1 的 `0x08` X/Y 联动 auto-fit，fresh mode 5/6 的 `0x4000` 垂直截止关闭，通用 flags `0x200` 固定 cell 度量/尾部居中修正，以及已知 Flag20 states 的 `sub_7C4070` 排版与 `+0x358` 状态均已实现。CATR `FontParamData` 到实际 mode、低位 flags、monospaced、裁剪、FontObject style/RFZ record flags、无 `$D/$L` scroll 初态和 shadow effect 的运行时链也已闭合并接入。`sub_7C0D40` 的记录过滤、atlas 分组与初始 hash 前向链顺序，以及 `sub_7C7F90` normal/effect glyph 的 origin/effective-scale、2D CPU matrix/3D packet matrix、效果色、位移、buffer 顺序与裁剪链同样已实现。SrTextCast 初始 world/color、零颜色门控、ShapeEnv material cull 和真实 D3D9Ex Composition 像素回归现已闭环；3D TextCast 证据见 [`textcast-3d.md`](textcast-3d.md)。当前剩余主线是 Flag40 的真实 flags 来源，以及 FontParam vertical、`$D/$L` 动态滚动与 mode 6 调用点。

## TEXT 记录

`srd_parse_cimg` (`0xAA1410`) 遇到直接 `TEXT` 子块时分配 `0x24` 字节并调用 `srd_parse_text` (`0xAA18F0`)。后者的直接写入为：

| 属性 | TEXT 偏移 | 读取行为 |
| --- | --- | --- |
| `0x78` | `+0x00`, u32 | unsigned scalar |
| `0x79` | `+0x04`, i32 | FONT 下标 |
| `0x7A` | `+0x08`, `char*` | 分配 2048 字节，最多复制 2047 字节 |
| `0x36` | `+0x0C/+0x10`, f32[2] | `sub_1298730(..., count=2)` |
| `0x7B` | `+0x14..+0x1A`, i16[4] | 四个 scalar 读为 i32 后截断写入 |
| `0x7C` | `+0x1C`, i16 | signed scalar 截断 |
| `0x41` | `+0x1E`, i16 | signed scalar 截断 |
| — | `+0x20` | 已解析 FONT 指针 |

解析器先把 `+0x08` 和 `+0x20` 清零。若 `0x79 >= 0` 且小于 PROJ 的 FONT 数，就按 `FONT_record_size=0x54` 把对应记录地址写入 `+0x20`。因此 TEXT 的字体关系是当前同一 PROJ 内的下标，不是由字符串名在运行时任意搜索。

Rust 的 `TextDefinition` 保留已证明字段；二进制没有显式初始化且样本可能省略的标量保存为 `Option`，没有虚构默认值。

## PROJ FONT 表

`srd_parse_srck` (`0xA9F370`) 对 PROJ：

- 属性 `0x05` 截断为 u16 写入 PROJ `+0x48`，表示直接 FONT 数；
- 按 `count * 0x54` 分配 FONT 数组并写入 PROJ `+0x54`；
- 直接 `FONT` 子块按顺序交给 `srd_parse_font` (`0xAA1110`)。

FONT 记录：

| 属性 | FONT 偏移 | 行为 |
| --- | --- | --- |
| `0x03` | `+0x00`, `char[0x40]` | 字体资源名 |
| `0x70` | `+0x44`, u32 | flags；低三位任一非零时 lookup 容量为 65536，否则为 256 |
| `0x71` | `+0x48`, u16 | unsigned scalar 截断 |
| — | `+0x50` | lookup 表，每项两个 i16，初始均为 `-1` |

`srd_parse_char` (`0xAA12D0`) 对每个直接 CHAR：

- `0x72` 读取 lookup 下标；
- `0x4A` 连续读取两个 signed scalar，截断为 i16 后写入该下标的两个值。

Rust 用稀疏 `FontCharacterMapping` 保存实际 CHAR 写入，并以文件顺序最后一次写入优先查询；不会为每个 FONT 强制分配 65536 项。当前本地 53 个 SRD 的 249 个 FONT 均没有内联 CHAR，说明这批样本实际依赖外部字体资源。

## SrTextCast 初始化

`srd_create_runtime_cast_for_node` (`0xACA030`) 只在 NODE type 1、CIMG 存在 TEXT 且 CIMG flags `0x100` 非零时选择工厂 key 2。工厂分配 `0x350` 字节并调用 `SrTextCast` 构造函数 `0xAD8270`；构造函数先建立 SrImageCast 基础部分，再写主虚表 `0x190E210`。

公共 CAST 初始化器调用虚表 `+0x08`，SrTextCast 在该槽进入 `sub_ADA4B0`：

1. 以 CIMG 初始化内嵌 SrImage；
2. 以 CIMG `+0x30` 的 TEXT 指针初始化 TextCast `+0x1F4` 的播放/文本状态；
3. 读取 TEXT `+0x08` 字符串；
4. 按 SrPlayer 的外部属性分支调用 TextCast 字符串 setter；
5. setter 最终更新 TextCast `+0x1F8` 起始的字符串状态并触发虚表 `+0xBC` 的重建路径。

这证明 TextCast 不是把 CIMG quad 直接换一张纹理：它拥有独立的字符串状态、字体资源和重建过程。

## RFZ 外部资源

AdvertiseLogo 的两个 FONT 名为：

```text
RFO_SEGAKAKUGOTHIC_DB_24pt.rfz
RFO_SEGAKAKUGOTHIC_DB_32pt.rfz
```

本地完整 data 根共有 6 个 RFZ，位于 `A000/font`。全部实文件开头为：

```text
59 53 02 00
Y  S  version=2
```

资源入口 `sub_F478A0` 明确：

1. 要求至少 4 字节且前两字节为 `Y`,`S`；
2. 要求第三字节为 `2`；
3. 建立序列化 reader，并把扩展名 `.rfz` 交给对应 handler；
4. 解析结果必须具有根类型名 `RHFONTDB`，否则释放并返回失败。

初始化函数 `sub_F531F0` 把普通 handler 注册为 `rfo`，把 RTTI 明确为 `yabukita::SerializeHandlerBinaryLZW` 的 handler 注册为 `rfz`。所以 RFZ 是游戏自己的 BinaryLZW 序列化容器；不能按 ZIP、zlib、DDS 或裸字体猜测解析，也不能用 Windows/ImGui 字体替代来声称复现游戏输出。

## 语料回归

本地 53 个 SRD：

```text
FONT records=249
inline CHAR mappings=0
TEXT records=1237
nonempty TEXT strings=1190
invalid nonnegative FONT indices=0
```

AdvertiseLogo 实际解析得到 2 个 FONT、多个英文/日文 TEXT；例如 warning 页的 `WARNING` 与版权说明均解析到 FONT `[1]` 的 32pt RFZ。

当前 `D:\sdhd\assets\data\surfboard` 完整目录的重新审计结果为：

```text
FONT records=365
TEXT records=1292
RFZ texts=1292
显式具有 0x78/0x36/0x7C/0x41 的 RFZ texts=1292
缺失上述静态排版输入的 RFZ texts=0
有效游戏模式 UTF-8=1292
当前已实现 token 子集=1292
```

因此 Rust 的静态 RFZ 输入转换器要求这四项属性显式存在；缺项直接返回错误，不需要也不允许为当前语料发明默认值。

## SrTextCast 排版与 glyph quad 路径

主虚表 `0x190E210` 的 `+0xBC` 进入 `sub_AD8D50`，字符串/样式状态更新后触发重建。实际 draw 入口 `sub_AD9160` 已继续闭环出以下调用链：

- `sub_AD8780` 按字节读取文本：低于 `0x80` 的字节直接成为 u16 code，高位字节与下一字节组成 big-endian u16 code；只有 `CR LF` 组合切分新行。它不是 UTF-8 解码器。
- `sub_AD8480` 对一行逐 code 查询 FONT `+0x50` 的两个 i16 映射，使用第一个值选择 540 字节 TEX 记录、第二个值选择 16 字节 CROP，并以 CROP 宽高和 TEX 像素尺寸累计行宽/行高；TEXT `+0x1C` 的字符间距会乘以 `count - 1` 加入宽度。
- `sub_AD86D0` 对所有 12 字节行记录调用上述测量并累计总高度。
- `sub_AD8620` 使用 TEXT flags `0x10/0x20` 选择垂直居中或底对齐，否则使用 TEXT `+0x18` 的原始 Y 起点。
- `sub_AD9160` 为每个 12 字节行记录调用虚表 `+0xC4`；SrTextCast 在该槽进入 `sub_AD9490`。
- `sub_AD9490` 使用 TEXT flags `0x04/0x08` 选择水平居中或右对齐，逐 code 再查 FONT/TEX/CROP，生成与当前 SrImage 路径相同的四顶点和两个相同 UV 通道。映射不存在时走明确的 16x16 fallback quad，而不是系统字体。

这里的 FONT/TEX/CROP 路径不是 RFZ 的前置转换结果。`sub_AD9160` 在进入上述逐字路径前先调用 `sub_46C4BD -> sub_AC5740`：

1. `sub_AC5740` 从 TextCast 状态读取字体资源名和 FONT 下标；
2. `sub_AC6440` 在 `SrRenderer +0x26C` 的字体资源树中按名字查找，并从该名字对应的 `shared_ptr<font::TextBox>` 数组按 FONT 下标取对象；
3. `sub_7BC820` 通过全局 `font::FontManager` 和 TextBox `+0x80` 的注册下标取得 `font::TextBoxObject`；
4. `sub_AC5740` 把 SrTextCast 的矩阵、颜色、对齐、裁剪和字符串状态写给 TextBoxObject；成功时返回 `1`；
5. `sub_AD9160` 对该返回值取反，只有 TextBox 路径返回 `0` 时才执行 `sub_AD8780/sub_AD8480/sub_AD9490` 的 FONT/TEX/CROP fallback。

`sub_AC6F50` 把 `TEXT.flags & 0x0C` 和 `TEXT.flags & 0x30` 传给 `sub_AC6BF0`，并把返回值写到 `TextBoxObject+0x2D0`。反编译与逐指令结果一致：水平组 `0/0x04/0x08` 分别贡献 `0/1/2`，垂直组 `0/0x10/0x20` 分别选择 `+0/+3/+6`，因此 9 个合法组合映射为 `0..8`。若任一组含同时置位的非法组合，函数回退为 `0`。Rust 的 `fennel_alignment_code_from_text_flags` 精确保留这张映射表和非法组合回退，没有把它简化成对所有位模式都成立的算式。

同一函数先调用 `sub_7C8DA0` 清除 `TextBoxObject+0x2E0` 的 `0x04/0x08`，再调用 `sub_7C8E80` 清除 `0x20/0x80/0x400/0x800/0x1000/0x2000/0x4000`。后续 `0..6` 模式 switch 会通过这两个 setter 重建对应位；已确认模式 `2..4` 会置位 `0x20`。`sub_7C90A0` 的逐指令分派顺序是 `flags & 0x20 -> sub_7C4070`，否则 `flags & 0x40 -> sub_7C5A20`，否则 `sub_7C1F90`；两位同时存在时 `0x20` 优先。Rust 的 `fennel_layout_dispatch` 只固化这张无语义命名的分派表，尚未把未闭合布局器接进 draw 路径。

flags 还有一条上层复制链。`sub_7BBD80` 构造的上层字体对象把 `+0x230` 初始化为 `3`；`sub_7BCB50/sub_7BCBA0/sub_7BCBD0/sub_7BCC00/sub_7BCC60` 分别修改与 TextBoxObject setters 对应的位；`sub_7BC1B0` 在 `0x7BC249..0x7BC24F` 把整个 `+0x230` DWORD 复制到新 TextBoxObject `+0x2E0`。对该上层对象方法区及原始 PE 的 `+0x230` 指令复核仍只看到构造、这些 setters 与复制，没有发现设置 `0x40` 的方法；现有 setters 的 `0xFFFF835F` 清除掩码会保留已有 `0x40`，但不会生成它。因此 `sub_7C5A20` 的真实入口来源仍未闭合，不能因为分派存在就假定任何 SRD mode 会进入它。

加载端与此完全对应。`srd_player_impl_load_project` 对每个 PROJ FONT 调用 `sub_AC4A80`。该函数为普通字体名建立并注册 `font::TextBox`，保存到同一个 `SrRenderer +0x26C` 资源树；但字体名包含 `.sbfont` 时明确跳过建立 TextBox。因此 `.sbfont` 必然进入旧式路径，而 RFZ 在 TextBox 可用时直接走 Fennel/Ruhuna；外部资源加载失败也会回退到旧式路径。二进制中不需要、也没有证据支持一个 RFZ runtime glyph 到 FONT/TEX/CROP 的写表桥。

`sub_7CB9B0` 已证明 RFZ Database/Glyph 到 128 字节 runtime glyph 的全部 box、bearing、advance、旋转和带一像素边框 UV 算法，细节见 [`ruhuna-font-archives.md`](ruhuna-font-archives.md)。`sub_F323B0` 再按 font slot 调用具体 FontResource 虚表 `+0x08` 取得该记录，并把 slot ID 写入 glyph `+0x04`。

`font::TextBoxObject::setTextByWideString` (`sub_7C90A0`) 随后把每个正常 glyph 转换为 116 字节布局记录，已证明的直接写入为：

| 布局偏移 | 来源 |
| ---: | --- |
| `+0x00` | 正常 glyph 为 `0`；runtime width 为零时为 `-2` |
| `+0x04` | runtime glyph 指针/不透明 token |
| `+0x08` | runtime glyph `+0x18` 的 atlas texture handle |
| `+0x0C` | token iterator 输出字段 |
| `+0x10/+0x14` | token iterator 的整数位置转 f32 |
| `+0x18/+0x1C` | 正常 glyph 为 runtime width/height 加两侧 `+0x24` 字段；零宽 glyph 使用 iterator 基准加 advance/line height |
| `+0x20` | token iterator 基准值转 f32 |
| `+0x24..+0x30` | 清零 |
| `+0x34..+0x40` | 当前四个 packed colors |
| `+0x44/+0x48` | `1.0/1.0` |
| `+0x4C..+0x68` | runtime glyph 四组 UV 原样复制 |

Rust 的 `FennelGlyphLayoutRecord` 固定为精确 116 字节，并已实现上述逐字段转换；仍未命名的 iterator 字段保留偏移名，不用猜测语义。

### 游戏实际字符串编码与 token 子集

FontManager 构造路径 `0x7B719C..0x7B71A4` 依次压入 `0x20` 和 `0`，调用 `sub_46B446 -> sub_F320F0 -> sub_F32B40`。后者把第一个参数 `0` 写到实现对象 `+0x04`，把第二个参数 `0x20` 用作最大 font slot 数，并把 `0x24` 写到 `+0x08`。因此本游戏的已执行配置是：

- 字符串编码模式 `0`；
- 最多 `0x20` 个 font slots；
- Fennel 控制前缀为 UTF-16 `U+0024`，即 `$`。

`font::TextBoxObject::setTextByString` (`sub_7C8F40`) 通过 `sub_F321C0` 读取模式。模式 `0` 明确调用 `sub_F525D0`：该函数按 UTF-8 continuation byte 解码，BMP 字符直接写一个 UTF-16 code unit，补充平面字符写 surrogate pair。转换成功后通过虚表 `+0x18` 进入 `sub_7C9070 -> sub_7C90A0`。所以 RFZ TextBox 路径不是旧 fallback 的“两字节 big-endian code”输入；当前游戏配置明确是 UTF-8。

`sub_F3BB80` 建立 token iterator，`sub_F3BD40` 逐 token 输出。当前完整 SRD 语料实际需要且已在 Rust 复现的分支为：

| 输入 | iterator 行为 |
| --- | --- |
| 普通 UTF-16 unit | `sub_F3DA10(..., type=0, code)`；code 写到 token `+0x08` |
| CRLF、CR、LF | type `3` 换行；CRLF 只产生一次换行 |
| `$$` | type `0` 的字面 `$` glyph |
| `$N` / `$n` | type `3` 换行 |
| `$t[x:y]` / `$T[x:y]` | T/t switch 的十进制有符号参数分支；把 x/y 写到 iterator 与 token `+0x20/+0x24`，不单独产生 glyph record |
| `$s` / `$S` | S/s switch 无参数切换 iterator `+0x820` bit `0x40000`，不单独产生 glyph record；后续普通 glyph 的输出状态拷贝把该 DWORD 放到 token `+0x10` |
| `$C` / `$c` | 无 `[` 时恢复 iterator 初始化时保存的四角颜色；`[hex]` 设置同一颜色到四角，`[h0:h1:h2:h3]` 分别设置四角。输入 DWORD 按高字节到低字节依次写入颜色对象 byte `0..3`，即 Rust 保存为 `value.swap_bytes()`；控制本身输出 type `2` 状态 token，不产生 glyph record |
| `$F` / `$f` | `[decimal]` 通过 `strtoul(...,10)` 后截断为 u16 写 iterator `+0x814`；无 `[` 时从保存值 `+0x850` 恢复初始 slot。控制不产生 record，后续普通 glyph 把该 slot 输出到 token `+0x04` |
| NUL、`U+001A` | iterator 结束 |

其他 `$` 命令仍明确返回 unsupported，不会被当作普通文字。当前完整 `surfboard` 语料的 1292 条 RFZ TEXT 全部通过游戏模式的 UTF-8 解码与上述 token 子集，因此当前实际输入覆盖为 1292/1292；未出现的控制命令仍不会被默认为普通文本。

颜色分支 `0xF3C5EF..0xF3C873` 使用 `sub_F3CB50` 以 radix 16 读取 `:`/`]` 分隔值，再通过 `sub_F25D30/sub_F25CE0/sub_F25C90/sub_F25BE0` 依次写颜色 byte `0..3`。短列表的实际 fallback 也已保留：两个值且第二个直接以 `]` 结束时第二值不提交，结果四角全取第一值；三个值时第三值不提交，结果为 `[first, second, first, first]`。Rust 只接受 1 到 8 位十六进制的闭合 bracket 形式；这覆盖已证明的正常格式，同时对 `strtoul` 的空串、符号、前导空白和超长异常输入继续明确拒绝，不把未审计的 CRT 边缘行为伪装成已支持语法。

font 分支 `0xF3BE62..0xF3BEB6` 的状态链也已闭合。`sub_F3BB80` 从 TextBoxObject `+0x04` 初始化当前/保存 slot 到 iterator `+0x814/+0x850`；普通 token 输出把 `+0x814` 放到 token `+0x04`，`sub_7C90A0` 随后以 `(slot, code)` 调 `sub_F323B0`。成功查找还会把 slot 写回 runtime glyph `+0x04`。Rust 的 `build_fennel_plain_record_stream_with_font_slots` 现要求调用方显式提供同样的 `(u16 slot, u16 code)` resolver；不会把 PROJ FONT 下标或文件名顺序当成全局 slot。现有单字体包装器只提供 slot 0，并对切换到其他 slot 的查找明确失败。

### FontResource 缓存、全局 slot 与请求顺序

FontManager 实现由 `sub_F32B40` 以 `0x20` 个槽构造。实现 `+0x2C..+0x34` 是 12 字节槽记录向量：首 WORD 保存槽号，`+0x04/+0x08` 分别保存已解析的字体接口和 backing object。`sub_F324D0` 只以 `+0x04 != 0` 判定占用；`sub_F323B0(slot, code)` 先做槽范围检查，再经该槽接口查询 glyph，并只在成功时把槽号写到 runtime glyph `+0x04`。

资源构造器 `sub_7B7300` 在 loader 成功建立 backing object 后调用 `sub_7BAF00`。后者从 0 开始逐槽调用 `sub_F324D0`，返回第一个空槽，因此分配规则是严格的 lowest-free，而不是 PROJ FONT 下标、FONT `0x71` 或文件名排序。随后 `sub_F33200` 注册该明确槽；析构器 `sub_7B7CC0` 先以对象 `+0x6C` 的槽号调用 `sub_F33310`，后者释放该槽所有 item 并把 `+0x04/+0x08` 清零，所以最后资源销毁后同一槽可立即由下一次 lowest-free 请求复用。

上游缓存也已闭合。`sub_7B96D0` 先通过 `sub_7BB0C0/sub_7BB210` 查找已存在的同一资源；命中时只增加现有缓存项引用计数，不重新构造 FontResource，也不分配新槽。`sub_7BBA60` 在最后引用释放时删除缓存项；FontResource 构造写入的虚表 `off_18E7CC8` 首项为 `0x4190FB -> sub_7B8370 -> sub_7B7CC0`，所以共享对象销毁明确进入上述 `sub_7B7CC0 -> sub_F33310` 注销链，并非仅凭 RAII 结构推断。满 32 槽时 `sub_7BAF00` 精确返回 `0x20`；`sub_F33200` 因越界返回失败，但构造器忽略返回值，所以该资源仍以 slot id `0x20` 进入缓存、却不占用 FontManager 槽。Rust 的 `FennelFontSlotRegistry` 保留了缓存复用、lowest-free、最终释放复用和这个满表边缘行为。

当前玩家自身的请求顺序来自 `sub_AAE6C0`，不是 PROJ FONT 表。函数从 `srd_player_get_runtime_scene_table` 取得 `SrPlayer::Impl+0x294`，按运行时 scene、其 layer 向量、layer 的 CAST 向量从头遍历；`srd_build_runtime_layer` 又按解析 NODE 顺序建立 CAST，因此这条顺序等价于原始 `SCN -> LAYR -> NODE` 顺序。每个虚类型 2 的 SrTextCast 先以主字体调用四槽 loader 的 local slot 0，随后按当前 NODE 最后挂接的 CATR 记录原顺序处理：

| CATR 名 | TextCast local slot | 资源请求 |
| --- | ---: | --- |
| `rubyFont` | 1 | 非空字符串调用同一资源缓存 |
| `rfzOutlineFont` | 2 | 非空字符串调用同一资源缓存 |
| `rfzOutlineRubyFont` | 3 | 非空字符串调用同一资源缓存 |

Rust 的 `collect_fennel_font_resource_requests` 复现这套区分大小写的顺序，且不按扩展名过滤：旧字体资源同样会占用全局槽，不能只统计 `.rfz`。`assign_fennel_font_resource_requests` 接受调用方提供的现有 registry，因此宿主可先加入更早加载的进程级字体资源。这里仍不声称某个原版游戏画面加载当前 SRD 时全局槽表为空；绝对 slot id 取决于同一进程中更早仍存活的字体资源，这是宿主生命周期输入，不在单个 SRD 文件内。

reference layer 不会在这条序列中追加请求。`srd_player_impl_load_project` 先完成 `srd_resolve_reference_scene_links` 和独立层构造，外层 `sub_AAC390` 才调用 `sub_AAE6C0`；但后者始终只遍历原始 runtime scene table。复制层的 TextCast 由同一个目标 parsed LAYR 重建，绘制时又通过自身文本状态中的 FONT 下标/资源名查询共享 renderer TextBox 树，所以使用目标层原始 TextCast 已请求的资源。完整调用顺序见 [`reference-runtime-recursion.md`](reference-runtime-recursion.md)。

对完整 91 文件语料逐文件从空 registry 审计得到 1292 次主字体请求、172 次首次资源构造和 1120 次缓存复用；没有出现上述三个 CATR 字体键，也没有满表请求，单文件最高注册槽为 4。旧 53 文件集合对应 1237/139/1098，最高槽同样为 4。这个统计只验证单个玩家请求序列和缓存关系，不把逐文件空 registry 当作原版整进程宿主状态。

`sub_7C90A0` 的 record stream 边界也已复现：普通 glyph 每个一条 116 字节记录；显式换行写 kind `-1`，超过 128 项 line-start 表时写 `-254`；iterator 结束后再写一个 kind `-1` 和最终 kind `-255`。缺字时游戏会尝试名为 `fennel_npc` 的 EmbeddedSprite；该 fallback 尚未闭环，因此 Rust 当前明确报缺字，不伪造替代 glyph。

### 默认静态排版输入、完整 mode-zero 路径与 fitting guard

TextBoxObject 构造函数 `sub_7BEB80` 先写 `+0x2D0=0`、`+0x2E0=3`、`+0x2E4/+0x2E8=200.0`。`sub_AC6F50` 清除模式位后，静态 SrTextCast 内部状态的 mode 初值为 `0`：

- TEXT `0x78 bit 0` 置位时，默认布局 flags 精确为 `3`；
- TEXT `0x78 bit 0` 清零时，`sub_7C8DA0` 再置 `0x04`，默认布局 flags 精确为 `7`；
- 两者都由 `sub_7C90A0` 选择 `sub_7C1F90`；bit `0x04` 只控制首行横向 auto-fit，不改变布局器选择。

构造初值与 SRD 写入来源现已分别闭环。`sub_AD8270` 在 TextCast `+0x1F4` 建立文本状态，并对状态 `+0x100` 调用 `sub_AE3580`；后者把扩展对象 `+0x08` 清零，因此 mode 构造值精确为 `0`。随后 SrTextCast 虚方法 `sub_AD9BF0` 按 NODE CATR 的原始 72 字节记录顺序调用 `sub_AB8720`；名称精确为 `FontParamData` 时，`noWrapPutMode#N` 经 `atoi` 后夹到 `0..6` 并写扩展 `+0x08`，即状态 `+0x108` / TextCast `+0x2FC`。同字段还可被 `autoScalingHeight#True/False` 写为 1/0，后出现的 token 覆盖先前值。

`sub_AC6F50` 先用 `sub_7C8DA0(0,0)` 清除 flags `0x04/0x08`，再用 `sub_7C8E80(0,0,0,0)` 清除 `0x7CA0` 模式位。若 TEXT `0x78 bit 0` 置位，函数跳过 mode switch；否则 mode `0..6` 在后续 FontParam flag setters 之前生成：

| mode | flags | 裁剪 |
| --- | --- | --- |
| `0` | `0x0007` | 否 |
| `1` | `0x000F` | 否 |
| `2` | `0x1CA3` | 是，Y 区间使用扩展形式 |
| `3` | `0x0CA3` | 是，Y 区间使用扩展形式 |
| `4` | `0x2CA3` | 是，Y 区间使用扩展形式 |
| `5` | `0x6C03` | 是，Y 区间为 `0..height` |
| `6` | `0x7C03` | 是，Y 区间为 `0..height` |

大于 `6` 的值走 switch default，保留低位 `3`。之后 `prohibition/wordWrap/monospaced` 总是分别覆盖 flags `0x01/0x02/0x200`；因此上表是 mode switch 中间状态，不是所有 CATR 应用后的最终值。Rust 的 `fennel_fresh_srd_textbox_flags` 保存上表，`fennel_srd_textbox_flags` 再复现三个 post-mode setters。完整 1,292 个 RFZ TextCast 的实际初始 mode 为 `0:1173, 2:12, 4:107`；最终 TextBox flags 有十种真实值，其中 119 个含裁剪 bit `0x400`，272 个含 monospaced bit `0x200`。

TextBoxObject 基类构造路径把 token iterator 的初始 x/y 状态 `+0x68/+0x6C` 清零。`sub_F2C670` 把 TEXT `+0x1C`（属性 `0x7C`）写到 TextBoxObject `+0x100`，`sub_F3BD40` 的普通 token 分支再把它复制到输出 `+0x6C`；`sub_7C90A0` 最终写入 layout record `+0x20`。同理，`sub_F2C620` 把 TEXT `+0x1E`（属性 `0x41`）写到 TextBoxObject `+0xFC`，作为行距；`sub_F2C5B0` 从 TEXT `0x36` 写入横纵缩放。因此 `FennelStaticTextProperties` 的来源为：

| Rust 输入 | 游戏来源 |
| --- | --- |
| `initial_x/initial_y` | TextBoxObject 构造初值 `0/0`；之后可被 `$t[x:y]` 改写 |
| `glyph_spacing` | TEXT `0x7C` → TextBoxObject `+0x100` → token `+0x6C` → record `+0x20` |
| `scale_x/scale_y` | TEXT `0x36` → TextBoxObject `+0xC4/+0xC8` |
| `line_spacing` | TEXT `0x41` → TextBoxObject `+0xFC` |
| `box_width/box_height` | 所属 CIMG 当前几何尺寸 → TextBoxObject `+0x2E4/+0x2E8` |

`sub_7C1F90` 的实现精确保留：

- 普通模式的视觉右边界为 runtime glyph `bearing_x + width`；水平 advance 为 `record.+0x20 + runtime advance_x`；行高取最大 `em_pixels_y`；
- 首行 auto-fit 使用 `f32::from_bits(0x3F7FBE77)` 的缩放偏置；静态 mode 0 不做统一 Y 缩放；
- 横向/纵向居中均使用 SSE `cvttss2si` 截断到整数后再转回 f32；
- 行距只插在第二行及后续行之前，既不加在第一行之前，也不追加到最后一行之后；
- 每个逻辑行维护当前 advance、视觉右边界、最近空格后的候选指针，以及候选保存时的“空格前” advance/视觉宽度；
- 当前 glyph 首次越宽且属于 FontManager `+0xE4` 集合时，仍把该 glyph 纳入当前逻辑行并只允许一次该状态；
- 其他越宽情况先检查前一个 glyph 是否属于 `+0xE0` 集合，命中时把断点回退一条记录；若存在空格候选，则候选断点最后覆盖该结果；
- 首 glyph 自身越宽且没有可回退断点时，游戏会产生不推进 record 指针的空逻辑行；后续由垂直边界终止，而不是强制把 glyph 塞入一行；
- flags `0x4000` 清零时，垂直检查使用 `abs(current_y) + line_height > box_height`，不包含当前行即将插入的行距；命中后把该逻辑行首记录 kind 改为 `-254` 并停止定位。fresh mode 5/6 的 `0x6C03/0x7C03` 置位 `0x4000`，因此完全跳过这些检查；
- 中/下对齐的测量 pass 若先命中垂直边界，会不写纵向 offset，随后定位出来的前缀因此保持顶对齐；
- 中/下对齐量写入 TextBoxObject `+0x108`，不会折进 layout record `+0x14`；`sub_7C7F90` 随后把它加到 TextBox 的 Y 平移。Rust 因此把 `textbox_vertical_offset` 与 record `y` 分开保存；
- `sub_7C90A0` 尾部扫描所有记录：没有 `-254` 时返回 glyph count；有标记时返回最后一个标记的记录下标，标记位于下标零时返回 `-1`。Rust 的 `record_limit` 保留这一结果。

### 默认排版的行元数据

TextBoxObject `+0x12C` 是 8 字节元素向量，`begin/end/capacity` 位于 `+0x12C/+0x130/+0x134`；`+0x34C` 是 16 字节元素向量，三指针位于 `+0x34C/+0x350/+0x354`。构造函数 `sub_7BEB80` 初始化后者，析构函数 `sub_7BEFB0` 释放并清零；`sub_7C1F90` 进入时把两个 end 都重置到 begin。

`sub_7C1F90` 的最终定位 pass 对每个非空视觉行锁步追加两个元素。空视觉行仍参与高度推进，但不追加元数据：

| 向量 | 元素字段 | 二进制来源 |
| --- | --- | --- |
| `+0x12C` | `float x_offset` | 当前行的水平对齐量 |
| `+0x12C` | `float y_bottom` | 当前行 bottom、累计 Y 与 TextBoxObject `+0x108` 纵向对齐量之和 |
| `+0x34C` | `record* first` | 当前视觉行第一条 116 字节 layout record 指针 |
| `+0x34C` | `u32 count` | 当前视觉行包含的 record 数 |
| `+0x34C` | `float advance` | 当前行各 record 水平 advance 的累计值 |
| `+0x34C` | `float line_height` | 当前行最大有效 em 高度；全零时使用已证明的 fallback 高度 |

中/下对齐通过测量与定位两 pass 完成：测量 pass 不追加，最终定位 pass 才写一次，因此不会产生重复元素。`+0x12C.y_bottom` 已包含 `+0x108`，而 glyph record `+0x14` 仍不包含该偏移。Rust 用 `FennelLinePosition` 保存 8 字节元素的语义；`FennelLineDescription.first_record_index` 以宿主无关的 record 下标替代原 x86 指针，不声称复制 32 位 ABI 内存布局。

消费者审计限定在已证明的 TextBox/Fennel 链：对 `0x7BE000..0x7CA000` 内 57 个函数的直接字段访问检查，以及 PE `.text` 中 `0x34C` displacement 的原始字节复核，只发现构造/析构与 `sub_7C1F90/sub_7C3940/sub_7C4070/sub_7C5A20/sub_7C7350` 等排版生产者。`sub_7C7F90` 和现有 batch 建立链不读取这两个向量；远处同 displacement 命中属于其他大对象字段或通用复制函数，不能据此认定为 TextBox 消费者。因此在当前游戏二进制的已闭合静态 draw 链中，这两组数据是保留的排版结果元数据，不改变已实现的 glyph 提交结果。

### Flag20 排版器 `sub_7C4070`

`sub_7C90A0` 在 `flags & 0x20` 时调用 `sub_7C4070`。当前两条已闭合的初始化链都只产生三个可达 Flag20 DWORD：`0x0CA3/0x1CA3/0x2CA3`；TEXT flags bit 0 置位时会在调用 mode switch 前跳过这些状态。Rust 因此只接受这三个 DWORD 且要求 TEXT bit 0 清零，其他组合明确报错。

`sub_7C1F90` 与 `sub_7C4070` 均为 `0x1487` 字节。逐基本块反编译和 1265/1276 条指令流对齐证明两者共享同一套 glyph 宽度/advance、固定字符表、空格候选、自动断行、行高、双 pass 对齐、`-254` 截止、record 定位和 `+0x12C/+0x34C` 追加状态机。需要保留的差异为：

- 默认函数在 `0x7C1FCF..0x7C1FEA` 从 flags bit `0x4000` 建立垂直截止 gate；已知 Flag20 DWORD 均不含该位，`sub_7C4070` 直接执行同一垂直检查；
- 已知 Flag20 DWORD 不含 `0x04/0x08`，所以不会执行 mode-zero flags `7` 的首行 auto-fit；
- `sub_7C4070` 在 `0x7C43C8` 把 TextBoxObject `+0x358` 清零；每次追加非空 `+0x34C` 描述后，`0x7C4B0B..0x7C4B29` 与 `0x7C51CE..0x7C51EC` 比较当前下标描述和新描述的 `+0x08 advance_width`。仅当前值严格大于新值时写入最新描述下标，`jbe` 路径使相等值保留较早下标；
- 两函数都保留 flags `0x200` 的字形度量/尾部修正分支；共享状态机已经复现该分支，但三个已知 fresh Flag20 DWORD 均不含该位，因此 `layout_fennel_static_flag20` 仍不接受未经上游来源证明的额外组合。

Rust 的 `layout_fennel_static_flag20` 复用已经逐字段闭合的公共状态机，并单独返回 host-independent `field_358`。完整语料中 TEXT bit 0 清零、因而允许进入 mode switch 的 RFZ TEXT 为 684 条；用已知 `0x0CA3` 状态逐条审计全部成功，其中 12 条得到非零 `+0x358`，最大下标为 1。该审计证明实现覆盖现有资源输入，不声称这些文本在原版首帧实际启用了动态 mode。

### fresh mode 1 的 X/Y 联动 auto-fit

fresh mode 1 生成 flags `0x000F`，仍由 `sub_7C1F90` 处理。它先与 mode zero 相同地用首条显式行计算 fitted X scale；flags `0x08` 置位时，若原始 `scale_y > fitted_scale_x`，`0x7C2192..0x7C21F4` 还把 effective Y scale 改为 fitted X scale，并保存 `fitted_scale_x / original_scale_y` 作为每条 record 的 Y multiplier。若原始 Y scale 不大于 fitted X scale，则 Y scale 和 record `scale_y` 都保持不变。

Rust 的 `layout_fennel_static_mode1` 只接受 TEXT bit 0 清零这一真实 mode-switch 前提，公共 prepare/position pass 分别保存 effective Y scale 和 record Y multiplier。完整语料中 684 条允许 mode switch 的 RFZ TEXT 全部完成 mode-1 假设布局；该统计同样不证明原版运行时实际选择了 mode 1。

### fresh mode 5/6 的默认排版

`sub_AC6F50` 对 fresh mode 5/6 生成 `0x6C03/0x7C03`，两者既不含 `0x20` 也不含 `0x40`，所以仍调用默认 `sub_7C1F90`。该函数在 `0x7C1FCF..0x7C1FEA` 仅当 flags `0x4000` 清零时执行垂直边界检查；mode 5/6 因而不会写入垂直 `-254` 截止。Rust 的 `layout_fennel_static_mode56` 只接受 mode 5 或 6 且要求 TEXT bit 0 清零，不把其他 flags 组合泛化进来。

若自动断行产生空逻辑行、record 指针不推进且 `0x4000` 又关闭垂直终止，原函数会重复同一状态。编辑器不能复制这种挂死，因此返回 `NonTerminatingWrapWithoutVerticalCutoff`，错误名明确描述原版控制流而不伪造排版结果。把 mode 5 假设应用到完整语料中 684 条允许 mode switch 的 RFZ TEXT 时，684 条全部终止且成功，没有命中该 guard；这仍只是资源覆盖审计，不证明原版运行时实际选择了 mode 5。

### flags `0x200` 的固定 cell 度量

`sub_7C1F90/sub_7C4070/sub_7C5A20` 的所有对应测试点都使用相同规则。flags `0x200` 清零时，视觉宽度使用 runtime glyph `bearing_x(+0x28) + width(+0x30)`，advance 使用 `advance_x(+0x3C)`；置位时，两者都改为 `cvttss2si(float(point_x(+0x0C)) * f32::from_bits(0x3FAAAAAA))`。行高仍读取 `em_pixels_y(+0x48)`，不被固定 cell 替换。

排版与两 pass 定位完成后，`0x7C333B..0x7C33F6` 遍历调用方的每条显式行，对行内每个非终止 record 执行 `record.x -= float(advance_x - fixed_cell) * 0.5f * effective_scale_x`；`0.5f` 的常量位为 `0x3F000000`。这会把原 glyph advance 居中到固定 cell 中，并且即使前面出现垂直 `-254`，仍按原显式行表处理对应 record。Rust 在 `FennelLayoutGlyphMetrics` 中补入 `point_x`，共享状态机复现度量与尾部修正；公开 mode/Flag20 入口仍保持各自证据白名单，未因内部已实现而擅自绑定状态字节 `+0x103` 的动态来源。

### Flag40 `sub_7C5A20` 的差分与来源审计

`sub_7C5A20` 大小 `0x1427`、1250 条指令；`sub_7C4070` 大小 `0x1487`、1276 条指令。完整指令流审计得到两者的外部 call target 多重集、字符串、全局引用和有效对象字段 displacement 一致，唯一新增对象字段为 Flag20 的 `+0x358` 五次访问：一次初始化、两组各两次的读/写最小值更新。Flag20 额外的两个 `comiss/jbe/sub/sar/dec` 组与这两次更新一一对应；其余差异是寄存器分配、vector append 分支排布和 NOP 对齐，不引入新的调用或对象字段。

flags 来源仍未闭合。对 Surfride 文本组件 `0x7B0000..0x7D0000` 的 rendered listing 分段扫描显示：上层对象 `+0x230` 只有构造初值、已枚举 setters 和 `sub_7BC1B0` 的整 DWORD 读取；TextBoxObject `+0x2E0` 只有构造、同一批 setters、该 DWORD 复制和排版/绘制读取。`sub_7C04F0` 的整 DWORD 写入只是在一次重建文本时以 `0xFFFF835F` 临时清位，随后在 `0x7C065C` 恢复原值，不生成新 bit；`sub_AC6F50` 的 mode 0..6 也不生成 `0x40`。远处 `sub_12E4B20` 的 `[edi+0x230]` 是包围对象中的另一字段，`sub_41E01F -> sub_F2C160` 写的是对象 `+0x98/+0x58`，均不能作为 TextBox flags 证据。因此当前只能闭合 Flag40 函数与 Flag20 的差分，不能声称本游戏存在已知可达的 `0x40` 状态，也暂不把它接入 Rust 布局入口。

`layout_fennel_static_fitting_lines` 仍作为原子 guard 保留：它在完整默认布局结果需要自动断行或垂直截止时返回明确错误且不修改输入，便于调用方只接受完整可见矩形；实际游戏路径由 `layout_fennel_static_default` 复现。

真实 1292 条 RFZ TEXT 与对应六套 RFZ runtime font 的逐条审计为：

```text
完整默认静态布局成功=1292
发生自动断行的 TEXT=9
自动生成的逻辑断行总数=16
最终含垂直 -254 标记的 TEXT=19
含非空行元数据的 TEXT=1242
行位置/行描述元素=1404/1404
单条 TEXT 最大行元数据数=9
fresh mode-1 假设布局成功=684
fresh mode-5 假设布局成功=684
fresh mode-5 非终止 guard=0
成功建立 texture batch 的 TEXT=1292
单条 TEXT 最大 texture batch 数=6
因 -254 停止 batch 扫描的 TEXT=19
成功建立 normal-glyph vertex batch 的 TEXT=1292
单条 TEXT 最大 normal-glyph 顶点数=1620
fitting-only guard 成功=1270
UTF-8/控制符/缺字/字体记录错误=0
```

此前 fitting guard 报告的 9 条断行与 13 条直接垂直边界现已全部进入默认游戏状态机；断行后又有部分文本触发垂直截止，因此最终 `-254` 文本数为 19。没有使用通用 Unicode line breaking、自动 fit-to-view 或简单裁切替代。

### layout record 到 texture batch

`sub_7C0D40` 清空已有 batch node 的两个计数和 record 指针数组后，按 TextBoxObject `+0xB0` 保存的完整 record 数顺序扫描：

- kind 为普通非负值时才进入 atlas 分组；`-1`、`-255` 等其他负值直接跳过；
- kind `-254` 立即返回，后面的记录不会进入任何 batch；
- TextBoxObject `+0x2C0` 为非负时，在 `processed + 1 >= maximum` 的当前记录之前立即返回；静态 SrTextCast 构造状态经 `sub_AE36C0 -> sub_AC5740` 明确把该值写为 `-1`，因此当前静态路径不启用该门限；
- 以 record `+0x08` 的 atlas texture token 为 key；同 texture 的 record 指针按出现顺序追加；
- node `+0x0C` 对每个普通 glyph 加一；record `+0x0C & 0x40000` 时 node `+0x10` 的 effect glyph 计数也加一；
- hash 为 wrapping `texture_token + (texture_token >> 3)`。

TextBoxObject 构造函数 `sub_7BEB80` 请求至少 11 个桶，prime table 首项为 `0x11`，所以初始桶数精确为 17。新 key 落入空桶时插入全局前向链表头；落入已有桶时插入该桶连续 node 组的最前端；已有 key 只追加 record，不改变 node 顺序。`sub_7C7F90` 从全局头开始沿 node `+0x00` 遍历，因此 Rust 保存的是这一原始遍历顺序，而不是 texture token 排序。

`sub_7C8BE0` 的 rehash 尚未移植；Rust 在第 18 个唯一 texture token 到达当前未覆盖域时明确报错。本地完整六套 RFZ 字体最多 7 个 atlas 页，1292 条真实文本每条最多产生 6 个 batch，全部严格落在不触发 rehash 的已证明域内。批次审计还确认全部 19 个垂直截止文本都在布局写入的同一 `-254` record 下标停止。

跨字体 atlas 的句柄链也已闭合。`sub_7CB9B0` 对 glyph 的 page 下标调用 `sub_EA4480`；后者经 `sub_EA4370/sub_E97A40` 从 renderer 纹理资源项 `+0x28` 取得不透明 32 位 texture handle，并写入 runtime glyph `+0x18`。`sub_7C90A0` 把它原样复制到 layout record `+0x08`，上述 `sub_7C0D40` 再直接以该值做 equality 和 hash；`sub_7C7F90` 在 `0x7C824B` 取 batch node `+0x08` 的同一 handle，并于 `0x7C829C` 写入 draw packet 的 stage-0 texture 字段。因此字体切换后不能继续把 page 下标当成全局纹理身份，也不能只用 TextCast 的主字体选择 atlas。

Rust 现在要求 `build_base_pose_fennel_draws` 接收一个 `FennelRenderResources`，其中的 `font_slots` 是已经包含宿主先存资源的 `FennelFontSlotRegistry`。TextCast 主字体从该 registry 取得初始全局 slot；`$F[n]` 按 `n as u16` 查询同一 registry，裸 `$F` 回到主 slot。内部 glyph token 用 `(slot, code)` 的无碰撞组合维持 layout/vertex 两阶段查找，runtime glyph 自身仍携带调用方提供的不透明 texture token。编辑器为每个已上传的 `(RFZ 资源, atlas page)` 分配唯一 token，并保存 token 到实际 `RuhunaD3d9AtlasSet/page` 的路由；提交时逐 batch 选择对应字体和页面，不再使用会在不同字体间冲突的 `page+1` 约定。

游戏 texture handle 的绝对数值来自进程内 renderer 资源项，不由 SRD/RFZ 文件决定；它与更早存在的宿主纹理生命周期一样属于外部运行时输入。Rust 精确保留 handle equality、32 位 wrapping hash、batch 顺序和路由语义，但不声称编辑器自行分配的数值等于某次原版进程的资源 handle。真实 RFZ 专项测试构造主字体、`$F[n]` 第二字体和裸 `$F` 复位，确认输出 batch 同时命中两个字体各自的 atlas token。完整 91 文件语料当前没有 `$F` token；接入 FontParam mode/shadow 与特殊 CAST matrix 后的初始帧回归为 557 draw/35466 vertices。

### FontManager 固定断行字符表

`sub_F33C30` 先清空 FontManager implementation `+0xE0/+0xE4` 的两个集合，然后：

- 把 `word_1940AB8` 的 45 个 UTF-16 值插入 `+0xE0`；`sub_F38DB0` 查询该集合；
- 把 `word_1940A9C` 的 14 个 UTF-16 值插入 `+0xE4`；`sub_F38D20` 查询该集合。

Rust 已按原始偏移名保存为 `FENNEL_FONT_MANAGER_SET_E0/E4`，并精确复现两个 membership 查询。断行实现直接遵循 `sub_7C1F90` 对当前/前一 glyph、一次越界状态和空格候选的组合顺序；没有把这两张表扩展成通用 Unicode 规则。

### 116 字节记录到 glyph triangles

`sub_7C10B0` 的无裁剪分支（TextBoxObject flags `& 0x400 == 0`）已经复现为 host-independent Rust 顶点生成器：

- 输出顶点精确为 28 字节：`float3 position`、4 字节 primary color、4 字节 secondary color、`float2 UV`；
- 四角索引表 `dword_18E8084` 的实值为 `[1, 0, 2, 3, 1, 2]`，即直接生成两个 triangle-list 三角形；
- 每个 UV 分量提交前加 `f32::from_bits(0x3727C5AC)`，即约 `0.00001`；
- primary color 取记录 `+0x34` 起对应角的 packed color，secondary color 取 TextBoxObject `+0x32C`；两个颜色都经过 `sub_F25B40 -> sub_5FF4A0`，在顶点内存中写为 BGRA byte 顺序；
- 局部四角由记录 `width/height`、独立调用参数 effective scale、`+0x24..+0x30` 和调用方 x/y 组成；随后按函数中的 4x4 row dot product 做 perspective divide。effective scale 不是重写后的 record 字段，而是 `sub_7C7F90` 计算的 `record.+0x44/+0x48 × TextBox.+0xC4/+0xC8`；
- `RuhunaD3d9AtlasSet` 已复用现有无 D3DX DDS 上传器，可把 RFZ/AVTS 中按 page 排序的 DDS 建为可 device-reset 重建的 D3D9 纹理。

`flags & 0x400 != 0` 的裁剪与 UV 重映射分支也已按 `0x7C1576..0x7C1698` 并入独立的 `build_fennel_clipped_vertices`：

- X 在矩阵变换前按 SSE 比较顺序夹到 `[0, TextBox+0x2E4]`；
- `TextBox+0x2E8` 先乘精确常量 `0.5f`。flags `0x4000` 清除此半高；再用 `0x80000000` sign mask 翻转下界符号。因此普通裁剪的 Y 区间为 `[-height*0.5, height*1.5]`，`0x4000` 分支为 `[-0.0, height]`；
- U range 精确取 record `uv1.u-uv0.u`，V range 取 `uv2.v-uv0.v`；每个裁后角使用 `(clipped-original_near)/(original_far-original_near)` 从共同 `uv0` 重算 UV；
- 裁后四角再走与无裁剪分支相同的 4x4 perspective divide、颜色转换、`[1,0,2,3,1,2]` 三角顺序和 UV bias。

Rust 还保留 MAXSS 在 NaN/相等输入时选择第二操作数的语义，以免把 signed zero 边界悄悄改写。该函数是证据完整的纯顶点分支；静态 `build_fennel_static_unclipped_vertex_batches` 仍只处理构造 mode 0，另由显式 runtime 入口承接调用方已经掌握的 flags 与 clip size。

随后对 SrTextCast 自身 `+0x1F4` 状态指针的所有直接消费者继续审计，定位到未被 IDA 自动建函数的 `0xADA300..0xADA348`。这是一条精确的显式 mode-6 控制方法：

- 参数顺序为 `enabled, value_to_state_130, value_to_state_134, value_to_state_12C`；
- 启用时先清除 `*(SrTextCast+0x1F4) flags bit 0`，再写 `SrTextCast+0x324/+0x328/+0x320`，最后写 `SrTextCast+0x2FC = 6`；
- 关闭时只设置该 flags bit 0 并写 mode 0，三个值保持不变；
- state `+0x130/+0x134` 随后由 `sub_AC5740` 进入三个非负提交量：`max((state.F4-state.130)*state.FC,0)`、`max(state.130*state.FC,0)`、`max(state.134*state.FC,0)`；state `+0x12C` 在 `sub_AD8D50` 的失败分支被复制到 state `+0xFC`。在更高层名字闭合前，Rust 保留这些偏移名，不把它们猜成通用 crop/scroll 属性；
- `off_190E210` 的 SrTextCast 虚表不含此方法。当前 IDB 对 `0xADA300` 仅有 jump-island `0x45335F` 的跳转引用，而该 thunk 本身无代码或数据引用。因此现有游戏内部没有已证明的调用点，不能把它接到 SRD 首帧或动画轨道。

Rust 现已用 `FennelSrdMode6ControlState`/`fennel_apply_srd_mode6_control` 固化上述显式 API，并增加 `build_fennel_normal_vertex_batches`。CATR 首帧路径则由 `layout_fennel_static_srd_font_param` 接收顺序解析后的 `FontParamData`，自动生成最终 TextBox flags、选择默认/Flag20 排版，并把所属 CIMG 尺寸作为 `+0x2E4/+0x2E8` clip size。原有窄包装器仍保留用于逐个函数状态审计，不因新增上游而接受任意 flags。

静态 mode-zero SrTextCast 的 normal-glyph 调用输入也已闭合：

- `sub_AE3580` 把 text state `+0x100` 的 correction mode、effect enable 和 effect offsets 清零；`sub_AC6F50 -> sub_F2C160` 因此把 TextBox byte `+0x98` 写为 0；
- FontObject 构造函数 `sub_F2BB60` 把 TextBox byte `+0x10C` 清零，静态初始化链不调用唯一的 `sub_F2C9C0` setter；
- 默认 flags 为 `3/7`，所以 `sub_7C04F0` 传给 `sub_7C7F90` 的 draw offset 精确为 `(0,0)`，并且不进入 `flags & 0x400` 裁剪；
- correction mode 0 的 glyph 修正为 `x=bearing_x`、`y=bearing_y-flag_mode-line_height+3`，最终两轴再减 glyph `enabled` 并乘 effective scale；
- TextBox position 由 `sub_AC6F50` 写为 `(-SrImage.origin_x,-SrImage.origin_y,0)`，Y 再加布局返回的 `+0x108`；
- 2D 分支通过 `sub_604010` 计算 `TextBox.+0x2EC * local_translation` 后交给 `sub_7C10B0`；3D 分支给 CPU 顶点生成器的只有 local translation，`+0x2EC` 另交 renderer 状态。

Rust 的初始 SRD draw 按已证明的 texture 前向链顺序执行 normal/effect 路径，并校验 record/runtime glyph texture token 一致。`diplayShadow=True` 的 110 个 TextCast 以初始 record bit `0x40000` 进入 effect-first buffer；颜色按 FontParam `shadowColor` 源字节 `2,1,0,3` 重排，位移取 `shadowX/Y`。完整语料 text `vertical=True` 为 0。

FontParam style 的初始 RFZ 下游现也已闭合。`sub_AC6F50` 从 FontObject 当前 style 开始，把 point X/Y 以 signed `max(value,1)` 写入两个 WORD；`sub_F2C100` 随后清除各自高字节，所以结果保留低 8 位。outline/italic/bold 分别设置 style flags `0x08/0x04/0x02` 和数值槽 `+0x10/+0x14/+0x0C`，style 构造基础 flag 为 `1`。`sub_F3BB80/sub_F3BD40` 把 style 带入每个 token；`FontDriverRFO` 虚表 `+0x08 -> sub_F3B780` 只读取 token style `+0x00` 的字符码调用 `sub_F41C50`，并只复制 `+0x08` flags 到 runtime glyph `+0x08`。它不以 point 或 numeric style 选择 glyph；`faceId` 在 `sub_AC6F50` 中没有读取。`sub_7C90A0` 原样把 flags 写到 layout record `+0x0C`，已知 RFZ batch/render 消费者仅检测 shadow bit `0x40000`。Rust 现在生成真实的默认 `1` 或 shadow `0x40001`，并保留任意 outline/italic/bold 输入对应的 flags/value 变换。

完整 1,292 个 TextCast 的 record style flags 为 `1:1182, 0x40001:110`；outline/italic/bold/faceId 非零数都是 0。14 个非默认 point 为 `28x28:12, 21x21:2`；pointY 的另一处读取位于 `sub_7C04F0`，仅在 flags 含 `0x800` 且不含 `0x20` 时减去 `pointY/2` 作边界比较，而这 14 个真实输入全部来自 mode 2/4 并含 `0x20`，因此初始路径不会进入该 margin 分支。

scroll 文本预处理和状态也已闭合。`sub_AD8D50` 先按索引顺序对 `$[0]..$[7]` 各执行一次 replace-all；`sub_5FDDB0` 每次替换后从插入串末尾继续，因此同一槽不会递归展开，但后续槽仍会处理先前替换新生成的 token。随后 `sub_AD9D80` 先处理 `$D`、再在删 D 后的结果上处理 `$L`。每种控制都先全串搜索大写形式，只有完全不存在大写时才搜索小写；只删除选中的第一个。若紧随 `[` 且之后存在 `]`，括号内容交给 CRT `atoi`，并连同完整括号删除；缺失右括号时只删两字节控制串；没有完整参数时保留调用方默认整数。FontManager 构造函数把 `$D` 默认值 `+0x38` 初始化为 `20`。

`$D` 命中时 text state `+0xF8` 取解析值，否则为 `-1`；`$L` 命中时 `+0xFC=0`，否则取 `scrollSpeed(+0x12C)`；`+0xF4` 总是清零。`sub_AD8D00` 在 Cast enable byte `+0x88` 非零时按 `(host+0x94 * argument) * 0x3C888889` 的 f32 顺序累加 `+0xF4`。`sub_AC5740` 再生成：

```text
TextBox +0x2C0 = F8 > 0 ? cvttss2si(float(F8) * F4) : -1
TextBox +0x2C4 = max((F4 - float(+0x130)) * float(FC), 0)
TextBox +0x2C8 = max(float(+0x130) * float(FC), 0)
TextBox +0x2CC = max(float(+0x134) * float(FC), 0)
```

`sub_7C04F0` 把最终二维 draw offset 作为参数 7 交给 `sub_7C7F90`，后者在 `0x7C84F5/0x7C84F9` 从 normal/effect glyph origin 两轴减去它。其滚动位移现已逐指令闭合：入口仅在 flags 含 `0x20`（横向）或 `0x4000`（纵向）时启用；`0x800` fit guard 横向比较实际几何宽与 clip width，纵向比较 `几何高 - floor(pointY/2)` 与 clip height，命中后临时以 `flags & 0xFFFF835F` 重排版并使用零位移。横向 `0x1000` 为 `fmod(+0x2C4 + clipWidth, clipWidth + textWidth) - clipWidth`；另一支实际重排版 `text + gap + text`，再以 `fmod(+0x2C4, measuredWidth - textWidth)` 循环。`0x2000` 的 gap 是 FontManager `+0x34` 个全角空格，构造默认值为 `3`；常量 CP932 `81 40` 经 `sub_103DC90` 转为 UTF-8 `E3 80 80`。纵向实际重排版 `text + "$n$n$n" + text`；`0x1000` 分支把 overflow、`+0x2C8` wait 与 `+0x2CC` tail 组成三段停留/移动函数，非 `0x1000` 分支只在 `fmod(+0x2C4, clipHeight + repeatedHeight) < repeatedHeight - textHeight` 时采用余数。所有 fmod 均来自 `sub_10461A0 -> UCRT _CIfmod`。

Rust 的 `prepare_fennel_srd_runtime_text`、`build_fennel_srd_repeated_text` 与 `prepare_fennel_srd_draw` 已分别固化上述文本、辅助串和位移/重排版决策；无法证明的 `atoi` 溢出不会伪造 CRT 结果，而是显式报错。`measure_fennel_srd_text_size_mode0` 进一步按 `sub_7BFAB0` 的 batch 遍历、maximum 截止、bearing/correction、scale、effect-origin 选择和正向最大值比较生成实际几何尺寸；draw-list 现在会按 TextBox 的真实对象状态选择原串、循环串或 `flags & 0xFFFF835F` 的原串二次排版，并在 fit guard 后恢复原 flags 作裁剪/绘制。`build_base_pose_fennel_draws` 接受每个 `(layer,node)` 的 8 个替换槽、default D、repeat-space count 与当前精确 `F4`；它不会把编辑器 animation frame 猜成游戏 host clock。initial 包装器仍在遇到 `$[0]..$[7]` 时明确拒绝，而不是假定空替换值。

完整语料的 scroll 三元组有六种，但解码后的 1,292 个 TEXT 中 `$D/$L` 都为 0；因此首帧 `F4=0`、maximum glyphs=`-1`，mode 2/4 的 fmod 输入 `+0x2C4=0`，两轴 draw offset 精确为零。完整 91 文件构造初态回归仍为 557 draw/35466 vertices，均为 2D；这不是 3D TextCast 不可达的证据。`LinkedVERSE_Gate` 完整整数状态审计在 `ANMS[10] frame 1 / LAYR[0]/NODE[149]` 首次得到实际可绘制 3D TextCast，runtime builder 产生 498 个顶点并提交闭环的 3D packet matrix。Advertise D3D9Ex 像素哈希仍为 `7B466FC4B4E0EC9A`。动画集 Fennel API 已复用 ProjectRuntime 的 layer enable、CAST transform/color、特殊 matrix 与 SrImage geometry；project layer/Cast vector/RefCast 结构递归、copied layer 独立 Image/Text draw、runtime gate、mixed CAST 顺序、命名-target 可见性剔除和 target planner 均已接线。右侧 Properties 以 `(runtime owner, NODE)` 保存显式替换/scroll 输入。剩余边界是何种已证明的游戏宿主事件驱动 `F4` 累加，以及尚未闭合的特殊布局路径。

`sub_7C7F90` 的 `record+0x0C & 0x40000` 第二组 effect glyph 下游也已闭合：

- `sub_7BF0C0` 先把同一份 116 字节 record 完整复制两次，normal 与 effect 各持一份；
- `sub_F2D730` 对 effect copy 的四个 `+0x34..+0x40` 颜色逐角处理：DWORD RGB 整体替换为 TextBox `+0x80..+0x8C` 的 effect color，alpha 则严格执行 `(effect_a/255)*(normal_a/255)*255` 后 `CVTTSS2SI`；
- `fennel::FontObject` 构造函数 `sub_F2BB60` 把这四个 effect color 初始化为 `0xFF000000`，并把 TextBox `+0x90/+0x94` 初始化为 `(2.0,2.0)`；`sub_F2C890` 与 `sub_F2C640` 是对应的显式 setter；
- effect origin 是 normal origin 再加 `+0x90/+0x94`。两次调用使用相同 effective scale、矩阵、TextBox flags 与 clip size，因此 effect 同样进入已实现的 clipped/unclipped `sub_7C10B0` 分支；
- effect 调用虽然计算并传入 `TextBox+0xC0+0.1f`，但 `sub_7C10B0` 的该栈参数在整函数中没有任何读取，Rust 不制造无效的 Z 行为；
- batch node `+0x10` 是 effect 数量。`sub_7C7F90` 令 effect 写指针从 buffer base 开始，normal 写指针从 `base + effect_count*0xA8` 开始，所以最终提交顺序精确为全部 effect vertices 在前、全部 normal vertices 在后。

`sub_F3BD40` 的 `$s/$S` case 在 `0xF3C095..0xF3C0BD` 对 iterator `+0x820` bit `0x40000` 做无参数 toggle。普通 glyph 输出在 `0xF3C89B`/`0xF3C91F` 从 iterator `+0x818` 起复制 16 字节到 token `+0x08`，所以 `+0x820` 精确落在 token `+0x10`；`sub_7C90A0` 再原样复制到 record `+0x0C`。Rust tokenizer/record builder 已复现该状态链，`build_fennel_normal_vertex_batches` 生成上述 effect-first 分段，并由调用方明确提供 effect colors/offset；静态包装器使用构造默认值。

### teaFontRenderer batch 与 format 13

`teaFontRenderer` 的 RTTI/vtable 从 `0x18E7EC8` 闭环；虚表 `+0x34` 是上述 `sub_7C10B0`。`sub_7C04F0` 最终直接调用实际绘制函数 `sub_7C7F90`。后者按 texture batch node 迭代，在 `0x7C8284..0x7C82C6` 完成以下步骤：

- 把 batch texture 写到内嵌 DrawPacket texture slot 0；slot 1/2 清零；
- 以 vertex format `13`、primitive `3`、`glyph_count * 6` 个顶点和调用方 `is_2d` 配置 renderer `+0x150` 的动态 batch；
- 每个普通 glyph 消耗 `0xA8 = 6 * 28` 字节；record flag `0x40000` 时再保留一组 `0xA8` 并第二次调用 `sub_7C10B0`；
- 虚表 `+0x10` 进入 `sub_6DF020`，把 `is_2d` 复制到 DrawPacket `+0x60 bit 7` 后提交。

`sub_671D30` 的 vertex declaration switch 中 case 13 精确注册：

1. POSITION float3；
2. COLOR0 D3DCOLOR；
3. COLOR1 D3DCOLOR；
4. TEXCOORD0 float2。

case 14 在这四项之后才增加 TEXCOORD1。因此 Fennel format 13 精确是 SRD format 14 的前 28 字节，而不是根据 `sub_7C10B0` 输出形状反推的自定义声明。Rust 的 `FENNEL_D3D9_VERTEX_DECLARATION` 和 `FennelRenderVertex::STRIDE` 已固定并测试该布局。

### DrawPacket、ShapeEnv 与 shader bytecode

字体 renderer 构造后默认选择 render preset 3。DrawPacket 构造、renderer 初始化和 `sub_7C7F90` 默认模式的最终值为：

```text
draw_flags_00 = 0x02AFE003
flags_60      = 0x00004020（2D 时再置 bit 7，成为 0x000040A0）
flags_64      = 0x00000000
field_28      = 31
field_2c      = 0
vertex_format = 13
textures      = [present, null, null]
```

由已经复现的 ShapeEnv key 与 Simple selector 得到：

| batch | ShapeEnv low:high | compact key |
| --- | --- | --- |
| 3D | `0x0036BFB0:0` | `AAMAAABAABGAAAAAAA` |
| 2D | `0x0036BFB8:0` | `EAMAAABAABGAAAAAAA` |

使用原游戏 `cg.dll` 和 `chusanApp.exe` 内的精确 Cg source 只做离线编译，两个 key 均通过 D3D assemble 和 HAL `CreateVertexShader/CreatePixelShader`。输出为：

| shader | bytes | SHA-256 |
| --- | ---: | --- |
| 3D VS | 332 | `86669F24505A70D6DB560C6B2838EBA7D262B0206825BA3927658AB5A7112D61` |
| 2D VS | 384 | `A3E0CA2EFA3452A529DDE7E92EB638FAAD4A230DF5A5537D2D50BE53BC045BD4` |
| shared textured PS | 248 | `066761E3FE149084A9526FDD1A091138B9DC894EAC29FA707D71992E4ED4E23F` |

这些 bytecode 分别与项目中已有的首个 3D VS、2D VS 和单贴图 PS 逐字节相等。`shader_bytecode.rs` 因此把两个 Fennel key 映射到相同的已验证数组，不嵌入重复副本。编辑器运行时不加载 Cg，也不使用 D3DX。

### RFZ atlas sampler

每个 RFZ 的 AVTS entry 0 是一个 Stevia YABX 数据库；其中每个 DDS atlas page 都有一个 `stevia::Texture` 对象。该类序列化字段前缀明确是 `_wrapU/_wrapV/_minFilter/_magFilter/_mipFilter/_anisoNumber/_lodBias`。本地完整六套字体的每一页均为：

```text
[2, 2, 1, 1, 0, 1, 0]
```

这组值也与 `ceylon::resource::Texture` 构造函数 `sub_E7E1E0` 在 `+0x18..+0x30` 写入的默认值逐项相同。`ceylon_texture_sync_sampler_state` (`sub_E7E8B0`) 把它们写入 backing sampler，`d3d9_flush_texture_sampler_state` (`sub_E59410`) 再明确提交：

- type 1/2：ADDRESSU/V，经地址表把内部 2 映射为 D3D9 Clamp；
- type 6/5/7：MIN/MAG/MIPFILTER，经 filter 表把内部 1/1/0 映射为 Linear/Linear/Point；
- type 9/10/8/4：MAXMIPLEVEL、MAXANISOTROPY、MIPMAPLODBIAS、BORDERCOLOR。

字体 DDS 当前均为一层 mip；`ceylon_image_initialize_from_stevia_request` 在 `0xE7FF5F..0xE7FF6E` 把零 mip count 至少提升到 1 并写到 image `+0x64/+0x68`，随后 `sub_E7E8B0` 将其用于 MAXMIPLEVEL。因此当前 atlas 的完整 D3D9 sampler 是 Clamp/Clamp、Linear/Linear/Point、max mip level 1、max anisotropy 1、LOD bias 0、border color 0。Rust 会解析并验证每个 Stevia texture object；页数不匹配、页间 sampler 不同或出现未证明值时直接报错。

`FennelDx9Renderer` 已实现可 ResetEx 失效/重建的 format 13 declaration、精确 shader 对、按需增长的动态 DEFAULT-pool vertex buffer、上述完整 stage-0 sampler、preset 3 的 blend/raster/depth 状态，以及 `D3DPT_TRIANGLELIST(vertices / 3)` 提交。默认静态 layout record、`sub_7C0D40` texture batch membership、normal-glyph CPU vertices 和初始 SrTextCast world/color 均已连接到 Composition。

### ShapeEnv material cull 与像素闭环

Fennel packet 的 `draw_flags_00 = 0x02AFE003` 设置了 `0x00800000`，因此 `ceylon_apply_draw_packet_state` 不会强制 `CULL_NONE`，而是保留 ShapeEnv material 同步出的基础 cull。该基础值的完整来源为：

1. `ceylon_create_shape_environment` (`0x670680`) 创建 `PrimitiveDummyShape`，并以 `sub_E87890` 新建 `ceylon::resource::State` 后交给其 `sea::Material`；
2. State 内的两个 52 字节 `StateParam` 均由 `sub_E859B0` 构造，packed `+0x08` 的低三位为 `0`；
3. Material override mask 初始为零，`sea_material_sync_render_commands` (`0x659810`) 因而从该默认 StateParam 提取低三位到 material command `+0x54`；
4. `sub_E93530` 把它写入全局 RenderState `+0x60`；内部值 `0` 经原版表映射为 `D3DCULL_CW`。

此前直接用全局 RenderState reset 默认值 `1`（`D3DCULL_CCW`）会把 Fennel 的右上→左上→左下三角形全部剔除，导致顶点、alpha 和 viewport 均正常但 Composition 没有任何 changed pixel。这个失败只用于定位，最终实现使用上述二进制闭环得到的 `D3DCULL_CW`，没有保留诊断性的 `CULL_NONE`。

真实语料初始帧审计得到 557 个可见 2D Fennel draw、35466 个顶点。独立 `--srd-fennel-smoke` 对 `CHU_UI_Advertise_00_v10.srd` 仍为 1 draw、834 vertices、两个 atlas batch；D3D9Ex Composition 在强制 `ResetEx` 前后均得到 40920 个 changed pixels、`white_pixels=0`、bbox `(651,396)..(1271,683)` 和 FNV-1a `7B466FC4B4E0EC9A`。哈希仅作为当前设备上的稳定诊断值，不定义为跨 GPU 像素规范。

普通编辑器路径现已不再停留于该独立 smoke。对用户截图对应的 scene 0、`AS_warning_in`、frame 24、AdvertiseLogo/MainScene、present `1080x1920`、screenParam `1920x1080`，统一 runtime builder 精确产生 22 个 source、2 个 target group，其中 20 个是 Fennel source、共 3072 个字体顶点；这 20 个 source 的 packet/layer/atlas token 相同，现已物理拼成一次 3072 顶点 triangle-list draw。隐藏 `--srd-runtime-smoke=0,0,24` 在同一个 Composition pass 中先渲染完整 Image/Slice/Number/Fennel stream，再关闭 Fennel 重绘并比较 RGB；两次（强制 `ResetEx` 前后）均有 73588 个像素因这条合并 list 而变化。这证明文字已经进入正常预览的 D3D9Ex target stream，而不是只在字体专项窗口中成立。

外部 Ruhuna 字形仍不是一条可以随意替换的 ImGui 文本路径。编辑器将上传 RFZ 内嵌 DDS atlas 并提交游戏布局记录对应的 glyph quad；不会使用系统字体冒充。

## 下一证据目标

- `sub_7C5A20` Flag40 排版器及 flags bit `0x40` 的真实写入来源；
- 游戏宿主在当前玩家之前仍存活的 FontManager/renderer 资源状态、其余控制 token（EmbeddedSprite 等）及 `fennel_npc` 缺字 fallback；
- FontParam `vertical` correction、`$D/$L` 动态解析及非零时钟下的完整循环位移，以及 mode 6 显式 API 的真实调用点；
