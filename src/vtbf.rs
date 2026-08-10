use std::fmt;
use std::ops::Range;

pub const TYPE_SIZES: [i8; 16] = [0, 1, -1, 1, 1, 2, 2, 2, 4, 4, 4, 4, 4, 8, 0, -1];
pub const MULTIPLIERS: [u32; 8] = [2, 3, 4, 9, 16, 0, 0, 0];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl ParseError {
    fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VTBF parse error at {:#x}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone)]
pub struct SrdFile {
    bytes: Vec<u8>,
    pub unknown_04: [u8; 4],
    pub format: [u8; 4],
    pub unknown_0e: [u8; 2],
    pub blocks: Vec<Block>,
    pub trailing: Range<usize>,
}

impl SrdFile {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, ParseError> {
        require(&bytes, 0, 16)?;
        if &bytes[0..4] != b"VTBF" {
            return Err(ParseError::new(0, "missing VTBF magic"));
        }

        let unknown_04 = bytes[4..8].try_into().unwrap();
        let format = bytes[8..12].try_into().unwrap();
        let block_count = read_u16(&bytes, 12)? as usize;
        let unknown_0e = bytes[14..16].try_into().unwrap();
        let mut cursor = 16;
        let mut blocks = Vec::with_capacity(block_count);
        for _ in 0..block_count {
            let (block, next) = parse_block(&bytes, cursor, 0)?;
            blocks.push(block);
            cursor = next;
        }

        Ok(Self {
            trailing: cursor..bytes.len(),
            bytes,
            unknown_04,
            format,
            unknown_0e,
            blocks,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn slice(&self, range: &Range<usize>) -> &[u8] {
        &self.bytes[range.clone()]
    }

    pub fn blocks_depth_first(&self) -> BlocksDepthFirst<'_> {
        BlocksDepthFirst {
            stack: self.blocks.iter().rev().collect(),
        }
    }
    /// Copies the parsed container into a mutable, range-free VTBF tree.
    pub fn to_owned(&self) -> OwnedSrdFile {
        OwnedSrdFile::from(self)
    }

    /// Canonically serializes this parsed file without relying on source ranges.
    pub fn encode_canonical(&self) -> Result<Vec<u8>, EncodeError> {
        self.to_owned().encode()
    }
}

/// An owned VTBF container suitable for structural edits and canonical serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedSrdFile {
    pub unknown_04: [u8; 4],
    pub format: [u8; 4],
    pub unknown_0e: [u8; 2],
    pub blocks: Vec<OwnedBlock>,
    pub trailing: Vec<u8>,
}

impl From<&SrdFile> for OwnedSrdFile {
    fn from(file: &SrdFile) -> Self {
        Self {
            unknown_04: file.unknown_04,
            format: file.format,
            unknown_0e: file.unknown_0e,
            blocks: file
                .blocks
                .iter()
                .map(|block| OwnedBlock::from_parsed(file, block))
                .collect(),
            trailing: file.slice(&file.trailing).to_vec(),
        }
    }
}

impl OwnedSrdFile {
    /// Canonically encodes this owned tree as a VTBF file.
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let block_count = u16::try_from(self.blocks.len()).map_err(|_| {
            EncodeError::new(format!(
                "top-level block count {} does not fit in u16",
                self.blocks.len()
            ))
        })?;
        let mut output = Vec::new();
        output.extend_from_slice(b"VTBF");
        output.extend_from_slice(&self.unknown_04);
        output.extend_from_slice(&self.format);
        output.extend_from_slice(&block_count.to_le_bytes());
        output.extend_from_slice(&self.unknown_0e);
        for block in &self.blocks {
            encode_owned_block(block, 0, &mut output)?;
        }
        output.extend_from_slice(&self.trailing);
        Ok(output)
    }
}

/// A mutable VTBF block whose children and properties own their serialized values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedBlock {
    pub tag: [u8; 4],
    pub properties: Vec<OwnedProperty>,
    pub property_tail: Vec<u8>,
    pub children: Vec<OwnedBlock>,
}

impl OwnedBlock {
    pub fn new(tag: [u8; 4]) -> Self {
        Self {
            tag,
            properties: Vec::new(),
            property_tail: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn insert_child(&mut self, index: usize, child: OwnedBlock) -> Result<(), TreeEditError> {
        if index > self.children.len() {
            return Err(TreeEditError::new(
                "child insertion",
                index,
                self.children.len(),
            ));
        }
        self.children.insert(index, child);
        Ok(())
    }

    pub fn remove_child(&mut self, index: usize) -> Result<OwnedBlock, TreeEditError> {
        if index >= self.children.len() {
            return Err(TreeEditError::new(
                "child removal",
                index,
                self.children.len(),
            ));
        }
        Ok(self.children.remove(index))
    }

    pub fn insert_property(
        &mut self,
        index: usize,
        property: OwnedProperty,
    ) -> Result<(), TreeEditError> {
        if index > self.properties.len() {
            return Err(TreeEditError::new(
                "property insertion",
                index,
                self.properties.len(),
            ));
        }
        self.properties.insert(index, property);
        Ok(())
    }

    pub fn remove_property(&mut self, index: usize) -> Result<OwnedProperty, TreeEditError> {
        if index >= self.properties.len() {
            return Err(TreeEditError::new(
                "property removal",
                index,
                self.properties.len(),
            ));
        }
        Ok(self.properties.remove(index))
    }

    fn from_parsed(file: &SrdFile, block: &Block) -> Self {
        Self {
            tag: block.tag,
            properties: block
                .properties
                .iter()
                .map(|property| OwnedProperty::from_parsed(file, property))
                .collect(),
            property_tail: file.slice(&block.property_tail).to_vec(),
            children: block
                .children
                .iter()
                .map(|child| Self::from_parsed(file, child))
                .collect(),
        }
    }
}

/// A mutable VTBF property with its decoded shape and owned value bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedProperty {
    pub code: u8,
    pub type_code: u8,
    pub count: u32,
    pub multiplier: u32,
    pub value: Vec<u8>,
}

impl OwnedProperty {
    /// Builds a type-2 string property from its raw string bytes.
    pub fn string(code: u8, value: impl Into<Vec<u8>>) -> Result<Self, EncodeError> {
        let value = value.into();
        if value.len() > 0x7fff {
            return Err(EncodeError::new(format!(
                "string property {code:#04x} has length {}, exceeding the VTBF limit 32767",
                value.len()
            )));
        }
        Ok(Self {
            code,
            type_code: 2,
            count: 1,
            multiplier: 1,
            value,
        })
    }
    /// Builds a type-1 unsigned byte scalar property.
    pub fn u8(code: u8, value: u8) -> Self {
        Self::scalar(code, 1, [value])
    }

    /// Builds a type-5 signed 16-bit scalar property.
    pub fn i16(code: u8, value: i16) -> Self {
        Self::scalar(code, 5, value.to_le_bytes())
    }

    /// Builds a type-6 unsigned 16-bit scalar property.
    pub fn u16(code: u8, value: u16) -> Self {
        Self::scalar(code, 6, value.to_le_bytes())
    }

    /// Builds a type-8 signed 32-bit scalar property.
    pub fn i32(code: u8, value: i32) -> Self {
        Self::scalar(code, 8, value.to_le_bytes())
    }

    /// Builds a type-9 unsigned 32-bit scalar property.
    pub fn u32(code: u8, value: u32) -> Self {
        Self::scalar(code, 9, value.to_le_bytes())
    }

    /// Builds a type-10 IEEE-754 single-precision scalar property.
    pub fn f32(code: u8, value: f32) -> Self {
        Self::scalar(code, 10, value.to_bits().to_le_bytes())
    }

    fn scalar<const N: usize>(code: u8, type_code: u8, value: [u8; N]) -> Self {
        Self {
            code,
            type_code,
            count: 1,
            multiplier: 1,
            value: value.to_vec(),
        }
    }

    fn from_parsed(file: &SrdFile, property: &Property) -> Self {
        Self {
            code: property.code,
            type_code: property.type_code,
            count: property.count,
            multiplier: property.multiplier,
            value: property.value_bytes(file).to_vec(),
        }
    }
}

/// A rejected structural insertion or removal index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEditError {
    pub operation: &'static str,
    pub index: usize,
    pub len: usize,
}

impl TreeEditError {
    fn new(operation: &'static str, index: usize, len: usize) -> Self {
        Self {
            operation,
            index,
            len,
        }
    }
}

impl fmt::Display for TreeEditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} index {} is invalid for a collection of length {}",
            self.operation, self.index, self.len
        )
    }
}

