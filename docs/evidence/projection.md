# 投影、相机、视口与最终屏幕坐标证据

本页只记录由游戏二进制和本地 SRD 样本闭环得到的结论。

- `chusanApp.exe` SHA-256：`28EBB4580A4CAE8ED0605B37F2F7C16460497412FE352E020A43D3A082FFEB67`
- 本轮保存后的 IDB SHA-256：`189A48449F4761584712182DB6755698526D50BDDD0A00C113DF376E5C01096F`

## SRD 文件中的 CAM 记录

`srd_resource_load_srff_and_external_textures` (`0xAA55C0`) 构造 RTTI 类型 `SrProject`，随后由 `srd_parse_srff` (`0xA9F220`) 和 `srd_parse_srck` (`0xA9F370`) 解析 `SRFF -> SRCK -> PROJ`。`srd_parse_srck` 为 PROJ 分配并清零精确的 `0x88` 字节记录；直接子块 `CAM ` 被 `srd_parse_cam` (`0xAA0040`) 写入 `SrProject+0x58`：

| CAM 属性 | SrProject CAM 偏移 | 语义 |
| --- | ---: | --- |
| `0x12` | `+0x00` | position `[f32; 3]` |
| `0x13` | `+0x0C` | target `[f32; 3]` |
| `0x14` | `+0x18` | signed angle units |
| `0x15` | `+0x1C` | near |
| `0x16` | `+0x20` | far |

多个 CAM 子块会按文件顺序写入同一块零初始化记录；缺失属性保留此前值，首次缺失则保持零。这一行为已由 Rust `CameraDefinition::from_project_block` 复现。

`srd_player_impl_load_project` (`0xAAB990`) 把项目资源保存到 `SrPlayer::Impl+0x08`；资源 `+0x04` 指向上述 `SrProject` 记录。`srd_player_impl_prepare_renderer` (`0xAACA80`) 取出它，`srd_renderer_configure_project_camera` (`0xAC7400`) 明确使用 `SrProject+0x58`，因此运行时相机输入直接来自当前 SRD 的 CAM，而不是旧编辑器默认值。

样本 `surfboard/system/CHU_UI_System_00_v10.srd` 的实值为：

```text
position    = [0, 0, 1000]
target      = [0, 0, 0]
angle_units = 8191
near        = 10
far         = 100000
```

同一样本的 SCN `0x40/0x41` 为 `1920/1080`。`srd_parse_scn` (`0xAA2FC0`) 分别把它们读成 scene 记录 `+0x60/+0x64` 的 f32；Rust `Scene.width/height` 已按相同行为解析。

## sea::Camera 对象与属性

全局管理器 `dword_1CA0B98` 的 `+0x20` 是构造函数 `sea_camera_construct` (`0x654550`) 创建的 `Camera` 对象。构造器注册的相关属性为：

```text
0 Near          default 1
1 Far           default 30000
2 FovY          default 45
3 Aspect        default 1
4 Width         default 1
5 Height        default 1
6 IsPerspective default true
8 Position      default [0,0,30]
9 Target        default [0,0,0]
10 Up           default [0,1,0]
11 OffsetX      default 0
12 OffsetY      default 0
```

`0xAC7400` 先把 property 2 解析得到的 target pointer 保存到 `SrRenderer+0x248`。只有该 pointer 非空且 `SrProject` 非空时，才执行以下 Camera 配置：

1. CAM position -> `sea_camera_set_position`；
2. CAM target -> `sea_camera_set_target`；
3. 固定 up `[0,1,0]` -> `sea_camera_set_up`；
4. CAM angle、当前渲染目标 Width、CAM near/far -> `sea_camera_set_perspective_parameters`；
5. `sea_camera_commit_dirty_properties` 重建矩阵。

角度的两次换算常量均已按原始 f32 位验证：

```text
degrees = float(angle_units) * f32::from_bits(0x3BB40000)
radians = degrees * f32::from_bits(0x3C8EFA35)
```

样本 `8191` 得到 `44.994507°`。注意：虚表 `+0x88` 已进一步闭环到 `sea_camera_set_perspective_parameters` (`0x656450`)；它把第二个参数写入名为 `Aspect` 的 property 3，但 `0xAC7400` 在非空命名 target 分支传入的是该 target Width。函数前面虽然另行取得外部 target/camera aspect 或 override 值并保存到局部变量，该值没有进入这次全局 Camera 调用。Rust API 保留显式的 `projection_width` 参数，没有擅自改成宽高比。空 `TargetScene` 不执行本段，详见 [`render-empty-target-matrices.md`](render-empty-target-matrices.md)。

## View、Projection 与 Projection*View

`sea_camera_rebuild_dirty_matrices` (`0x656770`) 证明 Camera 内的三个矩阵：

