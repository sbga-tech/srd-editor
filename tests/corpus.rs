use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use srd_editor::animation::{Evaluation, KeyData, Motion, ScalarValue, Track};
use srd_editor::attribute::CastAttributeValue;
use srd_editor::csli::CsliDefinition;
use srd_editor::dds::{
    D3d9Direct2dUpload, D3d9TextureCreation, DdsDescriptor, DdsLoadPolicy, GameTextureFormat,
};
use srd_editor::document::EditorDocument;
use srd_editor::fennel::{
    FENNEL_TEXTBOX_CLIP_FLAG, FennelDefaultLayoutError, FennelFittingLayoutError,
    FennelFontSlotRegistry, FennelLayoutGlyphMetrics, FennelMode56LayoutError,
    FennelPlainRecordError, FennelStaticTextProperties, FennelStaticUnclippedDrawInput,
    FennelTextureBatchStop, build_fennel_plain_record_stream,
    build_fennel_static_unclipped_vertex_batches, build_fennel_texture_batch_membership,
    decode_fennel_game_text, fennel_fresh_srd_textbox_flags, fennel_srd_font_style,
    fennel_srd_textbox_flags, layout_fennel_static_default, layout_fennel_static_fitting_lines,
    layout_fennel_static_flag20, layout_fennel_static_mode1, layout_fennel_static_mode56,
    tokenize_fennel_plain_text,
};
use srd_editor::game_host::CHUSAN_LINKED_VERSE_GATE_PLAYER;
use srd_editor::image::{ImageDefinition, ImageReferenceChannel};
use srd_editor::number::NumberDefinition;
use srd_editor::projection::identity_matrix4x4_game;
use srd_editor::reference_runtime::{ProjectRuntime, ReferenceLayerRuntimeState};
use srd_editor::renderer::{
    FennelTextFontRole, assign_fennel_font_resource_requests,
    collect_fennel_font_resource_requests, select_srd_image_render_preset,
};
use srd_editor::ruhuna::RuhunaFont;
use srd_editor::scene::{Layer, Project, ReferenceTarget};
use srd_editor::surf_file_table::parse_surf_file_table;
use srd_editor::texture::TextureList;
use srd_editor::transform::Affine3x4;
use srd_editor::vtbf::{Block, OwnedBlock, OwnedProperty, SrdFile};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CorpusProfile {
    Legacy53,
    Complete91,
}

fn srd_corpus_profile(file_count: usize) -> CorpusProfile {
    match file_count {
        53 => CorpusProfile::Legacy53,
        91 => CorpusProfile::Complete91,
        _ => panic!("unexpected local SRD corpus size: {file_count}"),
    }
}

fn dds_corpus_profile(file_count: usize) -> CorpusProfile {
    match file_count {
        97 => CorpusProfile::Legacy53,
        360 => CorpusProfile::Complete91,
        _ => panic!("unexpected local DDS corpus size: {file_count}"),
    }
}

macro_rules! corpus_root {
    () => {{
        let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
            eprintln!("skipping: GAME_DATA_CORPUS is not set");
            return;
        };
        root.join("surfboard")
    }};
}

fn collect_srd_files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_srd_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "srd") {
            output.push(path);
        }
    }
}

#[test]
fn shipped_surf_file_table_maps_resource_84_to_linked_verse_gate() {
    let Some(data_root) = corpus_root!().parent().map(Path::to_path_buf) else {
        return;
    };
    let path = data_root.join("db/SurfFileTableRecord.bin");
    if !path.exists() {
        eprintln!(
            "skipping: SurfFileTable sample not found at {}",
            path.display()
        );
        return;
    }
    let records = parse_surf_file_table(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(records.len(), 91);
    let record = records
        .get(CHUSAN_LINKED_VERSE_GATE_PLAYER.surf_file_id as usize)
        .expect("resource id 84 must exist");
    assert_eq!(record.id, CHUSAN_LINKED_VERSE_GATE_PLAYER.surf_file_id);
    assert_eq!(
        record.name,
        CHUSAN_LINKED_VERSE_GATE_PLAYER.surf_file_name.as_bytes()
    );
    assert_eq!(
        record.path,
        CHUSAN_LINKED_VERSE_GATE_PLAYER.surf_file_path.as_bytes()
    );
}

fn collect_dds_files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_dds_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "dds") {
            output.push(path);
        }
    }
}

fn collect_afb_files(path: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_afb_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "afb") {
            output.push(path);
        }
    }
}

fn count_afb_factory_type_keys(bytes: &[u8]) -> [usize; 3] {
    const PATTERNS: [[u8; 4]; 3] = [
        0x005B_916Au32.to_le_bytes(),
        0x005B_916Au32.to_be_bytes(),
        0x005B_9172u32.to_le_bytes(),
    ];
    let mut counts = [0; 3];
    for window in bytes.windows(4) {
        let index = match window[0] {
            0x6A => 0,
            0x00 => 1,
            0x72 => 2,
            _ => continue,
        };
        if window == PATTERNS[index] {
            counts[index] += 1;
        }
    }
    counts
}

#[test]
fn shipped_afb_corpus_has_no_sgl_scene_factory_type_key() {
    let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
        eprintln!("skipping: GAME_DATA_CORPUS is not set");
        return;
    };
    let mut paths = Vec::new();
    collect_afb_files(&root, &mut paths);
    paths.sort();
    assert_eq!(paths.len(), 293, "unexpected complete-game AFB count");

    let mut scene_little_endian_hits = 0;
    let mut scene_big_endian_hits = 0;
    let mut positive_control_hits = 0;
    let mut positive_control_files = BTreeSet::new();
    for path in &paths {
        let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let [little_endian_hits, big_endian_hits, hits] = count_afb_factory_type_keys(&bytes);
        scene_little_endian_hits += little_endian_hits;
        scene_big_endian_hits += big_endian_hits;
        positive_control_hits += hits;
        if hits != 0 {
            positive_control_files.insert(path.clone());
        }
    }

    assert_eq!(scene_little_endian_hits, 0);
    assert_eq!(scene_big_endian_hits, 0);
    assert_eq!(positive_control_hits, 35);
    assert_eq!(positive_control_files.len(), 15);
}

#[test]
fn parses_complete_game_ruhuna_font_archives_and_embedded_dds_pages() {
    let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
        eprintln!("skipping: GAME_DATA_CORPUS is not set");
        return;
    };
    let expected = [
        ("14pt", 10, 7_161, 1, 1_024),
        ("18pt", 14, 7_161, 1, 2_048),
        ("24pt", 18, 7_161, 2, 512),
        ("32pt", 24, 7_161, 2, 2_048),
        ("60pt", 45, 7_161, 7, 256),
        ("240pt", 180, 201, 2, 1_024),
    ];
    for (suffix, point, glyph_count, page_count, last_height) in expected {
        let path = root
            .join("A000/font")
            .join(format!("RFO_SEGAKAKUGOTHIC_DB_{suffix}.rfz"));
        let bytes = fs::read(&path).unwrap();
        let font = RuhunaFont::from_rfz(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(font.database.point, point, "{}", path.display());
        assert_eq!(font.glyphs.len(), glyph_count, "{}", path.display());
        assert_eq!(
            font.database.glyph_count as usize,
            glyph_count,
            "{}",
            path.display()
        );
        assert_eq!(font.textures.len(), 1, "{}", path.display());
        assert_eq!(font.database.texture_width, 2_048, "{}", path.display());
        assert_eq!(font.database.texture_height, 2_048, "{}", path.display());
        assert_eq!(
            font.database.texture_last_height,
            last_height,
            "{}",
            path.display()
        );
        let pages = font
            .atlas_pages(&font.textures[0])
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let sampler = font
            .atlas_sampler_state(&font.textures[0])
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(sampler.base.address_u as u32, 3, "{}", path.display());
        assert_eq!(sampler.base.address_v as u32, 3, "{}", path.display());
        assert_eq!(sampler.base.min_filter as u32, 2, "{}", path.display());
        assert_eq!(sampler.base.mag_filter as u32, 2, "{}", path.display());
        assert_eq!(sampler.mip_filter as u32, 1, "{}", path.display());
        assert_eq!(sampler.max_mip_level, 1, "{}", path.display());
        assert_eq!(sampler.max_anisotropy, 1, "{}", path.display());
        assert_eq!(sampler.mip_lod_bias_bits, 0, "{}", path.display());
        assert_eq!(sampler.border_color, 0, "{}", path.display());
        assert_eq!(pages.len(), page_count, "{}", path.display());
        for page in pages {
            assert_eq!(
                page.descriptor.format,
                GameTextureFormat::A4_R4_G4_B4,
                "{} page {}",
                path.display(),
                page.page_index
            );
            assert!(
                font.atlas_page_bytes(&page).starts_with(b"DDS "),
                "{} page {}",
                path.display(),
                page.page_index
            );
        }

        // Nonzero values are test-only opaque tokens. The conversion itself
        // must preserve the game's page lookup boundary without storing host
        // pointers in the 128-byte x86 record image.
        let runtime = font
            .build_runtime_font(0x1234_5678, |page| u32::from(page) + 1)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(runtime.glyphs.len(), glyph_count, "{}", path.display());
        assert_eq!(runtime.glyph_pages.len(), glyph_count, "{}", path.display());
        assert_eq!(
            runtime.dense_glyph_indices.len(),
            usize::from(runtime.maximum_code - runtime.minimum_code) + 1,
            "{}",
            path.display()
        );
        for (glyph_index, record) in runtime.glyphs.iter().enumerate() {
            assert_eq!(record.owner_token, 0x1234_5678, "{}", path.display());
            assert_eq!(
                runtime.dense_glyph_index(record.code),
                Some(glyph_index as u16),
                "{} code {:#06X}",
                path.display(),
                record.code
            );
            let Some(page) = runtime.glyph_pages[glyph_index] else {
                assert_eq!(record.texture_token, 0, "{}", path.display());
                continue;
            };
            assert_eq!(
                record.texture_token,
                u32::from(page) + 1,
                "{}",
                path.display()
            );
            let expected_height = if u32::from(page) == font.database.texture_page_count - 1 {
                font.database.texture_last_height
            } else {
                font.database.texture_height
            };
            assert_eq!(
                record.inverse_texture_height,
                1.0 / f32::from(expected_height),
                "{} code {:#06X}",
                path.display(),
                record.code
            );
        }
    }
}