impl std::error::Error for TreeEditError {}

/// A value or structural shape that cannot be represented by canonical VTBF bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeError {
    pub message: String,
}

impl EncodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for EncodeError {}

fn encode_owned_block(
    block: &OwnedBlock,
    depth: usize,
    output: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    if depth > 1024 {
        return Err(EncodeError::new("block nesting exceeds 1024"));
    }
    let child_count = u16::try_from(block.children.len()).map_err(|_| {
        EncodeError::new(format!(
            "block {:?} has {} children, exceeding u16",
            block.tag,
            block.children.len()
        ))
    })?;
    let property_count = u16::try_from(block.properties.len()).map_err(|_| {
        EncodeError::new(format!(
            "block {:?} has {} properties, exceeding u16",
            block.tag,
            block.properties.len()
        ))
    })?;

    let mut own_data = Vec::new();
    own_data.extend_from_slice(&block.tag);
    own_data.extend_from_slice(&child_count.to_le_bytes());
    own_data.extend_from_slice(&property_count.to_le_bytes());
    for property in &block.properties {
        encode_owned_property(property, &mut own_data)?;
    }
    own_data.extend_from_slice(&block.property_tail);
    let size_field = u32::try_from(own_data.len()).map_err(|_| {
        EncodeError::new(format!(
            "block {:?} own data is {} bytes, exceeding u32",
            block.tag,
            own_data.len()
        ))
    })?;

    output.extend_from_slice(b"vtc0");
    output.extend_from_slice(&size_field.to_le_bytes());
    output.extend_from_slice(&own_data);
    for child in &block.children {
        encode_owned_block(child, depth + 1, output)?;
    }
    Ok(())
}

