// Generated from shader_bytecode_generated.rs and the translated WGSL corpus. Do not edit.

const VERTEX_SOURCES: [&str; 14] = [
    include_str!("vs_00.wgsl"),
    include_str!("vs_01.wgsl"),
    include_str!("vs_02.wgsl"),
    include_str!("vs_03.wgsl"),
    include_str!("vs_04.wgsl"),
    include_str!("vs_05.wgsl"),
    include_str!("vs_06.wgsl"),
    include_str!("vs_07.wgsl"),
    include_str!("vs_08.wgsl"),
    include_str!("vs_09.wgsl"),
    include_str!("vs_10.wgsl"),
    include_str!("vs_11.wgsl"),
    include_str!("vs_12.wgsl"),
    include_str!("vs_13.wgsl"),
];

const PIXEL_SOURCES: [&str; 24] = [
    include_str!("ps_00.wgsl"),
    include_str!("ps_01.wgsl"),
    include_str!("ps_02.wgsl"),
    include_str!("ps_03.wgsl"),
    include_str!("ps_04.wgsl"),
    include_str!("ps_05.wgsl"),
    include_str!("ps_06.wgsl"),
    include_str!("ps_07.wgsl"),
    include_str!("ps_08.wgsl"),
    include_str!("ps_09.wgsl"),
    include_str!("ps_10.wgsl"),
    include_str!("ps_11.wgsl"),
    include_str!("ps_12.wgsl"),
    include_str!("ps_13.wgsl"),
    include_str!("ps_14.wgsl"),
    include_str!("ps_15.wgsl"),
    include_str!("ps_16.wgsl"),
    include_str!("ps_17.wgsl"),
    include_str!("ps_18.wgsl"),
    include_str!("ps_19.wgsl"),
    include_str!("ps_20.wgsl"),
    include_str!("ps_21.wgsl"),
    include_str!("ps_22.wgsl"),
    include_str!("ps_23.wgsl"),
];

const VS_INPUTS_00: [WgpuVertexInput; 2] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::Color0 },
];

const VS_INPUTS_01: [WgpuVertexInput; 4] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::TexCoord1 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color0 },
];

const VS_INPUTS_02: [WgpuVertexInput; 4] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_03: [WgpuVertexInput; 4] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_04: [WgpuVertexInput; 5] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::TexCoord1 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 4, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_05: [WgpuVertexInput; 3] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
];

const VS_INPUTS_06: [WgpuVertexInput; 3] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
];

const VS_INPUTS_07: [WgpuVertexInput; 5] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::Tangent },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 4, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_08: [WgpuVertexInput; 5] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::Tangent },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 4, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_09: [WgpuVertexInput; 2] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::Color0 },
];

const VS_INPUTS_10: [WgpuVertexInput; 4] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_11: [WgpuVertexInput; 4] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_12: [WgpuVertexInput; 5] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::TexCoord1 },
    WgpuVertexInput { location: 3, semantic: WgpuVertexSemantic::Color0 },
    WgpuVertexInput { location: 4, semantic: WgpuVertexSemantic::Color1 },
];

const VS_INPUTS_13: [WgpuVertexInput; 3] = [
    WgpuVertexInput { location: 0, semantic: WgpuVertexSemantic::Position },
    WgpuVertexInput { location: 1, semantic: WgpuVertexSemantic::TexCoord0 },
    WgpuVertexInput { location: 2, semantic: WgpuVertexSemantic::Color0 },
];

const VERTEX_INPUTS: [&[WgpuVertexInput]; 14] = [
    &VS_INPUTS_00,
    &VS_INPUTS_01,
    &VS_INPUTS_02,
    &VS_INPUTS_03,
    &VS_INPUTS_04,
    &VS_INPUTS_05,
    &VS_INPUTS_06,
    &VS_INPUTS_07,
    &VS_INPUTS_08,
    &VS_INPUTS_09,
    &VS_INPUTS_10,
    &VS_INPUTS_11,
    &VS_INPUTS_12,
    &VS_INPUTS_13,
];

