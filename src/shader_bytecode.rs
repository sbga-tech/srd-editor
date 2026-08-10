use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;

pub const FIRST_FIXTURE_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] = *b"AAEBABBAAAGAAAAAAA";
pub const FIRST_TEXTURED_FIXTURE_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"AAEBABBAABGAAAAAAA";
pub const FIRST_2D_FIXTURE_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"EAEBABBAAAGAAAAAAA";
pub const FIRST_TEXTURED_2D_FIXTURE_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"EAEBABBAABGAAAAAAA";
/// Exact compact Simple-selector key compiled from the game's Cg source for a
/// normal 3D Fennel atlas batch (Ceylon vertex format 13).
pub const FENNEL_TEXTURED_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"AAMAAABAABGAAAAAAA";
/// Exact compact Simple-selector key for the same Fennel batch with
/// DrawPacket +0x60 bit 7 (`ShapeEnv2D`) set.
pub const FENNEL_TEXTURED_2D_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"EAMAAABAABGAAAAAAA";
/// Exact compact Simple-selector key selected by textured 2D SliceCast cells
/// whose SrImage field_0c contributes the additional pixel-shader branch.
pub const SLICE_TEXTURED_2D_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"EAEBABBAABCBAAAAAA";
/// Exact 3D counterpart selected by CommonBackGround's shipped alpha-test
/// image path. It is part of the original 82-key collection.
pub const SLICE_TEXTURED_3D_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"AAEBABBAABCBAAAAAA";
/// Second exact textured 2D SliceCast key observed in the complete game
/// corpus. The game's compiler emits the already packaged textured 2D pair
/// byte-for-byte for this selector variant.
pub const SLICE_TEXTURED_2D_VARIANT_I_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"EAEBABBAABIAAAAAAA";
/// 3D counterpart of the exact SliceCast selector variant above. The original
/// compiler emits the already packaged normal textured vertex/pixel pair.
pub const SLICE_TEXTURED_3D_VARIANT_I_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"AAEBABBAABIAAAAAAA";
/// Exact 2D SliceCast selector variant observed in CourseSelect. Its pixel
/// bytecode is distinct, so the compact key remains explicit rather than
/// being folded into a nearby packaged variant.
pub const SLICE_2D_VARIANT_CB_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"EAEBABBAAACBAAAAAA";
/// Exact normal 3D dual-texture ImageCast key used by Advertise title draws.
pub const DUAL_TEXTURE_3D_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"AAEBABBAADIAAAAAAA";
/// Exact 3D dual-texture key whose MultiTex0 selector value is 9.
pub const DUAL_TEXTURE_VARIANT_9_3D_SIMPLE_KEY: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH] =
    *b"AAEBABBAADIIEAAAAA";

pub const FIRST_FIXTURE_VERTEX_SHADER_SHA256: &str =
    "86669F24505A70D6DB560C6B2838EBA7D262B0206825BA3927658AB5A7112D61";
pub const FIRST_FIXTURE_PIXEL_SHADER_SHA256: &str =
    "B7D50CF8DAC3A981DB13F2B5C3C7CAF8935FC4392B385B516620F7584EC2E53F";
pub const FIRST_TEXTURED_FIXTURE_PIXEL_SHADER_SHA256: &str =
    "066761E3FE149084A9526FDD1A091138B9DC894EAC29FA707D71992E4ED4E23F";
pub const FIRST_2D_FIXTURE_VERTEX_SHADER_SHA256: &str =
    "A3E0CA2EFA3452A529DDE7E92EB638FAAD4A230DF5A5537D2D50BE53BC045BD4";
pub const SLICE_TEXTURED_PIXEL_SHADER_SHA256: &str =
    "23CAC67F21338BC63A62D66DF9D5502AA0ED349C414D9AF73670BD8ADDCDCE65";
pub const SLICE_VARIANT_CB_PIXEL_SHADER_SHA256: &str =
    "B674C9D61CBE8B051742BB59C2C42618E027EA03DFE9D1F90839793CBBDDE290";