#[test]
fn parses_complete_game_dds_corpus_with_binary_resource_rules() {
    let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
        eprintln!("skipping: GAME_DATA_CORPUS is not set");
        return;
    };
    let mut files = Vec::new();
    collect_dds_files(&root, &mut files);
    files.sort();

    let mut format_counts = std::collections::BTreeMap::new();
    let mut mip_counts = std::collections::BTreeMap::new();
    let mut cube_counts = std::collections::BTreeMap::new();
    let mut plan_counts = std::collections::BTreeMap::new();
    for path in &files {
        let bytes = fs::read(path).unwrap();
        let descriptor = DdsDescriptor::parse(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        descriptor
            .validate_data_len(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(descriptor.required_data_len().unwrap(), bytes.len());
        *format_counts.entry(descriptor.format.0).or_insert(0usize) += 1;
        *mip_counts.entry(descriptor.mip_count).or_insert(0usize) += 1;
        *cube_counts.entry(descriptor.is_cube).or_insert(0usize) += 1;
        let plan = match descriptor.creation_plan(DdsLoadPolicy::default()) {
            D3d9TextureCreation::Direct2d { .. } => "direct_2d",
            D3d9TextureCreation::DirectCube { .. } => "direct_cube",
            D3d9TextureCreation::D3dx2d { .. } => "game_d3dx_2d",
            D3d9TextureCreation::D3dxCube { .. } => "game_d3dx_cube",
        };
        *plan_counts.entry(plan).or_insert(0usize) += 1;
    }

    eprintln!(
        "complete game DDS files={}, formats={format_counts:?}, mips={mip_counts:?}, cubes={cube_counts:?}, plans={plan_counts:?}",
        files.len()
    );
    assert_eq!(files.len(), 14_694);
    assert_eq!(
        format_counts,
        [
            (GameTextureFormat::A8_R8_G8_B8.0, 17),
            (GameTextureFormat::DXT1.0, 1_830),
            (GameTextureFormat::DXT5.0, 12_847),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(mip_counts, [(1, 14_690), (8, 4)].into_iter().collect());
    assert_eq!(cube_counts, [(false, 14_694)].into_iter().collect());
    assert_eq!(
        plan_counts,
        [("direct_2d", 7_490), ("game_d3dx_2d", 7_204)]
            .into_iter()
            .collect()
    );
}

#[test]
fn validates_editor_upload_and_decode_paths_for_complete_game_dds() {
    let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
        eprintln!("skipping: GAME_DATA_CORPUS is not set");
        return;
    };
    let mut files = Vec::new();
    collect_dds_files(&root, &mut files);
    files.sort();
    let mut direct_file_count = 0usize;
    let mut update_count = 0usize;
    let mut skipped_count = 0usize;
    let mut decoded_file_count = 0usize;
    let mut decoded_level_count = 0usize;
    for path in &files {
        let bytes = fs::read(path).unwrap();
        let descriptor = DdsDescriptor::parse(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        match descriptor.creation_plan(DdsLoadPolicy::default()) {
            D3d9TextureCreation::Direct2d { .. } => {
                direct_file_count += 1;
                let uploads = descriptor
                    .direct_2d_upload_plan()
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                for upload in uploads {
                    match upload {
                        D3d9Direct2dUpload::UpdateSurface(_) => update_count += 1,
                        D3d9Direct2dUpload::CompressedLevelBelowFourSkipped { .. } => {
                            skipped_count += 1
                        }
                    }
                }
            }
            D3d9TextureCreation::D3dx2d { .. } => {
                decoded_file_count += 1;
                let levels = descriptor
                    .decode_rgba8_levels_with_library(&bytes)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                assert_eq!(levels.len(), descriptor.mip_count as usize);
                assert_eq!(levels[0].width, descriptor.width);
                assert_eq!(levels[0].height, descriptor.height);
                decoded_level_count += levels.len();
            }
            plan => panic!("{}: unexpected plan {plan:?}", path.display()),
        }
    }
    eprintln!(
        "editor DDS paths files={} direct={direct_file_count} updates={update_count} skipped={skipped_count} decoded={decoded_file_count} decoded_levels={decoded_level_count}",
        files.len()
    );
    assert_eq!(files.len(), 14_694);
    assert_eq!(direct_file_count, 7_490);
    assert_eq!(decoded_file_count, 7_204);
}

#[test]
fn parses_local_dds_corpus_with_the_binary_resource_rules() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_dds_files(&root, &mut files);
    files.sort();
    let profile = dds_corpus_profile(files.len());

    let mut format_counts = std::collections::BTreeMap::new();
    let mut mip_counts = std::collections::BTreeMap::new();
    let mut direct_count = 0usize;
    let mut d3dx_count = 0usize;
    for path in files {
        let bytes = fs::read(&path).unwrap();
        let descriptor = DdsDescriptor::parse(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        descriptor
            .validate_data_len(&bytes)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(descriptor.required_data_len().unwrap(), bytes.len());
        assert!(!descriptor.is_cube);
        *format_counts.entry(descriptor.format.0).or_insert(0usize) += 1;
        *mip_counts.entry(descriptor.mip_count).or_insert(0usize) += 1;
        match descriptor.creation_plan(DdsLoadPolicy::default()) {
            D3d9TextureCreation::Direct2d { .. } => {
                direct_count += 1;
                let uploads = descriptor
                    .direct_2d_upload_plan()
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                assert!(!uploads.is_empty(), "{}", path.display());
                for upload in uploads {
                    match upload {
                        D3d9Direct2dUpload::UpdateSurface(upload) => {
                            let end = usize::try_from(upload.source_offset).unwrap()
                                + usize::try_from(upload.source_byte_len).unwrap();
                            assert!(end <= bytes.len(), "{}", path.display());
                            assert_eq!(upload.staging_pool, 2);
                            assert_eq!(upload.staging_lock_flags, 0);
                        }
                        D3d9Direct2dUpload::CompressedLevelBelowFourSkipped {
                            width,
                            height,
                            ..
                        } => assert!(width < 4 || height < 4),
                    }
                }
            }
            D3d9TextureCreation::D3dx2d { .. } => d3dx_count += 1,
            plan => panic!("{} unexpectedly produced {plan:?}", path.display()),
        }
    }

    eprintln!(
        "DDS profile={profile:?}, formats={format_counts:?}, mips={mip_counts:?}, direct={direct_count}, fallback={d3dx_count}"
    );
    let expected_formats = match profile {
        CorpusProfile::Legacy53 => [
            (GameTextureFormat::A8_R8_G8_B8.0, 70),
            (GameTextureFormat::DXT5.0, 27),
        ]
        .into_iter()
        .collect(),
        CorpusProfile::Complete91 => [
            (GameTextureFormat::A8_R8_G8_B8.0, 2),
            (GameTextureFormat::DXT1.0, 7),
            (GameTextureFormat::DXT5.0, 351),
        ]
        .into_iter()
        .collect(),
    };
    assert_eq!(format_counts, expected_formats);
    match profile {
        CorpusProfile::Legacy53 => {
            assert_eq!(mip_counts, [(1, 96), (10, 1)].into_iter().collect());
            assert_eq!(direct_count, 93);
            assert_eq!(d3dx_count, 4);
        }
        CorpusProfile::Complete91 => {
            assert_eq!(mip_counts, [(1, 360)].into_iter().collect());
            assert_eq!(direct_count, 234);
            assert_eq!(d3dx_count, 126);
        }
    }
}

#[test]
fn image_cast_flags_select_only_binary_proven_render_presets_in_the_local_corpus() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut cast_type_counts = [0usize; 3];
    let mut normal_preset_counts = std::collections::BTreeMap::new();
    let mut special_preset_counts = std::collections::BTreeMap::new();
    let mut invalid_low_nibbles = std::collections::BTreeMap::new();
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        for block in file.blocks_depth_first() {
            let (kind, flags, runtime) = if block.is_tag(b"CIMG") {
                let definition = ImageDefinition::from_block(&file, block).unwrap();
                (0, definition.flags, definition.initial_runtime_state())
            } else if block.is_tag(b"CSLI") {
                let definition = CsliDefinition::from_block(&file, block).unwrap();
                let image = ImageDefinition::from_csli_runtime_base(&definition);
                (1, image.flags, image.initial_runtime_state())
            } else if block.is_tag(b"CNUM") {
                let definition = NumberDefinition::from_block(&file, block).unwrap();
                let image = definition.image_base();
                (2, image.flags, image.initial_runtime_state())
            } else {
                continue;
            };
            cast_type_counts[kind] += 1;
            assert_eq!(runtime.field_10, 0);
            assert_eq!(runtime.field_14, 0);
            assert_eq!(runtime.field_18, 0);
            assert_eq!(runtime.render_preset_override, -1);
            assert_eq!(runtime.field_1c, -1);

            match select_srd_image_render_preset(flags, runtime.render_preset_override, false) {
                Some(preset) => *normal_preset_counts.entry(preset).or_insert(0usize) += 1,
                None => *invalid_low_nibbles.entry(flags & 0x0f).or_insert(0usize) += 1,
            }
            if let Some(preset) =
                select_srd_image_render_preset(flags, runtime.render_preset_override, true)
            {
                *special_preset_counts.entry(preset).or_insert(0usize) += 1;
            }
        }
    }

    eprintln!(
        "profile={profile:?}, CIMG/CSLI/CNUM={cast_type_counts:?}, normal presets={normal_preset_counts:?}, special presets={special_preset_counts:?}, unchanged low nibbles={invalid_low_nibbles:?}"
    );
    assert_eq!(
        cast_type_counts,
        match profile {
            CorpusProfile::Legacy53 => [13_773, 799, 552],
            CorpusProfile::Complete91 => [17_867, 983, 634],
        }
    );
    assert_eq!(
        normal_preset_counts,
        match profile {
            CorpusProfile::Legacy53 => [(3, 10_659), (4, 4_242), (5, 43), (9, 180)],
            CorpusProfile::Complete91 => [(3, 13_779), (4, 5_436), (5, 48), (9, 221)],
        }
        .into_iter()
        .collect()
    );
    assert_eq!(special_preset_counts, normal_preset_counts);
    assert!(invalid_low_nibbles.is_empty());
}

#[test]
fn cast_channel_23_only_targets_reference_casts() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut count = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        for block in file
            .blocks_depth_first()
            .filter(|block| block.is_tag(b"LAYR"))
        {
            let layer = Layer::from_block(&file, block).unwrap();
            for animation in block.children.iter().filter(|child| child.is_tag(b"ANIM")) {
                for motion_block in animation
                    .children
                    .iter()
                    .filter(|child| child.is_tag(b"MOT "))
                {
                    let motion = Motion::from_block(&file, motion_block).unwrap();
                    let Ok(node_index) = usize::try_from(motion.target) else {
                        continue;
                    };
                    let Some(node) = layer.nodes.get(node_index) else {
                        continue;
                    };
                    for track in motion.tracks.iter().filter(|track| track.target == 23) {
                        assert_eq!(node.cast_type(), Some(3));
                        assert_eq!(track.format, 0x13);
                        assert!(matches!(track.keys, KeyData::Key20F32(_)));
                        count += 1;
                    }
                }
            }
        }
    }
    assert_eq!(
        count,
        match profile {
            CorpusProfile::Legacy53 => 111,
            CorpusProfile::Complete91 => 118,
        }
    );
}

#[test]
fn parses_local_corpus_with_binary_proven_boundaries() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    srd_corpus_profile(files.len());

    for path in files {
        let bytes = fs::read(&path).unwrap();
        SrdFile::parse(bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }
}

#[test]
fn audits_unsupported_animation_and_catr_layouts_in_the_real_corpus() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut track_count = 0usize;
    let mut track_formats = BTreeMap::new();
    let mut key_layouts = BTreeMap::new();
    let mut unsupported_tracks = BTreeMap::new();
    let mut unsupported_track_examples = Vec::new();
    let mut attribute_type_codes = BTreeMap::new();
    let mut unsupported_attributes = BTreeMap::new();
    let mut unsupported_attribute_examples = Vec::new();

    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        for block in file.blocks_depth_first() {
            if block.is_tag(b"TRK ") {
                let track = Track::from_block(&file, block).unwrap_or_else(|error| {
                    panic!("{} TRK at {:#x}: {error}", path.display(), block.offset)
                });
                track_count += 1;
                *track_formats.entry(track.format).or_insert(0usize) += 1;
                let layout = match &track.keys {
                    KeyData::Key8F32(_) => "Key8F32",
                    KeyData::Key8I32(_) => "Key8I32",
                    KeyData::Key8Bytes4(_) => "Key8Bytes4",
                    KeyData::Key20F32(_) => "Key20F32",
                    KeyData::Key20I32(_) => "Key20I32",
                    KeyData::Unsupported => "Unsupported",
                };
                *key_layouts.entry(layout).or_insert(0usize) += 1;
                if matches!(track.keys, KeyData::Unsupported) {
                    let has_key_block = block.children.iter().any(|child| child.is_tag(b"KEY "));
                    *unsupported_tracks
                        .entry((track.target, track.format, track.key_count, has_key_block))
                        .or_insert(0usize) += 1;
                    if unsupported_track_examples.len() < 32 {
                        unsupported_track_examples.push(format!(
                            "{}@{:#x}: target={} format={:#x} count={} KEY={has_key_block}",
                            path.display(),
                            block.offset,
                            track.target,
                            track.format,
                            track.key_count
                        ));
                    }
                }
            }
        }

        let project = Project::from_file(&file).unwrap();
        for attribute in project
            .scenes
            .iter()
            .flat_map(|scene| &scene.layers)
            .flat_map(|layer| &layer.cast_attribute_lists)
            .flat_map(|list| &list.attributes)
        {
            *attribute_type_codes
                .entry(attribute.source_type_code)
                .or_insert(0usize) += 1;
            if let CastAttributeValue::Unsupported { type_code, bytes } = &attribute.value {
                *unsupported_attributes.entry(*type_code).or_insert(0usize) += 1;
                if unsupported_attribute_examples.len() < 32 {
                    unsupported_attribute_examples.push(format!(
                        "{}: name={:?} type={type_code:#x} bytes={bytes:02X?}",
                        path.display(),
                        String::from_utf8_lossy(&attribute.name)
                    ));
                }
            }
        }
    }

    eprintln!(
        "unsupported audit profile={profile:?}, tracks={track_count}, formats={track_formats:?}, layouts={key_layouts:?}, unsupported tracks={unsupported_tracks:?}, unsupported track examples={unsupported_track_examples:#?}, CATR types={attribute_type_codes:?}, unsupported CATR={unsupported_attributes:?}, unsupported CATR examples={unsupported_attribute_examples:#?}"
    );
    let (expected_track_count, expected_track_formats, expected_key_layouts, expected_catr_count) =
        match profile {
            CorpusProfile::Legacy53 => (
                116_566,
                [
                    (0x13, 51_334),
                    (0x23, 29_764),
                    (0x43, 8_371),
                    (0x51, 10_142),
                    (0x113, 10_271),
                    (0x123, 3_795),
                    (0x143, 1_535),
                    (0x151, 1_354),
                ]
                .into_iter()
                .collect(),
                [
                    ("Key20F32", 61_605),
                    ("Key20I32", 43_465),
                    ("Key8Bytes4", 11_496),
                ]
                .into_iter()
                .collect(),
                53_179,
            ),
            CorpusProfile::Complete91 => (
                150_327,
                [
                    (0x13, 67_140),
                    (0x23, 38_547),
                    (0x43, 10_983),
                    (0x51, 13_246),
                    (0x113, 12_308),
                    (0x123, 4_668),
                    (0x143, 1_805),
                    (0x151, 1_630),
                ]
                .into_iter()
                .collect(),
                [
                    ("Key20F32", 79_448),
                    ("Key20I32", 56_003),
                    ("Key8Bytes4", 14_876),
                ]
                .into_iter()
                .collect(),
                68_511,
            ),
        };
    assert_eq!(track_count, expected_track_count);
    assert_eq!(track_formats, expected_track_formats);
    assert_eq!(key_layouts, expected_key_layouts);
    assert!(unsupported_tracks.is_empty());
    assert!(unsupported_track_examples.is_empty());
    assert_eq!(
        attribute_type_codes,
        BTreeMap::from([(2, expected_catr_count)])
    );
    assert!(unsupported_attributes.is_empty());
    assert!(unsupported_attribute_examples.is_empty());
}