fn encode_owned_property(
    property: &OwnedProperty,
    output: &mut Vec<u8>,
) -> Result<(), EncodeError> {
    if property.type_code > 0x3f {
        return Err(EncodeError::new(format!(
            "property {:#04x} type code {} does not fit the VTBF flag field",
            property.code, property.type_code
        )));
    }
    if property.count == 0 {
        return Err(EncodeError::new(format!(
            "property {:#04x} has count zero",
            property.code
        )));
    }
    let has_count = property.count != 1;
    let has_multiplier = property.multiplier != 1;
    let multiplier_index = if has_multiplier {
        MULTIPLIERS
            .iter()
            .position(|&candidate| candidate == property.multiplier)
            .ok_or_else(|| {
                EncodeError::new(format!(
                    "property {:#04x} has unsupported multiplier {}",
                    property.code, property.multiplier
                ))
            })? as u8
    } else {
        0
    };

    if property.type_code == 2 {
        if property.value.len() > 0x7fff {
            return Err(EncodeError::new(format!(
                "string property {:#04x} has length {}, exceeding the VTBF limit 32767",
                property.code,
                property.value.len()
            )));
        }
    } else {
        let type_size = encoded_type_size(property.type_code)?;
        let expected_length = u64::from(property.count)
            .checked_mul(u64::from(property.multiplier))
            .and_then(|length| length.checked_mul(type_size as u64))
            .and_then(|length| usize::try_from(length).ok())
            .ok_or_else(|| {
                EncodeError::new(format!(
                    "property {:#04x} value length overflows usize",
                    property.code
                ))
            })?;
        if property.value.len() != expected_length {
            return Err(EncodeError::new(format!(
                "property {:#04x} type {} count {} multiplier {} has {} value bytes; expected {}",
                property.code,
                property.type_code,
                property.count,
                property.multiplier,
                property.value.len(),
                expected_length
            )));
        }
    }

    output.push(property.code);
    output.push(
        property.type_code
            | if has_count { 0x80 } else { 0 }
            | if has_multiplier { 0x40 } else { 0 },
    );
    if has_count || has_multiplier {
        let stored_count = property.count - 1;
        let count_width = has_count.then(|| minimal_count_width(stored_count));
        output.push(multiplier_index | count_width.map_or(0, CountWidth::extension_bits));
        if let Some(width) = count_width {
            write_stored_count(output, stored_count, width);
        }
    }
    if property.type_code == 2 {
        let length = property.value.len() as u16;
        if length < 0x80 {
            output.push(length as u8);
        } else {
            output.push(0x80 | (length >> 8) as u8);
            output.push(length as u8);
        }
    }
    output.extend_from_slice(&property.value);
    Ok(())
}