```text
Camera + 0x0C8 = View
Camera + 0x108 = Projection
Camera + 0x148 = Projection * View
```

对应 getter 为：

```text
virtual +0xE8 -> View
virtual +0xEC -> Projection
virtual +0xF0 -> Projection * View
virtual +0xF4 -> 把 View 复制到调用方缓冲区
```

View 由 `srd_build_look_at_rh` (`0x6B36E0`) 构造。它使用 `normalize(position-target)` 作为 forward，归一化 `forward × up` 作为 right，再计算 `forward × right`；平移列为三个基向量与 position 的负点积。若 position 与 target 的距离小于 `f32::from_bits(0x34000000)`，Camera 更新器先把 position.z 加上 `f32::from_bits(0x38D1B717)`。

Perspective 由 `sea_camera_build_projection_matrix` (`0x655630`) 和 `srd_build_perspective_fov_rh` (`0x6B3CD0`) 构造：

```text
q   = 1 / game_tan(fov_y_radians * 0.5)
M00 = q / Aspect
M11 = q
M22 = far / (near - far)
M23 = near * far / (near - far)
M32 = -1
```

其余项先清零，随后 OffsetX/OffsetY 覆盖 `M02/M12`。若 `far <= near + 0.0001f`，游戏先把 far 替换为 `near + 0.0001f`。`game_tan` 不是 CRT `tanf`：`0x6B4940/0x6B3DE0` 使用 `angle / pi` 的分段约化和 11 项 f64 Horner 多项式；Rust 实现保留了常量位模式与运算顺序。

同一 builder 的 orthographic 分支由 `srd_build_orthographic_off_center_rh` (`0x6B3B50`) 构造 centered `[-width/2,+width/2] × [-height/2,+height/2]` 矩阵。当前 SRD renderer 配置调用 `sea_camera_set_perspective_parameters` 并强制 `IsPerspective=true`，因此选定样本走 perspective 分支。

## Shader 的 mtxPrjView provider

`sea_all_env_basic_update_values` (`0x6E1400`) 对传入 Camera context 的调用顺序是：

```text
virtual +0xEC -> Projection
virtual +0xE8 -> View
virtual +0xF4 -> View copy
mtxPrjView = Projection * View
```

结果写入 `sea::AllEnvBasic+0x40` 对应的 shader resource，参数注册函数把它以名字 `mtxPrjView` 放到逻辑 slot 2。选定 Simple VS 的 `c10..c13` 正是这个矩阵，因而不再是未证明的占位常量。

该 Camera 属于实际渲染 target，而不是 SRD 文件本身。空 `TargetScene` 的 packet 会进入全局队列，再由当帧所有注册 target 分别过滤；每个基础 Scene 的虚表 `+0x58` 返回自己的 `target+0x90` Camera。完整路由证据见 [`render-target-routing.md`](render-target-routing.md)。因此同一份 SRD 在不同宿主 target 中可以得到不同的 `mtxPrjView`，不能把 SRD CAM 的 `Projection*View` 无条件直接写入 2D draw 的 `c10..c13`。

## SrPlayer 的 FirstCalcMatrix 输入边界

顶层 runtime layer 不直接以 identity 作为 CAST 根。`srd_update_runtime_layer` (`0xABE710`) 在没有 owning RefCast 时执行：

```text
layer_world(+0x16C) = SrRenderer(+0xB8) * layer_local(+0x13C)
```

`srd_reset_runtime_layer_state` (`0xAC15A0`) 把 layer local 初始化为单位变换。`SrRenderer+0xB8` 则由 `srd_renderer_configure_project_camera` 从 `SrPlayer` 虚表 `+0x40` 返回的 3x4 matrix 复制；该虚函数最终是 `0x60A230`，返回玩家 scene-node 的组合矩阵。SrPlayer 构造器把属性 3 注册为 `FirstCalcMatrix`，但这个 bool 只选择返回本地或组合矩阵，矩阵数值本身来自 SrPlayer 所在的外部 scene graph，而不在 SRD 文件中。

因此编辑器必须把 preview placement 明确作为宿主输入：SRD 负责 CAM、layer local 与 CAST local；宿主负责等价于 `FirstCalcMatrix` 的根 3x4。当前代码不会把 identity 冒充成游戏在任意调用现场的最终父矩阵。Composition 的 fit-to-view 可以作为编辑器显示变换实现，但必须与游戏证据矩阵分层保存。

## SrRenderer 屏幕矩阵

`srd_construct_player` (`0xAA68B0`) 创建 `surfride::SrPlayer::Impl`，Impl `+0x10` 的 RTTI 类型是 `surfride::SrRenderer`。`srd_construct_renderer` (`0xAC4010`) 把 `SrRenderer+0x08` 和 `+0x48` 初始化为单位矩阵。