#[test]
fn parses_cast_attribute_lists_and_ext_params() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut list_count = 0usize;
    let mut attached_node_count = 0usize;
    let mut attribute_count = 0usize;
    let mut ext_param_count = 0usize;
    let mut font_param_count = 0usize;
    let mut font_param_modes = std::collections::BTreeMap::new();
    let mut initial_text_modes = std::collections::BTreeMap::new();
    let mut initial_textbox_flags = std::collections::BTreeMap::new();
    let mut initial_text_monospaced_count = 0usize;
    let mut initial_text_shadow_count = 0usize;
    let mut initial_text_vertical_count = 0usize;
    let mut initial_text_prohibition_count = 0usize;
    let mut initial_text_word_wrap_count = 0usize;
    let mut initial_text_point_sizes = std::collections::BTreeMap::new();
    let mut initial_text_style_flags = std::collections::BTreeMap::new();
    let mut initial_text_nonzero_style_counts = [0usize; 4];
    let mut initial_text_nondefault_point_y_guard_count = 0usize;
    let mut initial_text_scroll_fields = std::collections::BTreeMap::new();
    let mut initial_text_scroll_control_counts = [0usize; 2];
    let mut monospaced_font_param_count = 0usize;
    let mut render_preset_overrides = std::collections::BTreeMap::new();
    let mut image_override_counts = std::collections::BTreeMap::new();
    let mut effective_image_presets = std::collections::BTreeMap::new();
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        for layer in project.scenes.iter().flat_map(|scene| &scene.layers) {
            list_count += layer.cast_attribute_lists.len();
            attached_node_count += layer.cast_attribute_list_by_node.iter().flatten().count();
            for list in &layer.cast_attribute_lists {
                assert!(list.attributes.len() <= list.declared_count as usize);
                attribute_count += list.attributes.len();
                for attribute in &list.attributes {
                    if let CastAttributeValue::ExtParam { source, parsed } = &attribute.value {
                        assert_eq!(*parsed, srd_editor::attribute::ExtParamData::parse(source));
                        ext_param_count += 1;
                        *render_preset_overrides
                            .entry(parsed.render_preset_override)
                            .or_insert(0usize) += 1;
                    }
                }
                if let Some(font_param) = list.font_param() {
                    font_param_count += 1;
                    *font_param_modes
                        .entry(font_param.no_wrap_put_mode)
                        .or_insert(0usize) += 1;
                    monospaced_font_param_count += usize::from(font_param.monospaced);
                }
            }
            for (node_index, list_index) in layer.cast_attribute_list_by_node.iter().enumerate() {
                if let Some(list_index) = list_index {
                    assert_eq!(
                        layer.ext_param_for_node(node_index),
                        layer.cast_attribute_lists[*list_index].ext_param()
                    );
                    assert_eq!(
                        layer.font_param_for_node(node_index),
                        layer.cast_attribute_lists[*list_index].font_param()
                    );
                }
                if let Some(text) = layer.image_by_node[node_index]
                    .as_ref()
                    .and_then(|image| image.text.as_ref())
                {
                    let font_param = layer.font_param_for_node(node_index).unwrap_or_default();
                    let source_mode = font_param.no_wrap_put_mode;
                    let active_mode = if text.field_78.unwrap_or(0) & 1 != 0 {
                        0
                    } else {
                        source_mode
                    };
                    *initial_text_modes.entry(active_mode).or_insert(0usize) += 1;
                    let textbox_flags =
                        fennel_srd_textbox_flags(text.field_78.unwrap_or(0), font_param);
                    *initial_textbox_flags.entry(textbox_flags).or_insert(0usize) += 1;
                    let font_style = fennel_srd_font_style(font_param);
                    *initial_text_style_flags
                        .entry(font_style.record_flags)
                        .or_insert(0usize) += 1;
                    initial_text_monospaced_count += usize::from(font_param.monospaced);
                    initial_text_shadow_count += usize::from(font_param.display_shadow);
                    initial_text_vertical_count += usize::from(font_param.vertical);
                    initial_text_prohibition_count += usize::from(font_param.prohibition);
                    initial_text_word_wrap_count += usize::from(font_param.word_wrap);
                    initial_text_nonzero_style_counts[0] += usize::from(font_param.outline != 0);
                    initial_text_nonzero_style_counts[1] += usize::from(font_param.italic != 0);
                    initial_text_nonzero_style_counts[2] += usize::from(font_param.bold != 0);
                    initial_text_nonzero_style_counts[3] += usize::from(font_param.face_id != 0);
                    initial_text_nondefault_point_y_guard_count += usize::from(
                        font_param.point_y != 32
                            && textbox_flags & 0x800 != 0
                            && textbox_flags & 0x20 == 0,
                    );
                    *initial_text_scroll_fields
                        .entry((
                            font_param.scroll_speed,
                            font_param.scroll_wait,
                            font_param.field_34,
                        ))
                        .or_insert(0usize) += 1;
                    let text_units = decode_fennel_game_text(&text.text).unwrap();
                    initial_text_scroll_control_counts[0] += text_units
                        .windows(2)
                        .filter(|pair| pair == &[b'$' as u16, b'D' as u16])
                        .count();
                    initial_text_scroll_control_counts[1] += text_units
                        .windows(2)
                        .filter(|pair| pair == &[b'$' as u16, b'L' as u16])
                        .count();
                    *initial_text_point_sizes
                        .entry((font_param.point_x, font_param.point_y))
                        .or_insert(0usize) += 1;
                }
                let image = match layer.nodes[node_index].cast_type() {
                    Some(1) => layer.image_by_node[node_index].clone(),
                    Some(2) => layer.csli_by_node[node_index]
                        .as_ref()
                        .map(ImageDefinition::from_csli_runtime_base),
                    Some(4) => layer.number_by_node[node_index]
                        .as_ref()
                        .map(NumberDefinition::image_base),
                    _ => None,
                };
                let Some(image) = image else {
                    continue;
                };
                let override_value = layer
                    .ext_param_for_node(node_index)
                    .map_or(-1, |ext_param| ext_param.render_preset_override);
                *image_override_counts
                    .entry(override_value)
                    .or_insert(0usize) += 1;
                if let Some(preset) =
                    select_srd_image_render_preset(image.flags, override_value, false)
                {
                    *effective_image_presets.entry(preset).or_insert(0usize) += 1;
                }
            }
        }
    }

    let (expected_lists, expected_attributes, expected_overrides) = match profile {
        CorpusProfile::Legacy53 => (
            22_579,
            53_179,
            [
                (-1, 22_398),
                (41, 7),
                (42, 1),
                (43, 3),
                (53, 24),
                (54, 11),
                (57, 1),
                (58, 20),
                (60, 114),
            ]
            .into_iter()
            .collect(),
        ),
        CorpusProfile::Complete91 => (
            29_138,
            68_511,
            [
                (-1, 28_863),
                (34, 2),
                (35, 2),
                (36, 3),
                (37, 2),
                (38, 2),
                (39, 2),
                (40, 2),
                (41, 9),
                (42, 3),
                (43, 5),
                (44, 2),
                (45, 2),
                (46, 2),
                (47, 2),
                (48, 2),
                (49, 2),
                (50, 2),
                (51, 2),
                (52, 2),
                (53, 32),
                (54, 13),
                (55, 2),
                (56, 2),
                (57, 4),
                (58, 22),
                (60, 150),
            ]
            .into_iter()
            .collect(),
        ),
    };
    assert_eq!(list_count, expected_lists);
    assert_eq!(attached_node_count, expected_lists);
    assert_eq!(attribute_count, expected_attributes);
    assert_eq!(ext_param_count, expected_lists);
    assert_eq!(render_preset_overrides, expected_overrides);
    let (
        expected_font_param_count,
        expected_font_param_modes,
        expected_initial_text_modes,
        expected_initial_textbox_flags,
        expected_initial_text_style_flags,
        expected_monospaced_font_params,
        expected_initial_text_counts,
        expected_initial_point_sizes,
    ) = match profile {
        CorpusProfile::Legacy53 => (
            13_731,
            [(0, 13_613), (1, 1), (2, 14), (4, 103)]
                .into_iter()
                .collect(),
            [(0, 1_133), (2, 10), (4, 94)].into_iter().collect(),
            [
                (0, 1),
                (3, 577),
                (7, 289),
                (515, 5),
                (516, 1),
                (519, 260),
                (7_331, 8),
                (7_843, 2),
                (11_425, 1),
                (11_427, 93),
            ]
            .into_iter()
            .collect(),
            [(1, 1_152), (262_145, 85)].into_iter().collect(),
            268,
            (268, 85, 0, 1_235, 1_234),
            [((28, 28), 12), ((32, 32), 1_225)].into_iter().collect(),
        ),
        CorpusProfile::Complete91 => (
            17_825,
            [(0, 17_689), (1, 1), (2, 16), (4, 119)]
                .into_iter()
                .collect(),
            [(0, 1_173), (2, 12), (4, 107)].into_iter().collect(),
            [
                (0, 1),
                (3, 601),
                (7, 301),
                (515, 6),
                (516, 1),
                (519, 263),
                (7_331, 10),
                (7_843, 2),
                (11_425, 1),
                (11_427, 106),
            ]
            .into_iter()
            .collect(),
            [(1, 1_182), (262_145, 110)].into_iter().collect(),
            272,
            (272, 110, 0, 1_290, 1_289),
            [((21, 21), 2), ((28, 28), 12), ((32, 32), 1_278)]
                .into_iter()
                .collect(),
        ),
    };
    assert_eq!(font_param_count, expected_font_param_count);
    assert_eq!(font_param_modes, expected_font_param_modes);
    assert_eq!(initial_text_modes, expected_initial_text_modes);
    assert_eq!(initial_textbox_flags, expected_initial_textbox_flags);
    assert_eq!(initial_text_style_flags, expected_initial_text_style_flags);
    assert_eq!(monospaced_font_param_count, expected_monospaced_font_params);
    assert_eq!(
        (
            initial_text_monospaced_count,
            initial_text_shadow_count,
            initial_text_vertical_count,
            initial_text_prohibition_count,
            initial_text_word_wrap_count,
        ),
        expected_initial_text_counts
    );
    assert_eq!(initial_text_point_sizes, expected_initial_point_sizes);
    assert_eq!(initial_text_nonzero_style_counts, [0; 4]);
    assert_eq!(initial_text_nondefault_point_y_guard_count, 0);
    assert_eq!(
        initial_text_scroll_fields,
        match profile {
            CorpusProfile::Legacy53 => [
                ((40, 1, 2), 2),
                ((40, 2, 2), 1_208),
                ((40, 3, 2), 1),
                ((50, 2, 2), 5),
                ((60, 1, 2), 21),
            ]
            .into_iter()
            .collect(),
            CorpusProfile::Complete91 => [
                ((20, 2, 2), 1),
                ((40, 1, 2), 2),
                ((40, 2, 2), 1_258),
                ((40, 3, 2), 4),
                ((50, 2, 2), 5),
                ((60, 1, 2), 22),
            ]
            .into_iter()
            .collect(),
        }
    );
    assert_eq!(initial_text_scroll_control_counts, [0; 2]);
    eprintln!(
        "CATR profile={profile:?}, lists={list_count}, attached nodes={attached_node_count}, attributes={attribute_count}, ExtParamData={ext_param_count}, FontParamData={font_param_count}, FontParam modes={font_param_modes:?}, initial text modes={initial_text_modes:?}, initial TextBox flags={initial_textbox_flags:?}, initial record style flags={initial_text_style_flags:?}, monospaced FontParamData={monospaced_font_param_count}, initial text monospaced={initial_text_monospaced_count}, shadow={initial_text_shadow_count}, vertical={initial_text_vertical_count}, prohibition={initial_text_prohibition_count}, word-wrap={initial_text_word_wrap_count}, nonzero outline/italic/bold/faceId={initial_text_nonzero_style_counts:?}, point sizes={initial_text_point_sizes:?}, nondefault pointY in 7C04F0 point-margin branch={initial_text_nondefault_point_y_guard_count}, scroll fields={initial_text_scroll_fields:?}, $D/$L controls={initial_text_scroll_control_counts:?}, overrides={render_preset_overrides:?}, image overrides={image_override_counts:?}, effective image presets={effective_image_presets:?}"
    );
}