#[derive(Clone, Copy)]
enum CountWidth {
    Implicit,
    U8,
    U16,
    U32,
}

impl CountWidth {
    fn extension_bits(self) -> u8 {
        match self {
            Self::Implicit => 0,
            Self::U8 => 0x08,
            Self::U16 => 0x10,
            Self::U32 => 0x18,
        }
    }
}

fn minimal_count_width(stored_count: u32) -> CountWidth {
    match stored_count {
        0 => CountWidth::Implicit,
        1..=0xff => CountWidth::U8,
        0x100..=0xffff => CountWidth::U16,
        _ => CountWidth::U32,
    }
}

fn write_stored_count(output: &mut Vec<u8>, stored_count: u32, width: CountWidth) {
    match width {
        CountWidth::Implicit => {}
        CountWidth::U8 => output.push(stored_count as u8),
        CountWidth::U16 => output.extend_from_slice(&(stored_count as u16).to_le_bytes()),
        CountWidth::U32 => output.extend_from_slice(&stored_count.to_le_bytes()),
    }
}

fn encoded_type_size(type_code: u8) -> Result<usize, EncodeError> {
    let size = TYPE_SIZES
        .get(usize::from(type_code))
        .copied()
        .ok_or_else(|| EncodeError::new(format!("unknown VTBF type code {type_code}")))?;
    if size < 0 {
        return Err(EncodeError::new(format!(
            "unsupported negative-sized VTBF type code {type_code}"
        )));
    }
    Ok(size as usize)
}

pub struct BlocksDepthFirst<'a> {
    stack: Vec<&'a Block>,
}

impl<'a> Iterator for BlocksDepthFirst<'a> {
    type Item = &'a Block;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.stack.pop()?;
        self.stack.extend(block.children.iter().rev());
        Some(block)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub offset: usize,
    pub sub_sig: [u8; 4],
    pub size_field: u32,
    pub tag: [u8; 4],
    pub properties: Vec<Property>,
    pub property_tail: Range<usize>,
    pub children: Vec<Block>,
}

impl Block {
    pub fn is_tag(&self, tag: &[u8; 4]) -> bool {
        &self.tag == tag
    }

    pub fn properties_with_code(&self, code: u8) -> impl DoubleEndedIterator<Item = &Property> {
        self.properties
            .iter()
            .filter(move |property| property.code == code)
    }