当 property 2 解析到非空命名 target 时，`srd_renderer_configure_project_camera` 分别取得该 target camera 与 SRD 全局 camera 的 Projection/View：

```text
external_projection_view = external_projection * external_view
external_inverse         = inverse(external_projection_view)
srd_projection_view      = srd_projection * srd_view
SrRenderer+0x08          = external_inverse * srd_projection_view
```

`ceylon_inverse_matrix4x4` (`0x6B4170`) 是带 pivot 的 4×4 Gauss-Jordan 求逆实现；该调用不是转置或复制。函数末尾再构造：

```text
viewport = viewport_matrix(width, height)
temporary = viewport * external_projection_view
screen_matrix = temporary * *(Matrix4x4 *)(SrRenderer + 0x08)
*(Matrix4x4 *)(SrRenderer + 0x48) = screen_matrix
```

因此在该分支可逆且忽略浮点误差时，最终组合代数上约为 `viewport * srd_projection_view`。若 `TargetScene` 解析为 null，函数在构造这些矩阵前返回，`SrRenderer+0x08/+0x48` 精确保持构造 identity；最终接收全局 packet 的 target Camera 不会回填这两个字段。

Width/Height getter 返回 signed i32，游戏用 `cvtdq2ps` 转成 f32 后从单位矩阵构造：

```text
[ width*0.5, 0,           0, width*0.5  ]
[ 0,         height*-0.5, 0, height*0.5 ]
[ 0,         0,           1, 0          ]
[ 0,         0,           0, 1          ]
```

它把 X 的 `[-1,+1]` 映射到 `[0,width]`，把 Y 的 `+1..-1` 映射到 `[0,height]`，并执行 D3D 屏幕坐标的 Y 反转。

## 4x4 乘法与 CAST 屏幕坐标

`srd_mul_matrix4x4` (`0x604010`) 对每个输出项严格按以下分组计算 row-major `lhs * rhs`：

```text
(lhs.w * rhs.row3 + lhs.z * rhs.row2)
  +
(lhs.y * rhs.row1 + lhs.x * rhs.row0)
```

`srd_project_cast_corners_to_screen` (`0xAD3B00`) 对二维 CAST 直接复制 world X/Y。三维 CAST 使用 `SrRenderer+0x48` 的第 0、1、3 行计算 homogeneous X/Y/W，然后乘 `1/W`。该函数的唯一实际调用方 `sub_AD40B0` 用结果做可见性/相交测试；它不生成最终 ImageCast draw vertex。命名 target 成功解析时 `+0x48` 已包含 viewport；查找结果为 null 时它保持 identity，而对应可见性矩形已证明未被初始化。

最终 ImageCast 顶点由 `srd_render_image_cast` (`0xAD77D0`) 直接写入 world XYZ。`srd_begin_quad_draw` (`0xAC5320`) 对三维 CAST 把 `SrRenderer+0x08` 复制到 packet world matrix；接收 target 的 `Projection*View` 则从 `sea::AllEnvBasic` 单独进入 `c10..c13`。详见 [`render-empty-target-matrices.md`](render-empty-target-matrices.md)。

## Rust 对应实现

[`src/camera.rs`](../../src/camera.rs) 当前实现 CAM 解析、游戏角度换算、自有 tan 多项式、RH look-at、perspective/orthographic 和 `Projection*View`；[`src/projection.rs`](../../src/projection.rs) 实现游戏 4x4 乘法、viewport、屏幕矩阵组合及 CAST X/Y/W 投影。完整 91 个 SRD 与 360 个 DDS 语料回归通过。

## 尚未闭环

- 游戏中各 host target 在具体时刻采用的 Camera 仍不能由独立 SRD 推出；编辑器交互策略已经固定为始终使用项目内嵌 `PROJ -> CAM `，缺失时保持 position、target、angle、near、far 全零，不把某个已知 host 构造 Camera 伪装为文件属性。
- 各个实际 Chusan 画面在具体时刻注册了哪些 target，以及其 camera/scene-node 数值。
- 3D CAST 的 world XYZ、packet world matrix 与 target `Projection*View` 提交链已闭环；命名 target 下 ImageCast、SliceCast、NumberCast 与 2D/3D Fennel 四角投影/AABB 剔除已实现。3D TextCast 的独立 TextBox matrix 分流见 [`textcast-3d.md`](textcast-3d.md)。Advertise/Common 的空字符串 lookup 已闭环为 null；原二进制在该分支读取未初始化可见性矩形。仍待闭合的是特殊 depth/stencil 分支及未覆盖的其他 target 配置。
- Camera OffsetX/OffsetY 是否存在 SRD renderer 之外的运行时写入者。