#[test]
fn reference_casts_resolve_inside_the_binary_project_scene_table() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut scene_count = 0usize;
    let mut reference_count = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project =
            Project::from_file(&file).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        scene_count += project.scenes.len();
        for scene in &project.scenes {
            for layer in &scene.layers {
                for reference in layer.reference_by_node.iter().flatten() {
                    let target = project.resolve_reference(reference).unwrap_or_else(|| {
                        panic!(
                            "{}: unresolved CRFD scene {:?}, layer {:?}",
                            path.display(),
                            String::from_utf8_lossy(&reference.source_name),
                            String::from_utf8_lossy(&reference.layer_name)
                        )
                    });
                    assert_eq!(
                        project.scenes[target.scene_index].name,
                        reference.source_name
                    );
                    assert_eq!(
                        project.scenes[target.scene_index].layers[target.layer_index].name,
                        reference.layer_name
                    );
                    reference_count += 1;
                }
            }
        }
    }

    assert!(scene_count > 0);
    assert_eq!(
        reference_count,
        match profile {
            CorpusProfile::Legacy53 => 1_090,
            CorpusProfile::Complete91 => 1_299,
        }
    );
    eprintln!("project scenes={scene_count}, resolved CRFD references={reference_count}");
}

#[test]
fn reference_runtime_construction_converges_for_the_local_corpus() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut definition_count = 0usize;
    let mut instance_count = 0usize;
    let mut multiply_instanced_targets = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project =
            Project::from_file(&file).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        definition_count += project
            .scenes
            .iter()
            .flat_map(|scene| &scene.layers)
            .flat_map(|layer| &layer.reference_by_node)
            .flatten()
            .count();
        let plan = project
            .build_reference_runtime_plan()
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(plan.unresolved.is_empty(), "{}", path.display());
        instance_count += plan.instances.len();

        let runtime = ProjectRuntime::new(&project)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let worlds = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(worlds.references.len(), plan.instances.len());
        for world in worlds
            .project_layers
            .iter()
            .flatten()
            .chain(&worlds.references)
        {
            let layer = &project.scenes[world.source.scene_index].layers[world.source.layer_index];
            assert_eq!(world.casts.len(), layer.nodes.len(), "{}", path.display());
        }

        let mut target_counts = std::collections::HashMap::new();
        for instance in &plan.instances {
            *target_counts.entry(instance.target).or_insert(0usize) += 1;
        }
        multiply_instanced_targets += target_counts.values().filter(|count| **count > 1).count();
    }

    assert_eq!(
        definition_count,
        match profile {
            CorpusProfile::Legacy53 => 1_090,
            CorpusProfile::Complete91 => 1_299,
        }
    );
    assert!(instance_count >= definition_count);
    assert!(multiply_instanced_targets > 0);
    eprintln!(
        "CRFD definitions={definition_count}, runtime reference layers={instance_count}, multiply-instanced targets={multiply_instanced_targets}"
    );
}

#[test]
fn copied_reference_text_casts_use_resources_requested_by_original_layers() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut copied_text_cast_count = 0usize;
    let mut copied_font_use_count = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project =
            Project::from_file(&file).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let requested_names = collect_fennel_font_resource_requests(&project)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            .into_iter()
            .map(|request| request.name)
            .collect::<BTreeSet<_>>();
        let plan = project
            .build_reference_runtime_plan()
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

        for instance in &plan.instances {
            let layer =
                &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
            for node_index in 0..layer.nodes.len() {
                let Some(image) = layer
                    .image_by_node
                    .get(node_index)
                    .and_then(Option::as_ref)
                    .filter(|image| image.creates_text_cast())
                else {
                    continue;
                };
                copied_text_cast_count += 1;
                let text = image.text.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{} copied SCN[{}]/LAYR[{}]/NODE[{node_index}] has no TEXT",
                        path.display(),
                        instance.target.scene_index,
                        instance.target.layer_index
                    )
                });
                let font_index = usize::try_from(text.font_index.unwrap_or(-1)).unwrap_or_else(|_| {
                    panic!(
                        "{} copied SCN[{}]/LAYR[{}]/NODE[{node_index}] has an invalid font index",
                        path.display(),
                        instance.target.scene_index,
                        instance.target.layer_index
                    )
                });
                let primary_name = &project.fonts[font_index].name;
                if !primary_name.is_empty() {
                    copied_font_use_count += 1;
                    assert!(
                        requested_names.contains(primary_name),
                        "{} copied SCN[{}]/LAYR[{}]/NODE[{node_index}] primary font {:?} was not requested by the original scene table",
                        path.display(),
                        instance.target.scene_index,
                        instance.target.layer_index,
                        String::from_utf8_lossy(primary_name)
                    );
                }

                let Some(attribute_list_index) = layer
                    .cast_attribute_list_by_node
                    .get(node_index)
                    .and_then(|index| *index)
                else {
                    continue;
                };
                for attribute in &layer.cast_attribute_lists[attribute_list_index].attributes {
                    if !matches!(
                        attribute.name.as_slice(),
                        b"rubyFont" | b"rfzOutlineFont" | b"rfzOutlineRubyFont"
                    ) {
                        continue;
                    }
                    let CastAttributeValue::String(name) = &attribute.value else {
                        panic!(
                            "{} copied SCN[{}]/LAYR[{}]/NODE[{node_index}] font CATR {:?} is not a string",
                            path.display(),
                            instance.target.scene_index,
                            instance.target.layer_index,
                            String::from_utf8_lossy(&attribute.name)
                        );
                    };
                    if name.is_empty() {
                        continue;
                    }
                    copied_font_use_count += 1;
                    assert!(
                        requested_names.contains(name),
                        "{} copied SCN[{}]/LAYR[{}]/NODE[{node_index}] CATR font {:?} was not requested by the original scene table",
                        path.display(),
                        instance.target.scene_index,
                        instance.target.layer_index,
                        String::from_utf8_lossy(name)
                    );
                }
            }
        }
    }

    let expected_copied_text_cast_count = match profile {
        CorpusProfile::Legacy53 => 1_449,
        CorpusProfile::Complete91 => 1_527,
    };
    assert_eq!(copied_text_cast_count, expected_copied_text_cast_count);
    assert_eq!(copied_font_use_count, expected_copied_text_cast_count);
    eprintln!(
        "{profile:?}: copied TextCasts={copied_text_cast_count}, covered font uses={copied_font_use_count}"
    );
}

#[test]
fn reference_instances_apply_the_binary_cast_channel_dispatch() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut instance_count = 0usize;
    let mut animation_count = 0usize;
    let mut common_channels = 0usize;
    let mut image_channels = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        let textures = TextureList::from_file(&file)
            .unwrap()
            .unwrap_or(TextureList {
                declared_count: 0,
                textures: Vec::new(),
            });
        let plan = project
            .build_reference_runtime_plan()
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for instance_index in 0..plan.instances.len() {
            let mut runtime =
                ReferenceLayerRuntimeState::new(&project, &plan, instance_index).unwrap();
            let target = plan.instances[instance_index].target;
            let layer = &project.scenes[target.scene_index].layers[target.layer_index];
            assert_eq!(runtime.image_bases.len(), layer.nodes.len());
            assert_eq!(runtime.image_states.len(), layer.nodes.len());
            for (node_index, state) in runtime.image_states.iter().enumerate() {
                assert_eq!(
                    state.render_preset_override,
                    layer
                        .ext_param_for_node(node_index)
                        .map_or(-1, |ext_param| ext_param.render_preset_override)
                );
            }
            let animation_names = layer
                .animations
                .iter()
                .map(|animation| animation.name.clone())
                .collect::<Vec<_>>();
            for animation_name in animation_names {
                let application = runtime
                    .apply_animation_channels(&project, &plan, &textures, &animation_name, 0.0)
                    .unwrap_or_else(|error| {
                        panic!(
                            "{} instance {instance_index} animation {:?}: {error}",
                            path.display(),
                            String::from_utf8_lossy(&animation_name)
                        )
                    })
                    .expect("animation disappeared from its owning layer");
                common_channels += application.common_channels;
                image_channels += application.image_channels;
                animation_count += 1;
            }
            instance_count += 1;
        }
    }

    assert_eq!(
        instance_count,
        match profile {
            CorpusProfile::Legacy53 => 2_087,
            CorpusProfile::Complete91 => 2_365,
        }
    );
    assert!(animation_count > 0);
    assert!(common_channels > 0);
    assert!(image_channels > 0);
    eprintln!(
        "reference instances={instance_count}, animations={animation_count}, common channels={common_channels}, SrImage channels={image_channels}"
    );
}

#[test]
fn reference_channel_23_recurses_through_real_runtime_instances() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut root_applications = 0usize;
    let mut reference_requests = 0usize;
    let mut recursively_animated_layers = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        let textures = TextureList::from_file(&file)
            .unwrap()
            .unwrap_or(TextureList {
                declared_count: 0,
                textures: Vec::new(),
            });
        let mut runtime = ProjectRuntime::new(&project)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for (scene_index, scene) in project.scenes.iter().enumerate() {
            for (layer_index, layer) in scene.layers.iter().enumerate() {
                let target = ReferenceTarget {
                    scene_index,
                    layer_index,
                };
                let animation_names = layer
                    .animations
                    .iter()
                    .map(|animation| animation.name.clone())
                    .collect::<Vec<_>>();
                for animation_name in animation_names {
                    let application = runtime
                        .apply_layer_animation(&project, &textures, target, &animation_name, 0.0)
                        .unwrap_or_else(|error| {
                            panic!(
                                "{} SCN[{scene_index}]/LAYR[{layer_index}] animation {:?}: {error}",
                                path.display(),
                                String::from_utf8_lossy(&animation_name)
                            )
                        })
                        .expect("animation disappeared from its owning layer");
                    root_applications += 1;
                    reference_requests += application.reference_requests;
                    recursively_animated_layers += application.animated_layers;
                }
            }
        }
    }

    assert!(root_applications > 0);
    assert!(reference_requests > 0);
    assert!(recursively_animated_layers > root_applications);
    eprintln!(
        "root applications={root_applications}, channel 23 requests={reference_requests}, recursively animated layers={recursively_animated_layers}"
    );
}

#[test]
fn avatar_track_uses_game_cubic_result() {
    let path = corpus_root!()
        .join("common")
        .join("commonAvatar")
        .join("CHU_UI_Common_Avatar_Position_00.srd");
    if !path.exists() {
        eprintln!("skipping: avatar sample not found at {}", path.display());
        return;
    }
    let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
    let animation = file
        .blocks_depth_first()
        .find(|block| {
            block.is_tag(b"ANIM")
                && block
                    .last_property(0x03)
                    .and_then(|property| property.string_bytes(&file))
                    == Some(b"001_Default_loop".as_slice())
        })
        .expect("animation not found");
    let motion = animation
        .children
        .iter()
        .find(|block| block.is_tag(b"MOT ") && signed_property(&file, block, 0x51) == Some(61))
        .expect("motion not found");
    let track_block = motion
        .children
        .iter()
        .find(|block| block.is_tag(b"TRK ") && unsigned_property(&file, block, 0x53) == Some(5))
        .expect("track not found");
    let track = Track::from_block(&file, track_block).unwrap();

    assert_eq!(track.format, 0x143);
    assert_eq!(track.key_count, 4);
    assert_eq!(
        track.evaluate(50.0),
        Evaluation::Value(ScalarValue::I32(349))
    );
}

#[test]
fn parses_binary_selected_layer_transform_records() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut layer_count = 0usize;
    let mut node_count = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        for block in file
            .blocks_depth_first()
            .filter(|block| block.is_tag(b"LAYR"))
        {
            let layer = Layer::from_block(&file, block).unwrap_or_else(|error| {
                panic!("{} at {:#x}: {error}", path.display(), block.offset)
            });
            assert_eq!(layer.nodes.len(), layer.transforms.len());
            let hierarchy = layer.build_hierarchy().unwrap_or_else(|error| {
                panic!("{} at {:#x}: {error}", path.display(), block.offset)
            });
            assert_eq!(hierarchy.parents.len(), layer.nodes.len());
            assert_eq!(hierarchy.children.len(), layer.nodes.len());
            let transforms = layer
                .transforms
                .iter()
                .copied()
                .map(|transform| transform.spatial())
                .collect::<Vec<_>>();
            let offsets = vec![[0.0, 0.0]; layer.nodes.len()];
            let worlds = layer
                .compose_world_matrices(&transforms, Affine3x4::IDENTITY, false, &offsets)
                .unwrap_or_else(|error| {
                    panic!("{} at {:#x}: {error}", path.display(), block.offset)
                });
            assert_eq!(worlds.len(), layer.nodes.len());
            layer_count += 1;
            node_count += layer.nodes.len();
        }
    }
    assert!(layer_count > 0);
    assert!(node_count > 0);
}