pub struct EmbeddedSimpleShaderPair {
    pub vertex_shader: &'static [u32],
    pub pixel_shader: &'static [u32],
}

struct EmbeddedSimpleShaderEntry {
    key: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
    vertex_shader: &'static [u32],
    pixel_shader: &'static [u32],
}

include!("shader_bytecode_generated.rs");

pub const EMBEDDED_SIMPLE_SHADER_KEYS: [[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH]; 84] = {
    let mut keys = [[0; CEYLON_SIMPLE_SHADER_KEY_LENGTH]; 84];
    let mut index = 0;
    while index < GAME_SIMPLE_SHADER_KEYS.len() {
        keys[index] = GAME_SIMPLE_SHADER_KEYS[index];
        index += 1;
    }
    keys[82] = FENNEL_TEXTURED_SIMPLE_KEY;
    keys[83] = FENNEL_TEXTURED_2D_SIMPLE_KEY;
    keys
};

pub(crate) fn canonical_embedded_simple_shader_key(
    key: &[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
) -> Option<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH]> {
    let canonical = match *key {
        FENNEL_TEXTURED_SIMPLE_KEY => FIRST_TEXTURED_FIXTURE_SIMPLE_KEY,
        FENNEL_TEXTURED_2D_SIMPLE_KEY => FIRST_TEXTURED_2D_FIXTURE_SIMPLE_KEY,
        _ => *key,
    };
    GAME_SIMPLE_SHADER_KEYS
        .binary_search(&canonical)
        .ok()
        .map(|_| canonical)
}