const WGPU_SHADER_ENTRIES: [WgpuShaderEntry; 82] = [
    WgpuShaderEntry { key: *b"AACAABBAAAGAAAAAAA", vertex_index: 0, pixel_index: 0 },
    WgpuShaderEntry { key: *b"AACBABBAADIADAAAAA", vertex_index: 1, pixel_index: 1 },
    WgpuShaderEntry { key: *b"AAEBABAAAAEHAAAAAA", vertex_index: 2, pixel_index: 2 },
    WgpuShaderEntry { key: *b"AAEBABAAABEFAAAAAA", vertex_index: 2, pixel_index: 3 },
    WgpuShaderEntry { key: *b"AAEBABAAABGFAAAAAA", vertex_index: 2, pixel_index: 4 },
    WgpuShaderEntry { key: *b"AAEBABAAABIHAAAAAA", vertex_index: 2, pixel_index: 5 },
    WgpuShaderEntry { key: *b"AAEBABBAAACBAAAAAA", vertex_index: 3, pixel_index: 6 },
    WgpuShaderEntry { key: *b"AAEBABBAAAGAAAAAAA", vertex_index: 3, pixel_index: 7 },
    WgpuShaderEntry { key: *b"AAEBABBAAAIAAAAAAA", vertex_index: 3, pixel_index: 7 },
    WgpuShaderEntry { key: *b"AAEBABBAABCBAAAAAA", vertex_index: 3, pixel_index: 8 },
    WgpuShaderEntry { key: *b"AAEBABBAABCJFAAAAA", vertex_index: 3, pixel_index: 8 },
    WgpuShaderEntry { key: *b"AAEBABBAABGAAAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAABGAGAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAABGIFAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAABIAAAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAABIAGAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAABIIFAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAABKAAAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAADCBGAAAAA", vertex_index: 4, pixel_index: 10 },
    WgpuShaderEntry { key: *b"AAEBABBAADCJFAAAAA", vertex_index: 4, pixel_index: 11 },
    WgpuShaderEntry { key: *b"AAEBABBAADGAAAAAAA", vertex_index: 4, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAADGAGAAAAA", vertex_index: 4, pixel_index: 12 },
    WgpuShaderEntry { key: *b"AAEBABBAADIAAAAAAA", vertex_index: 4, pixel_index: 9 },
    WgpuShaderEntry { key: *b"AAEBABBAADIAGAAAAA", vertex_index: 4, pixel_index: 12 },
    WgpuShaderEntry { key: *b"AAEBABBAADIIEAAAAA", vertex_index: 4, pixel_index: 13 },
    WgpuShaderEntry { key: *b"AAEBABBAADIIFAAAAA", vertex_index: 4, pixel_index: 14 },
    WgpuShaderEntry { key: *b"AAKAAABAAAGAAAAAAA", vertex_index: 5, pixel_index: 0 },
    WgpuShaderEntry { key: *b"AAKAAABAABGAAAAAAA", vertex_index: 5, pixel_index: 15 },
    WgpuShaderEntry { key: *b"AAKAABBAABEAAAAAAA", vertex_index: 5, pixel_index: 15 },
    WgpuShaderEntry { key: *b"AAKAABBAABGAAAAAAA", vertex_index: 5, pixel_index: 15 },
    WgpuShaderEntry { key: *b"AAKAABBAABIAAAAAAA", vertex_index: 5, pixel_index: 15 },
    WgpuShaderEntry { key: *b"AAKAABBAEBGAAAAAAA", vertex_index: 5, pixel_index: 15 },
    WgpuShaderEntry { key: *b"AAKAABBAEBIAAAAAAA", vertex_index: 5, pixel_index: 15 },
    WgpuShaderEntry { key: *b"AAKAABDAABIAAAAAAA", vertex_index: 6, pixel_index: 16 },
    WgpuShaderEntry { key: *b"AAMAABBAABGAAAAAAA", vertex_index: 3, pixel_index: 9 },
    WgpuShaderEntry { key: *b"ACMAAAEAAAAAAAAAAA", vertex_index: 7, pixel_index: 7 },
    WgpuShaderEntry { key: *b"ACMAAAFAAAAAAAAAAA", vertex_index: 7, pixel_index: 7 },
    WgpuShaderEntry { key: *b"ACMAAAFAABAAAAAAAA", vertex_index: 7, pixel_index: 9 },
    WgpuShaderEntry { key: *b"ACMAAAFAEBAAAAAAAA", vertex_index: 7, pixel_index: 9 },
    WgpuShaderEntry { key: *b"ACMAAAHAEBAAAAAAAA", vertex_index: 8, pixel_index: 17 },
    WgpuShaderEntry { key: *b"EACAAABAAAGAAAAAAA", vertex_index: 9, pixel_index: 0 },
    WgpuShaderEntry { key: *b"EACAABBAAAGAAAAAAA", vertex_index: 9, pixel_index: 0 },
    WgpuShaderEntry { key: *b"EAEBABAAAACHAAAAAA", vertex_index: 10, pixel_index: 18 },
    WgpuShaderEntry { key: *b"EAEBABAAAAEHAAAAAA", vertex_index: 10, pixel_index: 2 },
    WgpuShaderEntry { key: *b"EAEBABAAAAMGAAAAAA", vertex_index: 10, pixel_index: 19 },
    WgpuShaderEntry { key: *b"EAEBABAAABCFAAAAAA", vertex_index: 10, pixel_index: 20 },
    WgpuShaderEntry { key: *b"EAEBABAAABEHAAAAAA", vertex_index: 10, pixel_index: 21 },
    WgpuShaderEntry { key: *b"EAEBABAAABGFAAAAAA", vertex_index: 10, pixel_index: 4 },
    WgpuShaderEntry { key: *b"EAEBABAAABIHAAAAAA", vertex_index: 10, pixel_index: 5 },
    WgpuShaderEntry { key: *b"EAEBABAAABKGAAAAAA", vertex_index: 10, pixel_index: 22 },
    WgpuShaderEntry { key: *b"EAEBABBAAACBAAAAAA", vertex_index: 11, pixel_index: 6 },
    WgpuShaderEntry { key: *b"EAEBABBAAAGAAAAAAA", vertex_index: 11, pixel_index: 7 },
    WgpuShaderEntry { key: *b"EAEBABBAAAIAAAAAAA", vertex_index: 11, pixel_index: 7 },
    WgpuShaderEntry { key: *b"EAEBABBAAAKAAAAAAA", vertex_index: 11, pixel_index: 7 },
    WgpuShaderEntry { key: *b"EAEBABBAABCBAAAAAA", vertex_index: 11, pixel_index: 8 },
    WgpuShaderEntry { key: *b"EAEBABBAABGAAAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAABGAGAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAABIAAAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAABIAGAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAABIIFAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAADCJFAAAAA", vertex_index: 12, pixel_index: 11 },
    WgpuShaderEntry { key: *b"EAEBABBAADGAAAAAAA", vertex_index: 12, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAADGAFAAAAA", vertex_index: 12, pixel_index: 23 },
    WgpuShaderEntry { key: *b"EAEBABBAADGAGAAAAA", vertex_index: 12, pixel_index: 12 },
    WgpuShaderEntry { key: *b"EAEBABBAADGIFAAAAA", vertex_index: 12, pixel_index: 14 },
    WgpuShaderEntry { key: *b"EAEBABBAADIAAAAAAA", vertex_index: 12, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAEBABBAADIAGAAAAA", vertex_index: 12, pixel_index: 12 },
    WgpuShaderEntry { key: *b"EAEBABBAADIIFAAAAA", vertex_index: 12, pixel_index: 14 },
    WgpuShaderEntry { key: *b"EAEBABBAADKAGAAAAA", vertex_index: 12, pixel_index: 12 },
    WgpuShaderEntry { key: *b"EAKAAAAAAAAAAAAAAA", vertex_index: 13, pixel_index: 0 },
    WgpuShaderEntry { key: *b"EAKAAABAABEAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAAABAABGAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABAAABAAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABBAABEAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABBAABGAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABBAABIAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABBAABKAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABBAEBGAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAKAABBAEBIAAAAAAA", vertex_index: 13, pixel_index: 15 },
    WgpuShaderEntry { key: *b"EAMAABBAABGAAAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAMAABBAABGAGAAAAA", vertex_index: 11, pixel_index: 9 },
    WgpuShaderEntry { key: *b"EAMAABBAABGIFAAAAA", vertex_index: 11, pixel_index: 9 },
];