#[test]
fn parses_and_animates_common_transform_colors() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut non_identity_multiply = 0usize;
    let mut nonzero_additive = 0usize;
    let mut channel_counts = [0usize; 4];
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        for block in file
            .blocks_depth_first()
            .filter(|block| block.is_tag(b"LAYR"))
        {
            let layer = Layer::from_block(&file, block).unwrap();
            for transform in layer.transforms.iter().copied().map(|raw| raw.spatial()) {
                non_identity_multiply += usize::from(transform.multiply_color != [255; 4]);
                nonzero_additive += usize::from(transform.additive_color != [0; 4]);
            }

            for animation in block.children.iter().filter(|child| child.is_tag(b"ANIM")) {
                for motion_block in animation
                    .children
                    .iter()
                    .filter(|child| child.is_tag(b"MOT "))
                {
                    let motion = Motion::from_block(&file, motion_block).unwrap();
                    let Ok(node_index) = usize::try_from(motion.target) else {
                        continue;
                    };
                    let Some(base) = layer.transforms.get(node_index).copied() else {
                        continue;
                    };
                    for track in &motion.tracks {
                        let channel_index = match track.target {
                            9 => 0,
                            19 => 1,
                            21 => 2,
                            22 => 3,
                            _ => continue,
                        };
                        let mut transform = base.spatial();
                        assert!(transform.apply_common_track(
                            track.target,
                            track.evaluate(track.range_start as f32)
                        ));
                        channel_counts[channel_index] += 1;
                    }
                }
            }
        }
    }

    assert!(non_identity_multiply > 0);
    assert!(nonzero_additive > 0);
    assert!(channel_counts.iter().all(|count| *count > 0));
    eprintln!(
        "non-identity multiply colors={non_identity_multiply}, nonzero additive colors={nonzero_additive}, channels 9/19/21/22={channel_counts:?}"
    );
}

#[test]
fn parses_runtime_animation_slots_names_and_durations() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut animation_count = 0usize;
    let mut empty_motion_slots = 0usize;
    let mut automatic_durations = 0usize;
    let mut applied_common_channels = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        for scene in &project.scenes {
            for layer in &scene.layers {
                assert_eq!(layer.animations.len(), layer.animation_count as usize);
                for (animation_index, animation) in layer.animations.iter().enumerate() {
                    assert_eq!(
                        layer.find_animation(&animation.name).map(|entry| entry.0),
                        layer
                            .animations
                            .iter()
                            .position(|candidate| candidate.name == animation.name)
                    );
                    assert_eq!(
                        animation.motions.len(),
                        animation.declared_motion_count as usize
                    );
                    empty_motion_slots += animation
                        .motions
                        .iter()
                        .filter(|motion| motion.target < 0)
                        .count();
                    automatic_durations += usize::from(animation.duration < 0);
                    assert!(animation.runtime_duration() >= 0.0);

                    let mut transforms = layer
                        .transforms
                        .iter()
                        .copied()
                        .map(|raw| raw.spatial())
                        .collect::<Vec<_>>();
                    applied_common_channels += animation
                        .apply_common_channels(&mut transforms, 0.0)
                        .unwrap_or_else(|error| {
                            panic!("{} animation {animation_index}: {error}", path.display())
                        });
                    animation_count += 1;
                }
            }
        }
    }

    assert!(animation_count > 0);
    assert!(empty_motion_slots > 0);
    assert!(automatic_durations > 0);
    assert!(applied_common_channels > 0);
    eprintln!(
        "animations={animation_count}, empty MOT slots={empty_motion_slots}, automatic durations={automatic_durations}, applied common channels={applied_common_channels}"
    );
}

#[test]
fn parses_and_applies_scene_animation_sets() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut set_count = 0usize;
    let mut slot_count = 0usize;
    let mut enabled_slots = 0usize;
    let mut named_slots = 0usize;
    let mut unresolved_names = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        let textures = TextureList::from_file(&file)
            .unwrap()
            .unwrap_or(TextureList {
                declared_count: 0,
                textures: Vec::new(),
            });
        let mut runtime = ProjectRuntime::new(&project)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

        for (scene_index, scene) in project.scenes.iter().enumerate() {
            for (set_index, set) in scene.animation_sets.iter().enumerate() {
                assert_eq!(set.slots.len(), set.declared_slot_count as usize);
                assert!(set.slots.len() <= scene.layers.len());
                for (layer_index, slot) in set.slots.iter().enumerate() {
                    enabled_slots += usize::from(slot.is_enabled());
                    named_slots += usize::from(!slot.animation_name.is_empty());
                    unresolved_names += usize::from(
                        !slot.animation_name.is_empty()
                            && scene.layers[layer_index]
                                .find_animation(&slot.animation_name)
                                .is_none(),
                    );
                }
                runtime
                    .apply_animation_set(
                        &project,
                        &textures,
                        scene_index,
                        set_index,
                        set.start_frame as f32,
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "{} SCN[{scene_index}]/ANMS[{set_index}]: {error}",
                            path.display()
                        )
                    });
                for (layer_index, slot) in set.slots.iter().enumerate() {
                    assert_eq!(
                        runtime.project_layers[scene_index][layer_index].enabled,
                        slot.is_enabled()
                    );
                }
                set_count += 1;
                slot_count += set.slots.len();
            }
        }
    }

    assert!(set_count > 0);
    assert!(slot_count > 0);
    assert!(enabled_slots > 0);
    assert!(named_slots > 0);
    assert_eq!(unresolved_names, 0);
    eprintln!(
        "ANMS sets={set_count}, SANM slots={slot_count}, enabled={enabled_slots}, named={named_slots}"
    );
}

#[test]
fn parses_text_records_and_resolves_project_fonts() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut font_count = 0usize;
    let mut inline_character_count = 0usize;
    let mut text_count = 0usize;
    let mut nonempty_text_count = 0usize;
    let mut rfz_text_count = 0usize;
    let mut valid_rfz_utf8_count = 0usize;
    let mut plain_rfz_text_count = 0usize;
    let mut complete_static_rfz_text_count = 0usize;
    let mut missing_static_rfz_properties = BTreeMap::<u8, usize>::new();
    let mut text_flag_counts = BTreeMap::<u32, usize>::new();
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        assert_eq!(project.fonts.len(), project.declared_font_count as usize);
        font_count += project.fonts.len();
        inline_character_count += project
            .fonts
            .iter()
            .map(|font| font.characters.len())
            .sum::<usize>();

        for image in project
            .scenes
            .iter()
            .flat_map(|scene| &scene.layers)
            .flat_map(|layer| &layer.image_by_node)
            .flatten()
        {
            assert_eq!(image.has_text_child, image.text.is_some());
            let Some(text) = &image.text else {
                continue;
            };
            *text_flag_counts
                .entry(text.field_78.unwrap_or(0))
                .or_default() += 1;
            if let Some(font_index) = text.font_index.filter(|index| *index >= 0) {
                assert!(
                    project.resolve_text_font(text).is_some(),
                    "{} font index {font_index}",
                    path.display()
                );
            }
            if let Some(font) = project.resolve_text_font(text) {
                let lower_name = font
                    .name
                    .iter()
                    .map(u8::to_ascii_lowercase)
                    .collect::<Vec<_>>();
                if lower_name.ends_with(b".rfz") {
                    rfz_text_count += 1;
                    match FennelStaticTextProperties::from_text_definition(
                        text,
                        image.width,
                        image.height,
                    ) {
                        Ok(_) => complete_static_rfz_text_count += 1,
                        Err(error) => {
                            let srd_editor::fennel::FennelStaticTextPropertyError::MissingProperty {
                                code,
                            } = error;
                            *missing_static_rfz_properties.entry(code).or_default() += 1;
                        }
                    }
                    if let Ok(units) = decode_fennel_game_text(&text.text) {
                        valid_rfz_utf8_count += 1;
                        match tokenize_fennel_plain_text(&units) {
                            Ok(_) => plain_rfz_text_count += 1,
                            Err(error) => eprintln!(
                                "unsupported RFZ text control: {}: {error}; bytes={:02X?}",
                                path.display(),
                                text.text
                            ),
                        }
                    }
                }
            }
            nonempty_text_count += usize::from(!text.text.is_empty());
            text_count += 1;
        }
    }

    assert!(font_count > 0);
    assert!(text_count > 0);
    assert!(nonempty_text_count > 0);
    assert_eq!(valid_rfz_utf8_count, rfz_text_count);
    eprintln!(
        "FONT records={font_count}, inline CHAR mappings={inline_character_count}, TEXT records={text_count}, nonempty strings={nonempty_text_count}, RFZ texts={rfz_text_count}, complete static RFZ inputs={complete_static_rfz_text_count}, missing static RFZ properties={missing_static_rfz_properties:?}, valid RFZ UTF-8={valid_rfz_utf8_count}, plain RFZ token subset={plain_rfz_text_count}, TEXT 0x78={text_flag_counts:?}"
    );
}

