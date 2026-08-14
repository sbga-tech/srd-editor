# SrPlayer target 路由、全局队列与宿主相机边界

Chusan `AdvertiseLogoObject` 与 `CommonBackGroundObject` 的具体 SrPlayer 成员、初始化属性和 identity 根节点证据分别见 [`chusan-advertise-logo-player.md`](chusan-advertise-logo-player.md) 与 [`chusan-common-background-player.md`](chusan-common-background-player.md)。

本页记录 `projView::SrPlayer` 生成的 Ceylon draw packet 如何进入命名 target 或全局队列，以及空 `TargetScene` 时为何不存在唯一的游戏相机。结论只来自当前游戏二进制。

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`

## SrPlayer 属性与 renderer 配置参数

`srd_construct_player` (`0xAA68B0`) 注册的相关属性为：

| 属性 id | 名称 | 构造默认值 |
| ---: | --- | ---: |
| `1` | `DrawTargetSceneOnly` | `false` |
| `2` | `TargetScene` | 空字符串 |
| `3` | `FirstCalcMatrix` | `false` |
| `4` | `DrawMask` | `0xFFFF` |

`DrawMask` 的注册调用在 `0xAA6B4D..0xAA6B67`，参数同时给出最小值 `0`、最大值 `0x7FFFFFFF` 和步长 `1`。Advertise 的 common init (`0xC955D0 -> 0xBA6490`) 另外把属性 1 设为 `true`，但没有写入属性 2，因此它保留空 `TargetScene`。

`srd_player_impl_prepare_renderer` (`0xAACA80`) 的参数顺序可由调用点和 `srd_renderer_configure_project_camera` (`0xAC7400`) 的接收端共同闭环：

```text
target pointer       -> SrRenderer + 0x248
SrProject            -> 相机配置输入
FirstCalcMatrix      -> SrRenderer + 0x0B8，12 个 f32
Color                -> SrRenderer + 0x0E8
DrawMask             -> SrRenderer + 0x190
DrawTargetSceneOnly  -> SrRenderer + 0x25C
global mode byte     -> SrRenderer + 0x25D
```

ImageCast packet 建立时，`sub_AC5740` (`0xAC5CB2`) 把 `SrRenderer+0x190` 开始的 16 字节复制到 vertex builder `+0x1F0`。最终 packet 的同一组字段位于 `+0x80..+0x8F`；因此其首个 dword 是 SrPlayer 的 `DrawMask`，不是相机矩阵或颜色。

## 命名 target 与空 target 的提交分支

`sub_AC5F70` (`0xAC5F70`) 在需要提交 vertex batch 时调用 vertex builder 虚表 `+0x10`：

- `DrawTargetSceneOnly != 0` 时传入 `SrRenderer+0x248`；
- 否则传入空指针。

vertex builder 的该虚函数进入 `ceylon_submit_vertex_batch` (`0x6DF020`) 和 `ceylon_enqueue_draw_packet` (`0x670BE0`)：

- target 非空：调用该 target 虚表 `+0x60`；
- target 为空：调用全局管理器 `unk_1CA0BE8` 的 `sub_6314F0`。

`TargetScene` 的查找是精确字符串 map lookup，没有 fallback；Advertise 没有设置该属性。Scene 管理器的名称 map 构造为空，唯一 target 插入入口的 18 个调用点已经审计；正常运行时名称均非空，且本地完整 293 个 AFB 不含 `star::SglScene` 类型键 `0x005B916A`。因此 Advertise/Common 的空字符串查找结果精确为 null，packet 进入全局管理器。完整证据与未初始化可见性矩形边界见 [`render-visibility-culling.md`](render-visibility-culling.md)。

查找结果为 null 时，`srd_renderer_configure_project_camera` 跳过整个 target Width/Height/Camera 分支，保留 `SrRenderer+0x08/+0x48` 的构造 identity。最终接收全局 packet 的 Scene Camera 是 target-local shader environment 的另一路输入，不能反过来冒充 renderer preparation 的命名 target。null 分支矩阵见 [`render-empty-target-matrices.md`](render-empty-target-matrices.md)，未初始化可见性矩形边界见 [`render-visibility-culling.md`](render-visibility-culling.md)。

## 全局命令记录布局

`sub_6314F0` 取得全局管理器 `+0x118` 的队列对象并调用 `sub_63E380`。后者构造固定 `0x38` 字节记录：

| 记录偏移 | 来源 |
| ---: | --- |
| `+0x00` | 命令类型 `1` |
| `+0x08` | packet `+0x28` |
| `+0x0C..+0x1B` | packet `+0x80..+0x8F`，首 dword 为 `DrawMask` |
| `+0x2C` | 原 packet 指针 |
| `+0x30..+0x37` | 零 |

追加函数 `sub_6041C0` 的扩容、拷贝和尾指针更新均以 `0x38` 为步长。`sub_63E370` 把队列尾指针恢复到首指针，证明它是帧内清空操作。

## 全局队列按注册 target 分发

全局队列的消费者是 `sub_631550`。管理器 `+0x114` 是注册 target 的 map；函数对同一组 target 执行两个阶段：

1. 第一轮调用每个 target 虚表 `+0x4C`；
2. 第二轮调用虚表 `+0x3C`，随后把全局 `+0x118` 队列传给虚表 `+0x44`；
3. 所有 target 处理结束后，调用 `sub_63E370` 清空全局队列。

所以 target pointer 为 null 的 packet 语义是“交给每个注册 target 做自己的过滤与入队”，不是“选择一个隐藏的默认 target”。property 字符串为空是否产生该 null pointer 仍待闭环。

RTTI 和虚表交叉引用证明共享这套基础处理的类型至少包括：

```text
sea::BasicScene
sea::DefaultScene
air::Scene
air::DebugCameraScene
sea::DebugScene
adx2::SofdecScene
sea::WaterScene
```

`sea::ShadowParalellScene` 也参与同一框架，但覆盖了部分命令处理虚函数。

## 每个 target 自己过滤，并拥有自己的 Camera

基础虚表 `+0x44` 是 `sub_601C60`。它先检查 Scene `Enable` byte `+0x10`，再读取 property 2 `DrawIndex`，构造 `(1 << DrawIndex) & 0x7FFFFFFF`；结果非零才调用 target 虚表 `+0x40`。基础 `+0x40` 是 `sub_601CB0`，读取 property 3 `Attribute`，逐个遍历 `0x38` 字节命令记录，并由 `sub_63EA50` 同时检查：

- 当前 target bit 与记录 `DrawMask` 是否相交；
- packet `+0x58` 低字节是否包含 `1 << Attribute`；
- packet `+0x60 bit 0x100`、Attribute 1 专用的 `bit 0x04`，以及 command `+0x10 bit 0x02`。

`sub_601CB0` 还调用当前 target 虚表 `+0x58`。`sea::BasicScene`、`sea::DefaultScene` 和 `air::Scene` 的该槽最终进入 `sub_603970/sub_603980`，返回 target `+0x90` 保存的 Camera 指针。基础命令处理 `sub_601DE0` 使用传入的 target-local camera/context 做可见性和排序，之后才把记录加入该 target 自己的 draw queue。

独立的 shader 环境链也以 target context 为入口：`sea_all_env_basic_pre_update` (`0x6E19A0`) 先取得所属 target，再调用其虚表 `+0x58` 取得 Camera；`sea_all_env_basic_update_values` (`0x6E1400`) 才把该 Camera 的 `Projection*View` 写入 `mtxPrjView`，即 Simple VS `c10..c13`。

这两条链共同排除了“把 SRD CAM 的 Projection*View 无条件写入每个 2D draw 的 c10..c13”。SRD CAM 参与 `SrRenderer+0x08/+0x48` 的坐标转换；最终 shader 环境则属于实际接收该 packet 的 target。

## CAST 调用顺序与最终提交顺序的边界

ImageCast (`srd_render_image_cast`) 与 Fennel TextCast 的每个 texture batch (`sub_AD9490`) 最终都调用 `srd_begin_quad_draw -> sub_AC5F70`。`sub_AC5F70` 结束 vertex builder 后进入 `ceylon_submit_vertex_batch` (`0x6DF020`) 和 `ceylon_enqueue_draw_packet` (`0x670BE0`)；所以 RefCast 递归产生的 CAST 调用顺序确实也是 packet 初次 enqueue 的顺序。空 target 时，`sub_63E380` 又以 `0x38` 字节记录前向追加到全局命令向量。

这仍不等于唯一最终 GPU 顺序。后续队列容器现已闭环：`sub_64BAB0` 选择第一个匹配 rule index，`sea::SceneModelModule` 对每个 pass 稳定前向追加，target 再按 32 个 EntryInfo 的数组顺序及各自 inclusive pass range 前向提交；这条链不存在额外 comparator sort。完整指令证据与 Rust planner 见 [`render-target-pass-order.md`](render-target-pass-order.md)。

type-1/SRD command class 与首个 rule 匹配算法也已闭环；MainScene/BgScene 共用的 5 项默认 BasePass rule 和 32 个 EntryInfo 映射也已从 `air::Scene` 构造静态表及 `sub_64F970` 写入链闭环。普通 Image/Slice/Number/Fennel packet 还已证明始终是 class 3 / attribute group 0，而默认第一项 Back2DPass 对完整 u16 order 域恒真，所以这部分不再需要伪造 depth/order。Rust 可把一个 Image、每个 Slice cell、每个 Number glyph 和每个 Fennel texture batch 展平为逻辑 command，先执行已证明的 type-1 filter，再生成 target-local 顺序。

Chusan 的 MainScene/BgScene 构造完成状态也已闭环：两者 Scene Enable 均为 true、Attribute 均为 0，DrawIndex 分别为 0/16；Advertise 与 Common 的 DrawMask 都是 `0xFFFF`，因此两者的初始普通 packet 均被 MainScene 接纳、被 BgScene 拒绝。注册调用的第三个布尔量只控制 manager `+0x11C` current-target 指针，不是 Enable。普通 Image/Slice/Number/Fennel 的相邻 enqueue merge 已实现并物理提交，包括完整 renderer layer key、format-14 triangle-strip 的两个退化连接顶点和 Fennel triangle-list 的直接顺序拼接。尚缺的是后续帧可能发生的 Enable/manager-current 切换时序、其他 target 配置、真正依赖 depth/order 的其他路径，以及 stencil/special-depth merge 状态。因此 `RuntimeCastDraw` 本身仍只表示 CAST render invocation；merged planner 只对其声明的普通路径给出精确 record 数。

## Advertise 的已证明结论

Advertise 的嵌入式 `projView::SrPlayer`：

- `DrawTargetSceneOnly=true`；
- `TargetScene` 为空；
- 默认 `DrawMask=0xFFFF`，除非外部对象属性系统随后显式覆盖；目前构造、common init 和资源加载路径均未发现覆盖；
- 因而先进入全局队列，再由当帧已注册的 target 逐一过滤。

对 Advertise 的具体 Chusan 构造完成时刻，Main/Bg 初始接纳结果已经确定。单独的 `.srd` 文件仍不包含“后续帧有哪些 target 注册/Enable、各自 Camera 和 viewport 是什么”这些宿主状态；编辑器若脱离具体宿主打开任意 SRD，仍无法从文件本身恢复唯一最终相机。

## Common background 的已证明结论

`CommonBackGroundObject` 也保持空 `TargetScene`、`DrawTargetSceneOnly=true`、`DrawMask=0xFFFF`，以全局名称注册/查找，嵌入式 SrPlayer 没有 GraphNode parent，根矩阵为 identity。其 property 6 为 `2DLayer=6`，所以根 renderer key 为 `0x8680`；这不会改变 SRD 内 LAYR/CAST 的 3D 判定。对象、启用生命周期、初始 Main/Bg 过滤和样本回归见 [`chusan-common-background-player.md`](chusan-common-background-player.md)。

## 编辑器边界

编辑器必须把以下两类数据分开：

- SRD 证据：CAM、`FirstCalcMatrix` 输入接口、`DrawMask`、`SrRenderer+0x08/+0x48` 的数学；
- 宿主预览配置：目标 target 类型、viewport、target Camera、active/visibility 位和 scene-node 根矩阵。

在用户选择宿主预览策略前，代码不得把 `MainScene`、`BgScene`、单位矩阵、自动正交相机或 SRD CAM 冒充为游戏唯一默认值。

Chusan 自己创建的 `MainScene` / `BgScene` 已进一步闭环到 scene 属性、Camera 构造值与运行时 Aspect 更新；它们现在可以作为显式 target profile 使用。证据与仍需外部提供的 present 尺寸见 [`chusan-air-scene-profiles.md`](chusan-air-scene-profiles.md)。
