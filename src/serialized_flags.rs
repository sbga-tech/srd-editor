#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerializedFlagWord {
    pub record: &'static str,
    pub property: u8,
    pub known_mask: u32,
}

impl SerializedFlagWord {
    pub const fn unknown_mask(self) -> u32 {
        !self.known_mask
    }

    pub const fn unknown_set_bits(self, value: u32) -> u32 {
        value & self.unknown_mask()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerializedBooleanField {
    pub record: &'static str,
    pub property: u8,
}

pub const LAYER_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "LAYR",
    property: 0x20,
    known_mask: 0x0000_0101,
};
pub const NODE_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "NODE",
    property: 0x30,
    known_mask: 0x010F_07FF,
};
pub const ANIMATION_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "ANIM",
    property: 0x5F,
    known_mask: 0x0000_0001,
};
pub const TRACK_FORMAT: SerializedFlagWord = SerializedFlagWord {
    record: "TRK ",
    property: 0x54,
    known_mask: 0x0000_0373,
};
pub const IMAGE_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "CIMG",
    property: 0x49,
    known_mask: 0x0100_07FF,
};
pub const NUMBER_IMAGE_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "CNUM",
    property: 0x49,
    known_mask: 0x0100_06FF,
};
pub const SLICE_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "CSLI",
    property: 0x80,
    known_mask: 0x0100_06FF,
};
pub const SLICE_CELL_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "SLIC",
    property: 0x83,
    known_mask: 0x0000_03F3,
};
pub const TEXT_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "TEXT",
    property: 0x78,
    known_mask: 0x0000_003D,
};
pub const NUMBER_ALIGNMENT_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "CNUM",
    property: 0x78,
    known_mask: 0x0000_000C,
};
pub const NUMBER_FORMAT_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "CNUM",
    property: 0x80,
    known_mask: 0x0000_003F,
};
pub const TEXTURE_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "TEX ",
    property: 0x62,
    known_mask: 0x0000_0FF0,
};
pub const FONT_FLAGS: SerializedFlagWord = SerializedFlagWord {
    record: "FONT",
    property: 0x70,
    known_mask: 0x0000_0007,
};

pub const FLAG_WORDS: [SerializedFlagWord; 13] = [
    LAYER_FLAGS,
    NODE_FLAGS,
    ANIMATION_FLAGS,
    TRACK_FORMAT,
    IMAGE_FLAGS,
    NUMBER_IMAGE_FLAGS,
    SLICE_FLAGS,
    SLICE_CELL_FLAGS,
    TEXT_FLAGS,
    NUMBER_ALIGNMENT_FLAGS,
    NUMBER_FORMAT_FLAGS,
    TEXTURE_FLAGS,
    FONT_FLAGS,
];

pub const BOOLEAN_FIELDS: [SerializedBooleanField; 3] = [
    SerializedBooleanField {
        record: "TRS2/TRS3",
        property: 0x3B,
    },
    SerializedBooleanField {
        record: "SANM",
        property: 0x0F,
    },
    SerializedBooleanField {
        record: "CRFD",
        property: 0x82,
    },
];

pub fn set_bit_indices(bits: u32) -> impl Iterator<Item = u8> {
    (0..u32::BITS as u8).filter(move |bit| bits & (1u32 << bit) != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_masks_partition_every_u32_bit() {
        assert!(
            FLAG_WORDS
                .iter()
                .all(|word| word.known_mask & word.unknown_mask() == 0
                    && word.known_mask | word.unknown_mask() == u32::MAX)
        );
    }

    #[test]
    fn unknown_set_bits_reports_only_unlisted_bits() {
        assert_eq!(LAYER_FLAGS.unknown_set_bits(0x8000_0101), 0x8000_0000);
    }
}