#[test]
fn audits_binary_proven_static_fennel_layout_subset() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let Some(game_data_root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
        eprintln!("skipping: GAME_DATA_CORPUS is not set");
        return;
    };
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut runtime_fonts = BTreeMap::new();
    let mut text_count = 0usize;
    let mut fitting_count = 0usize;
    let mut default_layout_count = 0usize;
    let mut initial_unclipped_text_count = 0usize;
    let mut wrapped_text_count = 0usize;
    let mut automatic_wrap_count = 0usize;
    let mut vertical_overflow_text_count = 0usize;
    let mut batch_build_count = 0usize;
    let mut maximum_batch_count = 0usize;
    let mut batch_overflow_stop_count = 0usize;
    let mut vertex_batch_build_count = 0usize;
    let mut maximum_vertices_per_text = 0usize;
    let mut line_metadata_text_count = 0usize;
    let mut line_position_count = 0usize;
    let mut line_description_count = 0usize;
    let mut maximum_line_metadata_count = 0usize;
    let mut flag20_eligible_text_count = 0usize;
    let mut flag20_layout_count = 0usize;
    let mut flag20_nonzero_field_358_count = 0usize;
    let mut maximum_flag20_field_358 = 0usize;
    let mut mode1_layout_count = 0usize;
    let mut mode56_layout_count = 0usize;
    let mut mode56_nonterminating_wrap_count = 0usize;
    let mut record_error_counts = BTreeMap::<&'static str, usize>::new();
    let mut default_layout_error_counts = BTreeMap::<&'static str, usize>::new();
    let mut layout_error_counts = BTreeMap::<&'static str, usize>::new();
    let mut layout_error_samples = BTreeMap::<&'static str, Vec<String>>::new();
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        for image in project
            .scenes
            .iter()
            .flat_map(|scene| &scene.layers)
            .flat_map(|layer| &layer.image_by_node)
            .flatten()
        {
            let Some(text) = &image.text else {
                continue;
            };
            let Some(font) = project.resolve_text_font(text) else {
                continue;
            };
            if !font
                .name
                .iter()
                .map(u8::to_ascii_lowercase)
                .collect::<Vec<_>>()
                .ends_with(b".rfz")
            {
                continue;
            }
            text_count += 1;
            let properties =
                FennelStaticTextProperties::from_text_definition(text, image.width, image.height)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            initial_unclipped_text_count += usize::from(
                fennel_fresh_srd_textbox_flags(properties.layout.text_flags, 0)
                    & FENNEL_TEXTBOX_CLIP_FLAG
                    == 0,
            );
            if !runtime_fonts.contains_key(font.name.as_slice()) {
                let name = std::str::from_utf8(&font.name)
                    .unwrap_or_else(|error| panic!("{} font name: {error}", path.display()));
                let rfz_path = game_data_root.join("A000/font").join(name);
                let parsed = RuhunaFont::from_rfz(&fs::read(&rfz_path).unwrap())
                    .unwrap_or_else(|error| panic!("{}: {error}", rfz_path.display()));
                let runtime = parsed
                    .build_runtime_font(1, |page| u32::from(page) + 1)
                    .unwrap_or_else(|error| panic!("{}: {error}", rfz_path.display()));
                runtime_fonts.insert(font.name.clone(), runtime);
            }
            let runtime = runtime_fonts.get(font.name.as_slice()).unwrap();
            let mut stream = match build_fennel_plain_record_stream(
                &text.text,
                runtime,
                properties.glyph_placement(0, 0, [0; 4]),
                usize::MAX,
                |code, _| u32::from(code),
            ) {
                Ok(stream) => stream,
                Err(error) => {
                    let category = match error {
                        FennelPlainRecordError::Decode(_) => "decode",
                        FennelPlainRecordError::Control(_) => "control",
                        FennelPlainRecordError::MissingGlyph { .. } => "missing-glyph",
                        FennelPlainRecordError::RecordCapacity { .. } => "capacity",
                    };
                    *record_error_counts.entry(category).or_default() += 1;
                    continue;
                }
            };
            let mut default_stream = stream.clone();
            match layout_fennel_static_default(&mut default_stream, properties.layout, |token| {
                let code = u16::try_from(token).ok()?;
                runtime.glyph(code).map(FennelLayoutGlyphMetrics::from)
            }) {
                Ok(result) => {
                    default_layout_count += 1;
                    wrapped_text_count += usize::from(result.first_automatic_wrap.is_some());
                    automatic_wrap_count += result.automatic_wrap_count;
                    vertical_overflow_text_count += usize::from(result.vertical_overflow.is_some());
                    assert_eq!(
                        result.line_positions.len(),
                        result.line_descriptions.len(),
                        "{}",
                        path.display()
                    );
                    line_metadata_text_count += usize::from(!result.line_positions.is_empty());
                    line_position_count += result.line_positions.len();
                    line_description_count += result.line_descriptions.len();
                    maximum_line_metadata_count =
                        maximum_line_metadata_count.max(result.line_descriptions.len());
                    for description in &result.line_descriptions {
                        assert!(description.record_count > 0, "{}", path.display());
                        let end = description
                            .first_record_index
                            .checked_add(description.record_count)
                            .unwrap_or_else(|| panic!("{}: {description:?}", path.display()));
                        assert!(end <= default_stream.records.len(), "{}", path.display());
                    }
                    let batches = build_fennel_texture_batch_membership(&default_stream, -1)
                        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                    batch_build_count += 1;
                    maximum_batch_count = maximum_batch_count.max(batches.batches.len());
                    assert_eq!(
                        batches
                            .batches
                            .iter()
                            .map(|batch| batch.normal_glyph_count)
                            .sum::<usize>(),
                        batches.processed_glyph_count,
                        "{}",
                        path.display()
                    );
                    match (result.vertical_overflow, batches.stop) {
                        (
                            Some(overflow),
                            FennelTextureBatchStop::LineTableOverflow { record_index },
                        ) => {
                            assert_eq!(record_index, overflow.record_index, "{}", path.display());
                            batch_overflow_stop_count += 1;
                        }
                        (None, FennelTextureBatchStop::EndOfRecordArray) => {}
                        (layout, batch) => panic!(
                            "{} layout/batch stop mismatch: {layout:?}/{batch:?}",
                            path.display()
                        ),
                    }
                    let vertex_batches = build_fennel_static_unclipped_vertex_batches(
                        &default_stream,
                        -1,
                        FennelStaticUnclippedDrawInput {
                            is_2d: true,
                            textbox_position: [0.0; 3],
                            textbox_scale: [properties.layout.scale_x, properties.layout.scale_y],
                            textbox_vertical_offset: result.textbox_vertical_offset,
                            textbox_transform: identity_matrix4x4_game(),
                            secondary_color: 0,
                        },
                        |token| {
                            let code = u16::try_from(token).ok()?;
                            runtime.glyph(code).copied()
                        },
                    )
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                    vertex_batch_build_count += 1;
                    let vertex_count = vertex_batches
                        .batches
                        .iter()
                        .map(|batch| batch.vertices.len())
                        .sum::<usize>();
                    maximum_vertices_per_text = maximum_vertices_per_text.max(vertex_count);
                    assert_eq!(
                        vertex_count,
                        vertex_batches.processed_glyph_count * 6,
                        "{}",
                        path.display()
                    );
                    assert_eq!(vertex_batches.stop, batches.stop, "{}", path.display());
                }
                Err(error) => {
                    let category = match error {
                        FennelDefaultLayoutError::InvalidLineTable => "line-table",
                        FennelDefaultLayoutError::UnsupportedRecordKind { .. } => "record-kind",
                        FennelDefaultLayoutError::MissingGlyphMetrics { .. } => "glyph-metrics",
                        FennelDefaultLayoutError::NonProgressingZeroHeightWrap { .. } => {
                            "zero-height-wrap"
                        }
                        FennelDefaultLayoutError::NonTerminatingWrapWithoutVerticalCutoff {
                            ..
                        } => "nonterminating-no-cutoff-wrap",
                    };
                    *default_layout_error_counts.entry(category).or_default() += 1;
                }
            }
            if properties.layout.text_flags & 1 == 0 {
                flag20_eligible_text_count += 1;
                let mut flag20_stream = stream.clone();
                let result = layout_fennel_static_flag20(
                    &mut flag20_stream,
                    properties.layout,
                    0x0CA3,
                    |token| {
                        let code = u16::try_from(token).ok()?;
                        runtime.glyph(code).map(FennelLayoutGlyphMetrics::from)
                    },
                )
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                flag20_layout_count += 1;
                flag20_nonzero_field_358_count += usize::from(result.field_358 != 0);
                maximum_flag20_field_358 = maximum_flag20_field_358.max(result.field_358);
                assert!(
                    result.field_358 < result.layout.line_descriptions.len()
                        || result.layout.line_descriptions.is_empty(),
                    "{}",
                    path.display()
                );

                let mut mode1_stream = stream.clone();
                layout_fennel_static_mode1(&mut mode1_stream, properties.layout, |token| {
                    let code = u16::try_from(token).ok()?;
                    runtime.glyph(code).map(FennelLayoutGlyphMetrics::from)
                })
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                mode1_layout_count += 1;

                let mut mode56_stream = stream.clone();
                match layout_fennel_static_mode56(
                    &mut mode56_stream,
                    properties.layout,
                    5,
                    |token| {
                        let code = u16::try_from(token).ok()?;
                        runtime.glyph(code).map(FennelLayoutGlyphMetrics::from)
                    },
                ) {
                    Ok(_) => mode56_layout_count += 1,
                    Err(FennelMode56LayoutError::Layout(
                        FennelDefaultLayoutError::NonTerminatingWrapWithoutVerticalCutoff {
                            ..
                        },
                    )) => mode56_nonterminating_wrap_count += 1,
                    Err(error) => panic!("{}: {error}", path.display()),
                }
            }
            match layout_fennel_static_fitting_lines(&mut stream, properties.layout, |token| {
                let code = u16::try_from(token).ok()?;
                runtime.glyph(code).map(FennelLayoutGlyphMetrics::from)
            }) {
                Ok(_) => fitting_count += 1,
                Err(error) => {
                    let category = match error {
                        FennelFittingLayoutError::InvalidLineTable => "line-table",
                        FennelFittingLayoutError::UnsupportedRecordKind { .. } => "record-kind",
                        FennelFittingLayoutError::MissingGlyphMetrics { .. } => "glyph-metrics",
                        FennelFittingLayoutError::HorizontalWrapRequired { .. } => "wrap",
                        FennelFittingLayoutError::VerticalOverflow { .. } => "vertical-overflow",
                        FennelFittingLayoutError::NonTerminatingWrapWithoutVerticalCutoff {
                            ..
                        } => "nonterminating-no-cutoff-wrap",
                    };
                    *layout_error_counts.entry(category).or_default() += 1;
                    layout_error_samples
                        .entry(category)
                        .or_default()
                        .push(format!(
                            "{} box={}x{} flags={:#x} scale={:?} spacing={} text={:?}",
                            path.display(),
                            image.width,
                            image.height,
                            properties.layout.text_flags,
                            [properties.layout.scale_x, properties.layout.scale_y],
                            properties.layout.line_spacing,
                            String::from_utf8_lossy(&text.text)
                        ));
                }
            }
        }
    }

    eprintln!(
        "static RFZ texts={text_count}, initial unclipped texts={initial_unclipped_text_count}, default layout={default_layout_count}, wrapped texts={wrapped_text_count}, automatic logical wraps={automatic_wrap_count}, vertical-overflow texts={vertical_overflow_text_count}, texts with line metadata={line_metadata_text_count}, line positions={line_position_count}, line descriptions={line_description_count}, maximum line metadata/text={maximum_line_metadata_count}, Flag20-eligible texts={flag20_eligible_text_count}, Flag20 layouts={flag20_layout_count}, nonzero Flag20 +0x358={flag20_nonzero_field_358_count}, maximum Flag20 +0x358={maximum_flag20_field_358}, mode-1 layouts={mode1_layout_count}, mode-5 layouts={mode56_layout_count}, mode-5 nonterminating wraps={mode56_nonterminating_wrap_count}, batch builds={batch_build_count}, maximum texture batches/text={maximum_batch_count}, batch -254 stops={batch_overflow_stop_count}, vertex batch builds={vertex_batch_build_count}, maximum vertices/text={maximum_vertices_per_text}, default errors={default_layout_error_counts:?}, fitting subset={fitting_count}, record errors={record_error_counts:?}, fitting-only branches={layout_error_counts:?}"
    );
    for (category, samples) in &layout_error_samples {
        for sample in samples {
            eprintln!("  {category}: {sample}");
        }
    }
    assert!(text_count > 0);
    assert_eq!(initial_unclipped_text_count, text_count);
    assert!(fitting_count > 0);
    assert_eq!(default_layout_count, text_count);
    assert!(wrapped_text_count > 0);
    assert!(automatic_wrap_count >= wrapped_text_count);
    assert!(vertical_overflow_text_count > 0);
    assert_eq!(line_position_count, line_description_count);
    assert!(line_metadata_text_count > 0);
    assert!(maximum_line_metadata_count > 0);
    assert_eq!(flag20_layout_count, flag20_eligible_text_count);
    if profile == CorpusProfile::Complete91 {
        assert_eq!(line_metadata_text_count, 1_242);
        assert_eq!(line_position_count, 1_404);
        assert_eq!(line_description_count, 1_404);
        assert_eq!(maximum_line_metadata_count, 9);
        assert_eq!(flag20_eligible_text_count, 684);
        assert_eq!(flag20_layout_count, 684);
        assert_eq!(flag20_nonzero_field_358_count, 12);
        assert_eq!(maximum_flag20_field_358, 1);
        assert_eq!(mode1_layout_count, 684);
        assert_eq!(mode56_layout_count, 684);
        assert_eq!(mode56_nonterminating_wrap_count, 0);
    }
    assert_eq!(batch_build_count, text_count);
    assert!(maximum_batch_count > 0);
    assert_eq!(batch_overflow_stop_count, vertical_overflow_text_count);
    assert_eq!(vertex_batch_build_count, text_count);
    assert!(maximum_vertices_per_text > 0);
    assert!(default_layout_error_counts.is_empty());
    assert_eq!(
        text_count,
        fitting_count
            + record_error_counts.values().sum::<usize>()
            + layout_error_counts.values().sum::<usize>()
    );
}

#[test]
fn audits_fennel_font_resource_request_and_slot_order() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    let profile = srd_corpus_profile(files.len());

    let mut role_counts = [0usize; 4];
    let mut first_requests = 0usize;
    let mut cache_reuses = 0usize;
    let mut unregistered_requests = 0usize;
    let mut maximum_registered_slot = None::<u16>;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let project = Project::from_file(&file).unwrap();
        let requests = collect_fennel_font_resource_requests(&project)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for request in &requests {
            role_counts[match request.role {
                FennelTextFontRole::Primary => 0,
                FennelTextFontRole::Ruby => 1,
                FennelTextFontRole::Outline => 2,
                FennelTextFontRole::OutlineRuby => 3,
            }] += 1;
            assert!(!request.name.is_empty());
        }

        let mut registry = FennelFontSlotRegistry::default();
        for assignment in assign_fennel_font_resource_requests(&mut registry, requests) {
            if assignment.slot.first_request {
                first_requests += 1;
            } else {
                cache_reuses += 1;
            }
            if assignment.slot.registered {
                maximum_registered_slot = Some(
                    maximum_registered_slot.map_or(assignment.slot.font_slot_id, |slot| {
                        slot.max(assignment.slot.font_slot_id)
                    }),
                );
                assert_eq!(
                    registry.resource_for_slot(assignment.slot.font_slot_id),
                    Some(&assignment.request.name)
                );
            } else {
                unregistered_requests += 1;
            }
        }
    }

    let (expected_roles, expected_first_requests, expected_cache_reuses) = match profile {
        CorpusProfile::Legacy53 => ([1_237, 0, 0, 0], 139, 1_098),
        CorpusProfile::Complete91 => ([1_292, 0, 0, 0], 172, 1_120),
    };
    assert_eq!(role_counts, expected_roles);
    assert_eq!(first_requests, expected_first_requests);
    assert_eq!(cache_reuses, expected_cache_reuses);
    assert_eq!(unregistered_requests, 0);
    assert_eq!(maximum_registered_slot, Some(4));
    eprintln!(
        "Fennel font requests primary={} ruby={} outline={} outline-ruby={}, first resources={}, cache reuses={}, unregistered={}, maximum registered slot={maximum_registered_slot:?}",
        role_counts[0],
        role_counts[1],
        role_counts[2],
        role_counts[3],
        first_requests,
        cache_reuses,
        unregistered_requests,
    );
}

