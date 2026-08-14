# 空 `TargetScene` 的 renderer 矩阵与全局 target Camera

本文记录 property 2 查找结果为 null 时，renderer preparation、ImageCast 顶点、draw packet 矩阵和最终接收场景 Camera 之间的精确边界。Advertise/Common 的空字符串 lookup 已闭环为 null；其未初始化可见性矩形见 [`render-visibility-culling.md`](render-visibility-culling.md)。

## renderer preparation 的短路

`srd_player_impl_prepare_renderer` (`0xAACA80`) 先按 property 2 的精确字符串查找 target，再把结果作为第一个普通栈参数传给 `srd_renderer_configure_project_camera` (`0xAC7400`)。名称 map 构造为空、唯一插入入口及其 18 个注册点均已审计，本地 293 个 AFB 也没有 `star::SglScene` 类型记录；因此 Advertise/Common 的空字符串得到 null。

`srd_construct_renderer` (`0xAC4010`) 在 `0xAC40C7..0xAC40D9` 把 `SrRenderer+0x08` 与 `SrRenderer+0x48` 初始化为 identity。`0xAC7400` 在 `0xAC742D` 保存 target pointer 到 `+0x248`，但只有同时满足：

```text
SrRenderer+0x248 != null
SrProject pointer != null
```

才进入 `0xAC7502` 后的 Camera/viewport 构造。target 为 null 时函数不会读取 Width/Height/Camera，也不会重写 `+0x08/+0x48`。所以 Width-as-Aspect 只属于已解析到命名 target 的分支，不能用于空 `TargetScene` player。

## ImageCast 与 packet 矩阵

`srd_render_image_cast` (`0xAD77D0`) 在 `0xAD783B..0xAD7B1E` 用 CAST world 3x4 直接构造四个 world XYZ，并在 `0xAD7C75..0xAD7C8A` 原样复制到 36-byte format-14 顶点的 position。它没有调用 `srd_project_cast_corners_to_screen`。

`srd_project_cast_corners_to_screen` (`0xAD3B00`) 的唯一实际调用方是 `sub_AD40B0` (`0xAD40B0`) 的可见性/相交测试；它不生成最终 draw vertex。

`srd_begin_quad_draw` (`0xAC5320`) 在 `0xAC54A8..0xAC54B7` 明确分流：

- 2D CAST：向 `ceylon_vertex_builder_set_packet_matrix_from_object` 传 null，packet 两个矩阵为 identity；
- 3D CAST：把 `SrRenderer+0x08` 传给 `ceylon_vertex_builder_set_packet_matrices` (`0x6DF2D0`)。

后者把矩阵复制到 packet `+0xA0`，上一矩阵复制到 `+0xE0`。`ceylon_submit_draw_packet_fixed_shader_constants` 再分别上传为 VS `c0..c3` 与 `c4..c7`。因此在查找结果确为 null 的条件下，首个三维 packet 精确使用 identity `c0..c3`，不是 `inverse(receivingTargetPV) * srdPV`。

最终接收全局 packet 的 `air::Scene` 仍通过自己的 `sea::AllEnvBasic` 提供 VS `c10..c13 = Projection*View`。renderer preparation 的命名 target 与全局队列的接收 target 是两个独立对象角色，不能压成一个宿主 Camera 字段。

## Common 回归

Rust `WorldSnapshot` 现在分别保存：

- 可选的 renderer project target；
- 最终接收 target 的 `Projection*View`；
- 最终 target 的 ShapeEnv2D screen source size。

`CommonBackGroundObject` 与 `AdvertiseLogoObject` 的 property 2 字符串都为空，当前 Chusan profile 的 `renderer_project_target=None` 表示已证明的原游戏 null lookup。语料回归断言 `CHU_UI_Common_BK_00_v11.srd` 三维 Image draw 上传 identity `c0..c3`。null 分支的原二进制可见性矩形没有确定初值，Rust 预览显式跳过该项剔除。

使用 `CommonBackGround/MainScene` 的已证明 null lookup、present `1080x1920`、screen source `1920x1080` 执行 D3D9Ex smoke，两次回读（强制 `ResetEx` 前后）一致：

```text
changed pixels = 2,073,600
white pixels   = 0
bbox           = (0,0)..(1919,1079)
FNV-1a 64      = 990DD2CE283876B6
```

该结果没有使用自动 fit、强制 2D、`Aspect=Width/Height` 或额外 X scale。此前中间窄条来自编辑器错误地用最终接收 MainScene Camera 构造 `SrRenderer+0x08`，不是原游戏尚未发现的 viewport 补偿。