    pub fn last_property(&self, code: u8) -> Option<&Property> {
        self.properties_with_code(code).next_back()
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub offset: usize,
    pub code: u8,
    pub flags: u8,
    pub type_code: u8,
    pub count: u32,
    pub multiplier: u32,
    pub encoded: Range<usize>,
    pub value: Range<usize>,
    pub string_prefix: Option<Range<usize>>,
}

impl Property {
    pub fn encoded_bytes<'a>(&self, file: &'a SrdFile) -> &'a [u8] {
        file.slice(&self.encoded)
    }

    pub fn value_bytes<'a>(&self, file: &'a SrdFile) -> &'a [u8] {
        file.slice(&self.value)
    }

    pub fn string_bytes<'a>(&self, file: &'a SrdFile) -> Option<&'a [u8]> {
        (self.type_code == 2).then(|| self.value_bytes(file))
    }

    pub fn read_unsigned_scalar(&self, file: &SrdFile) -> Option<u32> {
        let value = self.value_bytes(file);
        match self.type_code {
            1 | 3 | 4 => value.first().copied().map(u32::from),
            5..=7 => read_u16(value, 0).ok().map(u32::from),
            8 | 9 | 11 | 12 => read_u32(value, 0).ok(),
            10 => read_f32(value, 0)
                .ok()
                .map(cvtt_f32_to_i32)
                .map(|v| v as u32),
            _ => None,
        }
    }

    pub fn read_signed_scalar(&self, file: &SrdFile) -> Option<i32> {
        self.read_signed_scalar_at(file, 0)
    }

    pub fn read_signed_scalar_at(&self, file: &SrdFile, index: usize) -> Option<i32> {
        let size = usize::try_from(*TYPE_SIZES.get(usize::from(self.type_code))?).ok()?;
        if size == 0 {
            return None;
        }
        let offset = index.checked_mul(size)?;
        let value = self.value_bytes(file).get(offset..)?;
        match self.type_code {
            1 | 4 => value.first().copied().map(i32::from),
            3 => value.first().copied().map(|v| i32::from(v as i8)),
            5 | 7 => read_u16(value, 0).ok().map(|v| i32::from(v as i16)),
            6 => read_u16(value, 0).ok().map(i32::from),
            8 | 9 | 11 | 12 => read_u32(value, 0).ok().map(|v| v as i32),
            10 => read_f32(value, 0).ok().map(cvtt_f32_to_i32),
            _ => None,
        }
    }

    pub fn read_scalar_as_f32(&self, file: &SrdFile) -> Option<f32> {
        self.read_scalar_as_f32_at(file, 0)
    }

    pub fn read_scalar_as_f32_at(&self, file: &SrdFile, index: usize) -> Option<f32> {
        let size = usize::try_from(*TYPE_SIZES.get(usize::from(self.type_code))?).ok()?;
        if size == 0 {
            return None;
        }
        let offset = index.checked_mul(size)?;
        let value = self.value_bytes(file).get(offset..)?;
        match self.type_code {
            1 | 4 => value.first().copied().map(f32::from),
            3 => value.first().copied().map(|v| f32::from(v as i8)),
            5 | 7 => read_u16(value, 0).ok().map(|v| f32::from(v as i16)),
            6 => read_u16(value, 0).ok().map(f32::from),
            8 | 9 | 11 | 12 => read_u32(value, 0).ok().map(|v| (v as i32) as f32),
            10 => read_f32(value, 0).ok(),
            _ => None,
        }
    }
}

fn parse_block(data: &[u8], offset: usize, depth: usize) -> Result<(Block, usize), ParseError> {
    if depth > 1024 {
        return Err(ParseError::new(offset, "block nesting exceeds 1024"));
    }
    require(data, offset, 16)?;
    let sub_sig: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
    if &sub_sig != b"vtc0" {
        return Err(ParseError::new(offset, "block does not start with vtc0"));
    }

    let size_field = read_u32(data, offset + 4)?;
    if size_field < 8 {
        return Err(ParseError::new(
            offset + 4,
            "block size is smaller than its tag header",
        ));
    }
    let own_end = offset
        .checked_add(8)
        .and_then(|value| value.checked_add(size_field as usize))
        .ok_or_else(|| ParseError::new(offset + 4, "block size overflow"))?;
    if own_end > data.len() {
        return Err(ParseError::new(
            offset + 4,
            "block own-data range exceeds file",
        ));
    }

    let tag = data[offset + 8..offset + 12].try_into().unwrap();
    let child_count = read_u16(data, offset + 12)? as usize;
    let property_count = read_u16(data, offset + 14)? as usize;
    let mut property_cursor = offset + 16;
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        let (property, next) = parse_property(data, property_cursor, own_end)?;
        properties.push(property);
        property_cursor = next;
    }
    if property_cursor > own_end {
        return Err(ParseError::new(
            property_cursor,
            "properties exceed block own-data range",
        ));
    }

    let mut child_cursor = own_end;
    let mut children = Vec::with_capacity(child_count);
    for _ in 0..child_count {
        let (child, next) = parse_block(data, child_cursor, depth + 1)?;
        children.push(child);
        child_cursor = next;
    }

    Ok((
        Block {
            offset,
            sub_sig,
            size_field,
            tag,
            properties,
            property_tail: property_cursor..own_end,
            children,
        },
        child_cursor,
    ))
}