#[test]
fn parses_and_links_binary_proven_csli_grids() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();

    let mut csli_count = 0usize;
    let mut cref_count = 0usize;
    let mut resolved_cell_crefs = 0usize;
    let mut active_color_cells = 0usize;
    let mut missing_3a = 0usize;
    let mut missing_33 = 0usize;
    let mut non_four_44 = 0usize;
    let mut texture_count = 0usize;
    let mut crop_count = 0usize;
    let mut nonnegative_cell_crefs = 0usize;
    let mut resolved_cell_textures = 0usize;
    let mut indexed_children = 0usize;
    let mut active_indexed_children = 0usize;
    let mut image_count = 0usize;
    let mut image_cref_count = 0usize;
    let mut image_cre1_count = 0usize;
    let mut resolved_image_channels = 0usize;
    let mut text_cast_count = 0usize;
    let mut number_count = 0usize;
    let mut number_cref_count = 0usize;
    let mut number_cre1_count = 0usize;
    let mut resolved_number_channels = 0usize;
    let mut valid_number_special_glyphs = 0usize;
    let mut number_glyph_count = 0usize;
    let mut drawable_number_glyph_count = 0usize;
    let mut resolved_number_glyph_textures = 0usize;
    let mut animated_image_reference_evaluations = 0usize;
    let mut resolved_animated_image_references = 0usize;
    let mut animated_vertex_color_evaluations = 0usize;
    let mut animated_image_size_evaluations = 0usize;
    let mut reference_count = 0usize;
    let mut animated_reference_frame_evaluations = 0usize;
    for path in files {
        let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let textures = TextureList::from_file(&file).unwrap();
        if let Some(textures) = &textures {
            texture_count += textures.textures.len();
            crop_count += textures
                .textures
                .iter()
                .map(|texture| texture.crops.len())
                .sum::<usize>();
            for texture in &textures.textures {
                assert!(texture.filename.len() <= 255);
                assert_eq!(texture.crops.len(), texture.crop_count as usize);
            }
        }
        for block in file
            .blocks_depth_first()
            .filter(|block| block.is_tag(b"LAYR"))
        {
            let layer = Layer::from_block(&file, block).unwrap_or_else(|error| {
                panic!("{} at {:#x}: {error}", path.display(), block.offset)
            });
            let hierarchy = layer.build_hierarchy().unwrap();
            for (node_index, definition) in layer.csli_by_node.iter().enumerate() {
                let Some(definition) = definition else {
                    continue;
                };
                assert_eq!(layer.nodes[node_index].cast_type(), Some(2));
                assert_eq!(definition.cells.len(), definition.expected_cell_count());
                assert_eq!(definition.crefs.len(), usize::from(definition.cref_count));
                cref_count += definition.crefs.len();
                resolved_cell_crefs += (0..definition.cells.len())
                    .filter(|cell_index| definition.cell_cref(*cell_index).is_some())
                    .count();
                for cell_index in 0..definition.cells.len() {
                    let Some(cref) = definition.cell_cref(cell_index) else {
                        continue;
                    };
                    if cref.image_index >= 0 && cref.rectangle_index >= 0 {
                        nonnegative_cell_crefs += 1;
                        resolved_cell_textures += usize::from(
                            textures
                                .as_ref()
                                .and_then(|textures| {
                                    textures.resolve_slice_cell(definition, cell_index)
                                })
                                .is_some(),
                        );
                    }
                }
                for cell in definition
                    .cells
                    .iter()
                    .filter(|cell| (cell.flags >> 8) & 1 != 0)
                {
                    missing_3a += usize::from(cell.field_3a.is_none());
                    missing_33 += usize::from(cell.field_33.is_none());
                    non_four_44 += usize::from(cell.field_44.len() != 4);
                    active_color_cells += 1;
                }
                let explicit_first_row_widths = definition
                    .cells
                    .iter()
                    .take(usize::from(definition.columns))
                    .filter(|cell| cell.flags & 0x01 != 0)
                    .count();
                let explicit_first_column_heights = definition
                    .cells
                    .iter()
                    .step_by(usize::from(definition.columns).max(1))
                    .take(usize::from(definition.rows))
                    .filter(|cell| cell.flags & 0x02 != 0)
                    .count();
                assert_eq!(
                    usize::from(definition.explicit_width_cell_count),
                    explicit_first_row_widths
                );
                assert_eq!(
                    usize::from(definition.explicit_height_cell_count),
                    explicit_first_column_heights
                );
                assert_eq!(
                    definition.generate_cell_rects().unwrap().len(),
                    definition.expected_cell_count()
                );
                csli_count += 1;
            }
            for (node_index, definition) in layer.image_by_node.iter().enumerate() {
                let Some(definition) = definition else {
                    continue;
                };
                assert_eq!(layer.nodes[node_index].cast_type(), Some(1));
                image_count += 1;
                image_cref_count += definition.crefs.len();
                image_cre1_count += definition.cre1s.len();
                text_cast_count += usize::from(definition.creates_text_cast());
                for channel in [ImageReferenceChannel::Cref, ImageReferenceChannel::Cre1] {
                    let state = definition.initial_coordinate_state(channel);
                    let (declared_count, references) = match channel {
                        ImageReferenceChannel::Cref => {
                            (definition.cref_count, definition.crefs.as_slice())
                        }
                        ImageReferenceChannel::Cre1 => {
                            (definition.cre1_count, definition.cre1s.as_slice())
                        }
                    };
                    if state.reference_index < 0
                        || u32::from(state.reference_index as u16) >= u32::from(declared_count)
                        || references.is_empty()
                    {
                        continue;
                    }
                    let reference = references[state.reference_index as usize];
                    if reference.image_index < 0 || reference.rectangle_index < 0 {
                        continue;
                    }
                    definition
                        .resolve_coordinates(
                            channel,
                            state,
                            textures.as_ref().expect("CIMG reference requires TEXL"),
                            ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                        )
                        .unwrap_or_else(|error| {
                            panic!(
                                "{} CIMG NODE {node_index} {channel:?}: {error}",
                                path.display()
                            )
                        });
                    resolved_image_channels += 1;
                }
            }
            for (node_index, definition) in layer.number_by_node.iter().enumerate() {
                let Some(definition) = definition else {
                    continue;
                };
                assert_eq!(layer.nodes[node_index].cast_type(), Some(4));
                number_count += 1;
                number_cref_count += definition.crefs.len();
                number_cre1_count += definition.cre1s.len();
                let special = definition.special_glyphs();
                for glyph in [
                    special.plus,
                    special.minus,
                    special.comma,
                    special.decimal_point,
                ] {
                    if glyph >= 0 && u32::from(glyph as u16) < u32::from(definition.cref_count) {
                        valid_number_special_glyphs += 1;
                    }
                }
                let base = definition.image_base();
                for channel in [ImageReferenceChannel::Cref, ImageReferenceChannel::Cre1] {
                    let state = base.initial_coordinate_state(channel);
                    let (declared_count, references) = match channel {
                        ImageReferenceChannel::Cref => {
                            (definition.cref_count, definition.crefs.as_slice())
                        }
                        ImageReferenceChannel::Cre1 => {
                            (definition.cre1_count, definition.cre1s.as_slice())
                        }
                    };
                    if declared_count == 0 || references.is_empty() {
                        continue;
                    }
                    let reference = references[0];
                    if reference.image_index < 0 || reference.rectangle_index < 0 {
                        continue;
                    }
                    base.resolve_coordinates(
                        channel,
                        state,
                        textures.as_ref().expect("CNUM reference requires TEXL"),
                        NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "{} CNUM NODE {node_index} {channel:?}: {error}",
                            path.display()
                        )
                    });
                    resolved_number_channels += 1;
                }
                let formatted = definition.initial_formatted_text();
                assert!(
                    formatted
                        .combined
                        .iter()
                        .all(|byte| matches!(byte, b'0'..=b'9' | b'+' | b'-' | b',' | b'.'))
                );
                let glyphs = definition.build_glyph_records(&formatted, true);
                assert_eq!(
                    glyphs.len(),
                    definition.build_glyph_records(&formatted, false).len()
                );
                assert!(glyphs.len() <= formatted.combined.len());
                number_glyph_count += glyphs.len();
                for glyph in glyphs {
                    assert!(i32::from(glyph.glyph_index) < i32::from(definition.cref_count));
                    if !glyph.drawable() {
                        continue;
                    }
                    drawable_number_glyph_count += 1;
                    let reference = definition.crefs[glyph.glyph_index as usize];
                    if reference.image_index < 0 || reference.rectangle_index < 0 {
                        continue;
                    }
                    base.resolve_coordinates(
                        ImageReferenceChannel::Cref,
                        definition
                            .glyph_coordinate_state(glyph.glyph_index, ImageReferenceChannel::Cref),
                        textures.as_ref().expect("CNUM glyph requires TEXL"),
                        NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                    )
                    .unwrap_or_else(|error| {
                        panic!(
                            "{} CNUM NODE {node_index} glyph {}: {error}",
                            path.display(),
                            glyph.glyph_index
                        )
                    });
                    resolved_number_glyph_textures += 1;
                }
            }
            for (node_index, definition) in layer.reference_by_node.iter().enumerate() {
                let Some(definition) = definition else {
                    continue;
                };
                assert_eq!(layer.nodes[node_index].cast_type(), Some(3));
                assert!(definition.source_name.len() <= 512);
                assert!(definition.layer_name.len() <= 512);
                assert!(definition.animation_name.len() <= 512);
                reference_count += 1;
            }
            for animation in block.children.iter().filter(|child| child.is_tag(b"ANIM")) {
                for motion_block in animation
                    .children
                    .iter()
                    .filter(|child| child.is_tag(b"MOT "))
                {
                    let motion = Motion::from_block(&file, motion_block).unwrap();
                    let Ok(node_index) = usize::try_from(motion.target) else {
                        continue;
                    };
                    for track in motion.tracks.iter().filter(|track| track.target == 23) {
                        let definition = layer
                            .reference_by_node
                            .get(node_index)
                            .and_then(|definition| definition.as_ref())
                            .unwrap_or_else(|| {
                                panic!(
                                    "{} channel 23 NODE {node_index} has no CRFD",
                                    path.display()
                                )
                            });
                        let KeyData::Key20F32(keys) = &track.keys else {
                            panic!("reference frame track has non-f32 KEY data");
                        };
                        for (index, key) in keys.iter().enumerate() {
                            let request = definition
                                .animation_request(track, key.frame as f32)
                                .expect("enabled reference track produced no request");
                            assert_eq!(request.source_name, definition.source_name);
                            assert_eq!(request.layer_name, definition.layer_name);
                            assert_eq!(request.animation_name, definition.animation_name);
                            animated_reference_frame_evaluations += 1;
                            if let Some(next) = keys.get(index + 1) {
                                let request = definition
                                    .animation_request(
                                        track,
                                        (key.frame as f32 + next.frame as f32) * 0.5,
                                    )
                                    .expect("enabled reference midpoint produced no request");
                                assert_eq!(request.source_name, definition.source_name);
                                assert_eq!(request.layer_name, definition.layer_name);
                                assert_eq!(request.animation_name, definition.animation_name);
                                animated_reference_frame_evaluations += 1;
                            }
                        }
                    }
                    let base = layer
                        .image_by_node
                        .get(node_index)
                        .cloned()
                        .flatten()
                        .or_else(|| {
                            layer
                                .number_by_node
                                .get(node_index)
                                .and_then(|definition| definition.as_ref())
                                .map(NumberDefinition::image_base)
                        });
                    let Some(base) = base else {
                        continue;
                    };
                    let mut geometry = base.initial_geometry_state();
                    for track in motion
                        .tracks
                        .iter()
                        .filter(|track| matches!(track.target, 11 | 12))
                    {
                        let KeyData::Key20F32(keys) = &track.keys else {
                            panic!("image size track has non-f32 KEY data");
                        };
                        for (index, key) in keys.iter().enumerate() {
                            assert!(base.apply_size_track(&mut geometry, track, key.frame as f32));
                            animated_image_size_evaluations += 1;
                            if let Some(next) = keys.get(index + 1) {
                                assert!(base.apply_size_track(
                                    &mut geometry,
                                    track,
                                    (key.frame as f32 + next.frame as f32) * 0.5,
                                ));
                                animated_image_size_evaluations += 1;
                            }
                        }
                    }
                    for track in motion
                        .tracks
                        .iter()
                        .filter(|track| matches!(track.target, 13..=16))
                    {
                        let KeyData::Key8Bytes4(keys) = &track.keys else {
                            panic!("vertex color track has non-byte4 KEY data");
                        };
                        let mut state = base.initial_coordinate_state(ImageReferenceChannel::Cref);
                        for (index, key) in keys.iter().enumerate() {
                            assert!(base.apply_vertex_color_track(
                                &mut state,
                                track,
                                key.frame as f32
                            ));
                            animated_vertex_color_evaluations += 1;
                            if let Some(next) = keys.get(index + 1) {
                                assert!(base.apply_vertex_color_track(
                                    &mut state,
                                    track,
                                    (key.frame as f32 + next.frame as f32) * 0.5,
                                ));
                                animated_vertex_color_evaluations += 1;
                            }
                        }
                    }
                    for track in motion
                        .tracks
                        .iter()
                        .filter(|track| matches!(track.target, 17 | 20) && track.format & 3 == 3)
                    {
                        let KeyData::Key20I32(keys) = &track.keys else {
                            panic!("coordinate track has non-i32 KEY data");
                        };
                        let channel = if track.target == 17 {
                            ImageReferenceChannel::Cref
                        } else {
                            ImageReferenceChannel::Cre1
                        };
                        let mut state = base.initial_coordinate_state(channel);
                        let mut frames = Vec::with_capacity(keys.len().saturating_mul(2));
                        for (index, key) in keys.iter().enumerate() {
                            frames.push(key.frame as f32);
                            if let Some(next) = keys.get(index + 1) {
                                frames.push((key.frame as f32 + next.frame as f32) * 0.5);
                            }
                        }
                        for frame in frames {
                            if !base
                                .apply_coordinate_track(
                                    channel,
                                    &mut state,
                                    track,
                                    frame,
                                    textures
                                        .as_ref()
                                        .expect("coordinate animation requires TEXL"),
                                )
                                .unwrap_or_else(|error| {
                                    panic!(
                                        "{} NODE {node_index} channel {} frame {frame}: {error}",
                                        path.display(),
                                        track.target
                                    )
                                })
                            {
                                continue;
                            }
                            animated_image_reference_evaluations += 1;
                            let declared_count = match channel {
                                ImageReferenceChannel::Cref => base.cref_count,
                                ImageReferenceChannel::Cre1 => base.cre1_count,
                            };
                            if state.reference_index < 0
                                || u32::from(state.reference_index as u16)
                                    >= u32::from(declared_count)
                                || state.explicit_image_index < 0
                            {
                                continue;
                            }
                            base.resolve_coordinates(
                                channel,
                                state,
                                textures
                                    .as_ref()
                                    .expect("coordinate animation requires TEXL"),
                                ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                            )
                            .unwrap_or_else(|error| {
                                panic!(
                                    "{} NODE {node_index} channel {} frame {frame}: {error}",
                                    path.display(),
                                    track.target
                                )
                            });
                            resolved_animated_image_references += 1;
                        }
                    }
                }
            }
            for (node_index, node) in layer.nodes.iter().enumerate() {
                let Some(cell_index) = node.parent_csli_cell_index.filter(|index| *index >= 0)
                else {
                    continue;
                };
                let parent_index = hierarchy.parents[node_index].unwrap_or_else(|| {
                    panic!(
                        "{} NODE {node_index} has CSLI cell index but no parent",
                        path.display()
                    )
                });
                assert_eq!(layer.nodes[parent_index].cast_type(), Some(2));
                let definition = layer.csli_by_node[parent_index]
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!(
                            "{} parent NODE {parent_index} has no linked CSLI",
                            path.display()
                        )
                    });
                let cell = definition
                    .cells
                    .get(cell_index as usize)
                    .unwrap_or_else(|| {
                        panic!(
                            "{} NODE {node_index} indexes CSLI cell {cell_index} outside {} cells",
                            path.display(),
                            definition.cells.len()
                        )
                    });
                indexed_children += 1;
                active_indexed_children += usize::from((cell.flags >> 8) & 1 != 0);
            }
            let offsets = layer.compute_parent_csli_offsets().unwrap();
            assert_eq!(offsets.len(), layer.nodes.len());
            let transforms = layer
                .transforms
                .iter()
                .copied()
                .map(|transform| transform.spatial())
                .collect::<Vec<_>>();
            let worlds = layer
                .compose_world_matrices_with_csli_layout(&transforms, Affine3x4::IDENTITY, false)
                .unwrap();
            assert_eq!(worlds.len(), layer.nodes.len());
        }
    }
    eprintln!(
        "CSLI definitions={csli_count}, CREF records={cref_count}, resolved cell CREFs={resolved_cell_crefs}, CIMG definitions={image_count}, CIMG CREF records={image_cref_count}, CIMG CRE1 records={image_cre1_count}, resolved CIMG channels={resolved_image_channels}, text casts={text_cast_count}, CNUM definitions={number_count}, CNUM CREF records={number_cref_count}, CNUM CRE1 records={number_cre1_count}, resolved CNUM channels={resolved_number_channels}, valid CNUM special glyphs={valid_number_special_glyphs}, CNUM glyphs={number_glyph_count}, drawable CNUM glyphs={drawable_number_glyph_count}, resolved CNUM glyph textures={resolved_number_glyph_textures}, animated image reference evaluations={animated_image_reference_evaluations}, resolved animated image references={resolved_animated_image_references}, animated vertex color evaluations={animated_vertex_color_evaluations}, animated image size evaluations={animated_image_size_evaluations}, CRFD definitions={reference_count}, animated reference frame evaluations={animated_reference_frame_evaluations}, TEX records={texture_count}, CROP records={crop_count}, nonnegative cell CREFs={nonnegative_cell_crefs}, resolved cell textures={resolved_cell_textures}, active color cells={active_color_cells}, missing 0x3A={missing_3a}, missing 0x33={missing_33}, non-four 0x44={non_four_44}, indexed children={indexed_children}, active indexed children={active_indexed_children}"
    );
    assert!(csli_count > 0);
    assert!(cref_count > 0);
    assert!(resolved_cell_crefs > 0);
    assert!(image_count > 0);
    assert!(image_cref_count > 0);
    assert!(image_cre1_count > 0);
    assert!(resolved_image_channels > 0);
    assert!(number_count > 0);
    assert!(number_cref_count > 0);
    assert!(number_cre1_count > 0);
    assert!(resolved_number_channels > 0);
    assert!(valid_number_special_glyphs > 0);
    assert!(number_glyph_count > 0);
    assert!(drawable_number_glyph_count > 0);
    assert!(resolved_number_glyph_textures > 0);
    assert!(animated_image_reference_evaluations > 0);
    assert!(resolved_animated_image_references > 0);
    assert!(animated_vertex_color_evaluations > 0);
    assert!(animated_image_size_evaluations > 0);
    assert!(reference_count > 0);
    assert!(animated_reference_frame_evaluations > 0);
    assert!(texture_count > 0);
    assert!(crop_count > 0);
    assert!(nonnegative_cell_crefs > 0);
    assert_eq!(resolved_cell_textures, nonnegative_cell_crefs);
    assert!(active_color_cells > 0);
    assert_eq!(missing_3a, 0);
    assert_eq!(missing_33, active_color_cells);
    assert_eq!(non_four_44, 0);
    assert!(indexed_children > 0);
}