pub fn embedded_simple_shader_pair(
    key: &[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
) -> Option<EmbeddedSimpleShaderPair> {
    let canonical = canonical_embedded_simple_shader_key(key)?;
    let index = GAME_SIMPLE_SHADER_ENTRIES
        .binary_search_by_key(&canonical, |entry| entry.key)
        .ok()?;
    let entry = &GAME_SIMPLE_SHADER_ENTRIES[index];
    Some(EmbeddedSimpleShaderPair {
        vertex_shader: entry.vertex_shader,
        pixel_shader: entry.pixel_shader,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_game_collection_and_fennel_fixture_keys_are_packaged() {
        assert_eq!(EMBEDDED_SIMPLE_SHADER_KEYS.len(), 84);
        assert_eq!(GAME_SIMPLE_SHADER_KEYS.len(), 82);
        let pair = embedded_simple_shader_pair(&FIRST_FIXTURE_SIMPLE_KEY).unwrap();
        assert_eq!(pair.vertex_shader.len() * 4, 332);
        assert_eq!(pair.pixel_shader.len() * 4, 216);
        assert_eq!(pair.vertex_shader[0], 0xFFFE0300);
        assert_eq!(pair.pixel_shader[0], 0xFFFF0300);
        assert_eq!(pair.vertex_shader.last(), Some(&0x0000FFFF));
        assert_eq!(pair.pixel_shader.last(), Some(&0x0000FFFF));

        let textured = embedded_simple_shader_pair(&FIRST_TEXTURED_FIXTURE_SIMPLE_KEY).unwrap();
        assert_eq!(textured.vertex_shader, pair.vertex_shader);
        assert_eq!(textured.pixel_shader.len() * 4, 248);
        assert_eq!(textured.pixel_shader[0], 0xFFFF0300);
        assert_eq!(textured.pixel_shader.last(), Some(&0x0000FFFF));

        let two_d = embedded_simple_shader_pair(&FIRST_2D_FIXTURE_SIMPLE_KEY).unwrap();
        assert_eq!(two_d.vertex_shader.len() * 4, 384);
        assert_eq!(two_d.pixel_shader, pair.pixel_shader);
        assert_ne!(two_d.vertex_shader, pair.vertex_shader);
        assert_eq!(two_d.vertex_shader.last(), Some(&0x0000ffff));

        let textured_two_d =
            embedded_simple_shader_pair(&FIRST_TEXTURED_2D_FIXTURE_SIMPLE_KEY).unwrap();
        assert_eq!(textured_two_d.vertex_shader, two_d.vertex_shader);
        assert_eq!(textured_two_d.pixel_shader, textured.pixel_shader);

        let fennel = embedded_simple_shader_pair(&FENNEL_TEXTURED_SIMPLE_KEY).unwrap();
        assert_eq!(fennel.vertex_shader.len() * 4, 332);
        assert_eq!(fennel.pixel_shader.len() * 4, 248);
        assert_eq!(fennel.vertex_shader, textured.vertex_shader);
        assert_eq!(fennel.pixel_shader, textured.pixel_shader);

        let fennel_two_d = embedded_simple_shader_pair(&FENNEL_TEXTURED_2D_SIMPLE_KEY).unwrap();
        assert_eq!(fennel_two_d.vertex_shader.len() * 4, 384);
        assert_eq!(fennel_two_d.pixel_shader.len() * 4, 248);
        assert_eq!(fennel_two_d.vertex_shader, textured_two_d.vertex_shader);
        assert_eq!(fennel_two_d.pixel_shader, textured_two_d.pixel_shader);

        let slice = embedded_simple_shader_pair(&SLICE_TEXTURED_2D_SIMPLE_KEY).unwrap();
        assert_eq!(slice.vertex_shader, two_d.vertex_shader);
        assert_eq!(slice.pixel_shader.len() * 4, 296);
        assert_ne!(slice.pixel_shader, textured.pixel_shader);
        assert_eq!(slice.pixel_shader[0], 0xFFFF0300);
        assert_eq!(slice.pixel_shader.last(), Some(&0x0000FFFF));

        let slice_3d = embedded_simple_shader_pair(&SLICE_TEXTURED_3D_SIMPLE_KEY).unwrap();
        assert_eq!(slice_3d.vertex_shader, pair.vertex_shader);
        assert_eq!(slice_3d.pixel_shader, slice.pixel_shader);

        let slice_variant_i =
            embedded_simple_shader_pair(&SLICE_TEXTURED_2D_VARIANT_I_SIMPLE_KEY).unwrap();
        assert_eq!(slice_variant_i.vertex_shader, textured_two_d.vertex_shader);
        assert_eq!(slice_variant_i.pixel_shader, textured_two_d.pixel_shader);

        let slice_variant_i_3d =
            embedded_simple_shader_pair(&SLICE_TEXTURED_3D_VARIANT_I_SIMPLE_KEY).unwrap();
        assert_eq!(slice_variant_i_3d.vertex_shader, textured.vertex_shader);
        assert_eq!(slice_variant_i_3d.pixel_shader, textured.pixel_shader);

        let slice_variant_cb =
            embedded_simple_shader_pair(&SLICE_2D_VARIANT_CB_SIMPLE_KEY).unwrap();
        assert_eq!(slice_variant_cb.vertex_shader, two_d.vertex_shader);
        assert_eq!(slice_variant_cb.pixel_shader.len() * 4, 252);
        assert_ne!(slice_variant_cb.pixel_shader, pair.pixel_shader);
        assert_ne!(slice_variant_cb.pixel_shader, slice.pixel_shader);
        assert_eq!(slice_variant_cb.pixel_shader.last(), Some(&0x0000FFFF));

        for key in [
            DUAL_TEXTURE_3D_SIMPLE_KEY,
            DUAL_TEXTURE_VARIANT_9_3D_SIMPLE_KEY,
        ] {
            let dual = embedded_simple_shader_pair(&key).unwrap();
            assert_eq!(dual.vertex_shader.first(), Some(&0xFFFE0300));
            assert_eq!(dual.pixel_shader.first(), Some(&0xFFFF0300));
            assert_eq!(dual.vertex_shader.last(), Some(&0x0000FFFF));
            assert_eq!(dual.pixel_shader.last(), Some(&0x0000FFFF));
        }

        let mut unsupported = FIRST_FIXTURE_SIMPLE_KEY;
        unsupported[0] = b'B';
        assert!(embedded_simple_shader_pair(&unsupported).is_none());
    }
}