fn parse_property(
    data: &[u8],
    offset: usize,
    block_own_end: usize,
) -> Result<(Property, usize), ParseError> {
    require_to(data, offset, 2, block_own_end)?;
    let code = data[offset];
    let flags = data[offset + 1];
    let type_code = flags & 0x3f;
    let has_multiplier = flags & 0x40 != 0;
    let has_count = flags & 0x80 != 0;
    let mut cursor = offset + 2;
    let mut count = 1u32;
    let mut multiplier = 1u32;

    if has_count || has_multiplier {
        require_to(data, cursor, 1, block_own_end)?;
        let extension = data[cursor];
        cursor += 1;
        if has_count {
            let stored = match extension & 0x18 {
                0x08 => {
                    require_to(data, cursor, 1, block_own_end)?;
                    let value = u32::from(data[cursor]);
                    cursor += 1;
                    value
                }
                0x10 => {
                    require_to(data, cursor, 2, block_own_end)?;
                    let value = u32::from(read_u16(data, cursor)?);
                    cursor += 2;
                    value
                }
                0x18 => {
                    require_to(data, cursor, 4, block_own_end)?;
                    let value = read_u32(data, cursor)?;
                    cursor += 4;
                    value
                }
                _ => 0,
            };
            count = stored
                .checked_add(1)
                .ok_or_else(|| ParseError::new(cursor, "property count overflow"))?;
        }
        if has_multiplier {
            multiplier = MULTIPLIERS[usize::from(extension & 7)];
        }
    }

    let (string_prefix, value_start, value_end) = if type_code == 2 {
        require_to(data, cursor, 1, block_own_end)?;
        let prefix_start = cursor;
        let first = data[cursor];
        cursor += 1;
        let length = if first & 0x80 == 0 {
            usize::from(first)
        } else {
            require_to(data, cursor, 1, block_own_end)?;
            let second = data[cursor];
            cursor += 1;
            (usize::from(first & 0x7f) << 8) | usize::from(second)
        };
        let value_start = cursor;
        require_to(data, value_start, length, block_own_end)?;
        let value_end = value_start + length;
        (Some(prefix_start..value_start), value_start, value_end)
    } else {
        let size = TYPE_SIZES
            .get(usize::from(type_code))
            .copied()
            .ok_or_else(|| ParseError::new(offset + 1, format!("unknown type code {type_code}")))?;
        if size < 0 {
            return Err(ParseError::new(
                offset + 1,
                format!("unsupported negative-sized type code {type_code}"),
            ));
        }
        let length = u64::from(count)
            .checked_mul(u64::from(multiplier))
            .and_then(|value| value.checked_mul(size as u64))
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| ParseError::new(offset, "property byte length overflow"))?;
        let value_start = cursor;
        require_to(data, value_start, length, block_own_end)?;
        let value_end = value_start + length;
        (None, value_start, value_end)
    };

    Ok((
        Property {
            offset,
            code,
            flags,
            type_code,
            count,
            multiplier,
            encoded: offset..value_end,
            value: value_start..value_end,
            string_prefix,
        },
        value_end,
    ))
}

fn require(data: &[u8], offset: usize, size: usize) -> Result<(), ParseError> {
    require_to(data, offset, size, data.len())
}

fn require_to(data: &[u8], offset: usize, size: usize, end: usize) -> Result<(), ParseError> {
    let requested_end = offset
        .checked_add(size)
        .ok_or_else(|| ParseError::new(offset, "range overflow"))?;
    if requested_end > end || requested_end > data.len() {
        Err(ParseError::new(offset, format!("need {size} bytes")))
    } else {
        Ok(())
    }
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, ParseError> {
    require(data, offset, 2)?;
    Ok(u16::from_le_bytes(
        data[offset..offset + 2].try_into().unwrap(),
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, ParseError> {
    require(data, offset, 4)?;
    Ok(u32::from_le_bytes(
        data[offset..offset + 4].try_into().unwrap(),
    ))
}

fn read_f32(data: &[u8], offset: usize) -> Result<f32, ParseError> {
    Ok(f32::from_bits(read_u32(data, offset)?))
}

fn cvtt_f32_to_i32(value: f32) -> i32 {
    if !value.is_finite() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}