#[test]
fn avatar_motion_targets_runtime_cast_index() {
    let path = corpus_root!()
        .join("common")
        .join("commonAvatar")
        .join("CHU_UI_Common_Avatar_Position_00.srd");
    if !path.exists() {
        eprintln!("skipping: avatar sample not found at {}", path.display());
        return;
    }
    let file = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
    let animation = file
        .blocks_depth_first()
        .find(|block| {
            block.is_tag(b"ANIM")
                && block
                    .last_property(0x03)
                    .and_then(|property| property.string_bytes(&file))
                    == Some(b"001_Default_loop".as_slice())
        })
        .expect("animation not found");
    let motion = animation
        .children
        .iter()
        .find(|block| block.is_tag(b"MOT ") && signed_property(&file, block, 0x51) == Some(61))
        .expect("motion not found");
    let motion = Motion::from_block(&file, motion).unwrap();
    assert_eq!(motion.target, 61);
    assert!(motion.tracks.iter().any(|track| track.target == 5));

    let layer_block = file
        .blocks_depth_first()
        .find(|block| {
            block.is_tag(b"LAYR")
                && block.children.iter().any(|child| {
                    child.is_tag(b"ANIM")
                        && child
                            .last_property(0x03)
                            .and_then(|property| property.string_bytes(&file))
                            == Some(b"001_Default_loop".as_slice())
                })
        })
        .expect("owning layer not found");
    let layer = Layer::from_block(&file, layer_block).unwrap();
    let mut transform = layer.transforms[61].spatial();
    assert!(motion.apply_proven_common_channels(&mut transform, 50.0) > 0);
    assert_eq!(transform.rotation[2], 349);
}

fn unsigned_property(file: &SrdFile, block: &Block, code: u8) -> Option<u32> {
    block
        .last_property(code)
        .and_then(|property| property.read_unsigned_scalar(file))
}

fn signed_property(file: &SrdFile, block: &Block, code: u8) -> Option<i32> {
    block
        .last_property(code)
        .and_then(|property| property.read_signed_scalar(file))
}

#[test]
fn canonical_owned_vtbf_reemits_complete_corpus_byte_identically() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    assert_eq!(srd_corpus_profile(files.len()), CorpusProfile::Complete91);

    for path in files {
        let original = fs::read(&path).unwrap();
        let parsed = SrdFile::parse(original.clone())
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let encoded = parsed
            .to_owned()
            .encode()
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        if encoded != original {
            panic!(
                "{}: canonical owned VTBF differs at {:#x}",
                path.display(),
                first_byte_difference(&original, &encoded)
            );
        }
    }
}

#[test]
fn clean_editor_save_as_reemits_complete_corpus_byte_identically() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    assert_eq!(srd_corpus_profile(files.len()), CorpusProfile::Complete91);

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    for (index, path) in files.iter().enumerate() {
        let original = fs::read(path).unwrap();
        let destination = std::env::temp_dir().join(format!(
            "srd-editor-clean-corpus-save-{}-{nonce}-{index}.srd",
            std::process::id()
        ));
        let mut document = EditorDocument::load(path).unwrap();
        document.save_as(&destination).unwrap();
        assert_eq!(
            fs::read(&destination).unwrap(),
            original,
            "{}",
            path.display()
        );
        fs::remove_file(destination).unwrap();
    }
}

#[test]
fn owned_vtbf_structural_insert_preserves_existing_bytes() {
    let root = corpus_root!();
    if !root.exists() {
        eprintln!("skipping: SRD corpus not found at {}", root.display());
        return;
    }
    let mut files = Vec::new();
    collect_srd_files(&root, &mut files);
    files.sort();
    assert_eq!(srd_corpus_profile(files.len()), CorpusProfile::Complete91);
    let path = files.first().expect("complete corpus has a first SRD file");
    let original = fs::read(path).unwrap();
    let parsed = SrdFile::parse(original.clone()).unwrap();
    let original_root = parsed.blocks.first().expect("corpus file has a root block");
    let original_child = original_root
        .children
        .first()
        .expect("corpus root has an existing child");
    let original_child_end = parsed_block_end(original_child);
    let original_child_bytes = original[original_child.offset..original_child_end].to_vec();
    let original_property_bytes = original_root
        .properties
        .iter()
        .map(|property| property.encoded_bytes(&parsed).to_vec())
        .collect::<Vec<_>>();

    let mut owned = parsed.to_owned();
    let owned_root = owned.blocks.first_mut().expect("owned root block");
    owned_root
        .insert_property(
            owned_root.properties.len(),
            OwnedProperty::u16(0xee, 0x1234),
        )
        .unwrap();
    owned_root
        .insert_child(owned_root.children.len(), OwnedBlock::new(*b"TEST"))
        .unwrap();

    let encoded = owned.encode().unwrap();
    let reparsed = SrdFile::parse(encoded.clone()).unwrap();
    let reparsed_root = reparsed.blocks.first().expect("reparsed root block");
    let inserted_property = reparsed_root
        .last_property(0xee)
        .expect("inserted property is present");
    assert_eq!(inserted_property.type_code, 6);
    assert_eq!(inserted_property.value_bytes(&reparsed), &[0x34, 0x12]);
    assert_eq!(reparsed_root.children.last().unwrap().tag, *b"TEST");

    // The file header, root signature, root tag, every pre-existing root property, and the
    // original child subtree stay byte-for-byte identical. Only the root size/count fields and
    // the positions following the inserted records legitimately change.
    assert_eq!(&encoded[..20], &original[..20]);
    assert_eq!(&encoded[24..28], &original[24..28]);
    for (original_property, reparsed_property) in original_property_bytes
        .iter()
        .zip(&reparsed_root.properties[..original_property_bytes.len()])
    {
        assert_eq!(
            original_property.as_slice(),
            reparsed_property.encoded_bytes(&reparsed)
        );
    }
    let reparsed_child = reparsed_root
        .children
        .first()
        .expect("original child is still first");
    assert_eq!(
        &encoded[reparsed_child.offset..parsed_block_end(reparsed_child)],
        original_child_bytes
    );
}

fn parsed_block_end(block: &Block) -> usize {
    block.children.last().map_or(
        block.offset + 8 + block.size_field as usize,
        parsed_block_end,
    )
}

fn first_byte_difference(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| left.len().min(right.len()))
}
