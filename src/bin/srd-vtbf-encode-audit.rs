use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use srd_editor::vtbf::{Block, MULTIPLIERS, Property, SrdFile};

const EXAMPLE_LIMIT: usize = 8;
const DIVERGENCE_CLASSES: &[&str] = &[
    "canonical property header differs",
    "non-minimal stored-count width",
    "redundant has_count for count=1",
    "redundant has_multiplier for multiplier=1",
    "non-canonical zero multiplier index",
    "unused count-width extension bits",
    "unused multiplier-index extension bits",
    "reserved extension bits",
    "two-byte string prefix for length < 0x80",
    "opaque property tail omitted",
    "opaque file trailing bytes omitted",
    "stored size_field differs from parsed layout",
];

fn main() -> Result<(), Box<dyn Error>> {
    if env::args_os().nth(1).is_some() {
        return Err("usage: srd-vtbf-encode-audit (uses GAME_DATA_CORPUS)".into());
    }

    let root = corpus_root()?;
    let mut paths = Vec::new();
    collect_srd_files(&root, &mut paths)?;
    paths.sort();
    if paths.len() != 91 {
        return Err(format!(
            "expected the complete 91-file SRD corpus under {}, found {} files",
            root.display(),
            paths.len()
        )
        .into());
    }

    let mut audit = Audit::default();
    for path in paths {
        let file = SrdFile::parse(fs::read(&path)?).map_err(|error| {
            format!(
                "{}: VTBF parse failed at {:#x}: {}",
                path.display(),
                error.offset,
                error.message
            )
        })?;
        let label = display_path(&root, &path);
        let canonical = encode_file(&file, false)?;
        let canonical_with_opaque_ranges = encode_file(&file, true)?;

        audit.file_count += 1;
        if canonical == file.bytes() {
            audit.byte_identical_files += 1;
        } else {
            audit.canonical_file_mismatches.push(FileMismatch {
                file: label.clone(),
                original: file.bytes().to_vec(),
                canonical,
            });
        }
        if canonical_with_opaque_ranges == file.bytes() {
            audit.byte_identical_with_opaque_ranges += 1;
        } else {
            audit.opaque_aided_file_mismatches.push(FileMismatch {
                file: label.clone(),
                original: file.bytes().to_vec(),
                canonical: canonical_with_opaque_ranges,
            });
        }

        for block in &file.blocks {
            audit_block(&file, &label, block, &mut audit)?;
        }
        audit_trailing(&file, &label, &mut audit);
    }

    print_report(&audit);
    Ok(())
}

fn corpus_root() -> Result<PathBuf, Box<dyn Error>> {
    let root = env::var_os("GAME_DATA_CORPUS").ok_or("GAME_DATA_CORPUS is not set")?;
    Ok(PathBuf::from(root))
}

// This is the recursive `read_dir` discovery convention used by tests/corpus.rs.
fn collect_srd_files(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_srd_files(&path, output)?;
        } else if path.extension().is_some_and(|extension| extension == "srd") {
            output.push(path);
        }
    }
    Ok(())
}

/// Re-emits only decoded structural fields. `include_opaque_ranges` models the extra raw-byte
/// side channel a lossless writer would need for block property tails and the file trailer.
fn encode_file(file: &SrdFile, include_opaque_ranges: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut output = Vec::with_capacity(file.bytes().len());
    output.extend_from_slice(b"VTBF");
    output.extend_from_slice(&file.unknown_04);
    output.extend_from_slice(&file.format);
    output.extend_from_slice(
        &u16::try_from(file.blocks.len())
            .map_err(|_| "top-level block count does not fit in u16")?
            .to_le_bytes(),
    );
    output.extend_from_slice(&file.unknown_0e);
    for block in &file.blocks {
        encode_block(file, block, include_opaque_ranges, &mut output)?;
    }
    if include_opaque_ranges {
        output.extend_from_slice(file.slice(&file.trailing));
    }
    Ok(output)
}

fn encode_block(
    file: &SrdFile,
    block: &Block,
    include_opaque_ranges: bool,
    output: &mut Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    let mut own_data = Vec::new();
    own_data.extend_from_slice(&block.tag);
    own_data.extend_from_slice(
        &u16::try_from(block.children.len())
            .map_err(|_| "child count does not fit in u16")?
            .to_le_bytes(),
    );
    own_data.extend_from_slice(
        &u16::try_from(block.properties.len())
            .map_err(|_| "property count does not fit in u16")?
            .to_le_bytes(),
    );
    for property in &block.properties {
        own_data.extend_from_slice(&canonical_property(file, property)?);
    }
    if include_opaque_ranges {
        own_data.extend_from_slice(file.slice(&block.property_tail));
    }

    let size_field =
        u32::try_from(own_data.len()).map_err(|_| "canonical block size does not fit in u32")?;
    output.extend_from_slice(b"vtc0");
    output.extend_from_slice(&size_field.to_le_bytes());
    output.extend_from_slice(&own_data);
    for child in &block.children {
        encode_block(file, child, include_opaque_ranges, output)?;
    }
    Ok(())
}

fn canonical_property(file: &SrdFile, property: &Property) -> Result<Vec<u8>, Box<dyn Error>> {
    canonical_property_fields(
        property.code,
        property.type_code,
        property.count,
        property.multiplier,
        property.value_bytes(file),
    )
}

fn canonical_property_fields(
    code: u8,
    type_code: u8,
    count: u32,
    multiplier: u32,
    value: &[u8],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let has_count = count != 1;
    let has_multiplier = multiplier != 1;
    let mut output = Vec::with_capacity(2 + 5 + value.len());
    output.push(code);
    output
        .push(type_code | if has_count { 0x80 } else { 0 } | if has_multiplier { 0x40 } else { 0 });

    if has_count || has_multiplier {
        let stored_count = count.checked_sub(1).ok_or("parsed count was zero")?;
        let count_width = has_count.then(|| minimal_count_width(stored_count));
        let multiplier_index = if has_multiplier {
            MULTIPLIERS
                .iter()
                .position(|&candidate| candidate == multiplier)
                .ok_or_else(|| {
                    format!("no multiplier table index for parsed multiplier {multiplier}")
                })? as u8
        } else {
            0
        };
        let extension = multiplier_index | count_width.map_or(0, CountWidth::extension_bits);
        output.push(extension);
        if let Some(width) = count_width {
            write_stored_count(&mut output, stored_count, width);
        }
    }

    if type_code == 2 {
        let length = u16::try_from(value.len()).map_err(|_| "string is longer than u16")?;
        if length < 0x80 {
            output.push(length as u8);
        } else if length <= 0x7fff {
            output.push(0x80 | ((length >> 8) as u8));
            output.push(length as u8);
        } else {
            return Err("string is longer than the VTBF two-byte prefix supports".into());
        }
    }
    output.extend_from_slice(value);
    Ok(output)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CountWidth {
    Implicit,
    U8,
    U16,
    U32,
}

impl CountWidth {
    fn bytes(self) -> usize {
        match self {
            Self::Implicit => 0,
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

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

fn stored_count_width(extension: u8) -> CountWidth {
    match extension & 0x18 {
        0x08 => CountWidth::U8,
        0x10 => CountWidth::U16,
        0x18 => CountWidth::U32,
        _ => CountWidth::Implicit,
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

fn audit_block(
    file: &SrdFile,
    path: &str,
    block: &Block,
    audit: &mut Audit,
) -> Result<(), Box<dyn Error>> {
    audit.block_count += 1;
    let expected_size = 8usize
        .checked_add(
            block
                .properties
                .iter()
                .map(|property| property.encoded.len())
                .sum::<usize>(),
        )
        .and_then(|size| size.checked_add(block.property_tail.len()))
        .ok_or("parsed block size overflow")?;
    if usize::try_from(block.size_field)? != expected_size {
        let example = block_example(
            path,
            block,
            None,
            file.slice(&(block.offset + 4..block.offset + 8)),
            &(expected_size as u32).to_le_bytes(),
            format!(
                "stored size_field={} but parsed own-data size is {}",
                block.size_field, expected_size
            ),
        );
        audit.record("stored size_field differs from parsed layout", example);
    }

    for property in &block.properties {
        audit.property_count += 1;
        let canonical = canonical_property(file, property)?;
        let original = property.encoded_bytes(file);
        if original != canonical {
            audit.record(
                "canonical property header differs",
                property_example(
                    path,
                    block,
                    property,
                    original,
                    &canonical,
                    "decoded fields select a different property header".to_owned(),
                ),
            );
        }

        let has_count = property.flags & 0x80 != 0;
        let has_multiplier = property.flags & 0x40 != 0;
        if has_count {
            audit.has_count += 1;
            let extension = property_extension(file, property).expect("count requires extension");
            let actual_width = stored_count_width(extension);
            let minimal_width = minimal_count_width(property.count - 1);
            if actual_width > minimal_width {
                let example = property_example(
                    path,
                    block,
                    property,
                    original,
                    &canonical,
                    format!(
                        "count={} stores {} bytes, canonical stores {} bytes",
                        property.count,
                        actual_width.bytes(),
                        minimal_width.bytes()
                    ),
                );
                audit.nonminimal_count_width.push(example.clone());
                audit.record("non-minimal stored-count width", example);
            }
            if property.count == 1 {
                audit.record(
                    "redundant has_count for count=1",
                    property_example(
                        path,
                        block,
                        property,
                        original,
                        &canonical,
                        "has_count is set although decoded count is 1".to_owned(),
                    ),
                );
            }
        }
        if has_multiplier {
            audit.has_multiplier += 1;
            let extension =
                property_extension(file, property).expect("multiplier requires extension");
            let multiplier_index = extension & 7;
            if property.multiplier == 1 {
                audit.record(
                    "redundant has_multiplier for multiplier=1",
                    property_example(
                        path,
                        block,
                        property,
                        original,
                        &canonical,
                        "has_multiplier is set although decoded multiplier is 1".to_owned(),
                    ),
                );
            }
            if (5..=7).contains(&multiplier_index) {
                audit.zero_multiplier_indices[usize::from(multiplier_index - 5)] += 1;
                if multiplier_index != 5 {
                    audit.record(
                        "non-canonical zero multiplier index",
                        property_example(
                            path,
                            block,
                            property,
                            original,
                            &canonical,
                            format!(
                                "multiplier index {} decodes to 0; canonical table lookup picks index 5",
                                multiplier_index
                            ),
                        ),
                    );
                }
            }

            if property.type_code != 2
                && property.multiplier > 1
                && property
                    .count
                    .checked_mul(property.multiplier)
                    .is_some_and(|total| total <= u32::MAX)
            {
                let total = property.count * property.multiplier;
                audit.multiplier_factor_alternative_count += 1;
                *audit
                    .multiplier_factor_alternatives_by_multiplier
                    .entry(property.multiplier)
                    .or_default() += 1;
                if audit.multiplier_factor_alternative_examples.len() < EXAMPLE_LIMIT {
                    let count_only = canonical_property_fields(
                        property.code,
                        property.type_code,
                        total,
                        1,
                        property.value_bytes(file),
                    )?;
                    audit
                        .multiplier_factor_alternative_examples
                        .push(property_example_with_bytes(
                            path,
                            block,
                            property,
                            original,
                            &count_only,
                            format!(
                                "decoded count={} and multiplier={} give total={}; count-only can encode that same total",
                                property.count, property.multiplier, total
                            ),
                        ));
                }
            }
        }

        if let Some(extension) = property_extension(file, property) {
            if !has_count && extension & 0x18 != 0 {
                audit.record(
                    "unused count-width extension bits",
                    property_example(
                        path,
                        block,
                        property,
                        original,
                        &canonical,
                        format!(
                            "extension {extension:02X} has count-width bits although has_count is clear"
                        ),
                    ),
                );
            }
            if !has_multiplier && extension & 7 != 0 {
                audit.record(
                    "unused multiplier-index extension bits",
                    property_example(
                        path,
                        block,
                        property,
                        original,
                        &canonical,
                        format!(
                            "extension {extension:02X} has multiplier bits although has_multiplier is clear"
                        ),
                    ),
                );
            }
            if extension & 0xe0 != 0 {
                audit.record(
                    "reserved extension bits",
                    property_example(
                        path,
                        block,
                        property,
                        original,
                        &canonical,
                        format!(
                            "extension {extension:02X} has ignored high bits {:#02X}",
                            extension & 0xe0
                        ),
                    ),
                );
            }
        }

        if property.type_code == 2 {
            audit.string_count += 1;
            let prefix = property
                .string_prefix
                .as_ref()
                .expect("string property has a prefix");
            if prefix.len() == 2 && property.value.len() < 0x80 {
                audit.record(
                    "two-byte string prefix for length < 0x80",
                    property_example(
                        path,
                        block,
                        property,
                        original,
                        &canonical,
                        format!(
                            "string length {} uses a two-byte prefix",
                            property.value.len()
                        ),
                    ),
                );
            }
        }
    }

    if !block.property_tail.is_empty() {
        let bytes = file.slice(&block.property_tail);
        audit.property_tails.record(bytes);
        audit.record(
            "opaque property tail omitted",
            block_example(
                path,
                block,
                None,
                bytes,
                &[],
                format!("property_tail has {} opaque bytes", bytes.len()),
            ),
        );
    }

    for child in &block.children {
        audit_block(file, path, child, audit)?;
    }
    Ok(())
}

fn audit_trailing(file: &SrdFile, path: &str, audit: &mut Audit) {
    if file.trailing.is_empty() {
        return;
    }
    let bytes = file.slice(&file.trailing);
    audit.file_trailers.record(bytes);
    audit.record(
        "opaque file trailing bytes omitted",
        Example {
            file: path.to_owned(),
            offset: file.trailing.start,
            tag: None,
            code: None,
            original: bytes.to_vec(),
            canonical: Vec::new(),
            detail: format!("file trailing range has {} opaque bytes", bytes.len()),
        },
    );
}

fn property_extension(file: &SrdFile, property: &Property) -> Option<u8> {
    (property.flags & 0xc0 != 0).then(|| file.bytes()[property.offset + 2])
}

fn property_example(
    path: &str,
    block: &Block,
    property: &Property,
    original_property: &[u8],
    canonical_property: &[u8],
    detail: String,
) -> Example {
    property_example_with_bytes(
        path,
        block,
        property,
        original_property,
        canonical_property,
        detail,
    )
}

fn property_example_with_bytes(
    path: &str,
    block: &Block,
    property: &Property,
    original_property: &[u8],
    canonical_property: &[u8],
    detail: String,
) -> Example {
    let value_length = property.value.len();
    let original_header_length = original_property.len().saturating_sub(value_length);
    let canonical_header_length = canonical_property.len().saturating_sub(value_length);
    Example {
        file: path.to_owned(),
        offset: property.offset,
        tag: Some(block.tag),
        code: Some(property.code),
        original: original_property[..original_header_length].to_vec(),
        canonical: canonical_property[..canonical_header_length].to_vec(),
        detail,
    }
}

fn block_example(
    path: &str,
    block: &Block,
    code: Option<u8>,
    original: &[u8],
    canonical: &[u8],
    detail: String,
) -> Example {
    Example {
        file: path.to_owned(),
        offset: block.offset,
        tag: Some(block.tag),
        code,
        original: original.to_vec(),
        canonical: canonical.to_vec(),
        detail,
    }
}

#[derive(Default)]
struct Audit {
    file_count: usize,
    byte_identical_files: usize,
    byte_identical_with_opaque_ranges: usize,
    block_count: usize,
    property_count: usize,
    has_count: usize,
    has_multiplier: usize,
    string_count: usize,
    divergences: BTreeMap<&'static str, Divergence>,
    nonminimal_count_width: Vec<Example>,
    multiplier_factor_alternative_count: usize,
    multiplier_factor_alternatives_by_multiplier: BTreeMap<u32, usize>,
    multiplier_factor_alternative_examples: Vec<Example>,
    zero_multiplier_indices: [usize; 3],
    property_tails: OpaqueRanges,
    file_trailers: OpaqueRanges,
    canonical_file_mismatches: Vec<FileMismatch>,
    opaque_aided_file_mismatches: Vec<FileMismatch>,
}

impl Audit {
    fn record(&mut self, class: &'static str, example: Example) {
        let entry = self.divergences.entry(class).or_default();
        entry.occurrences += 1;
        entry.files.insert(example.file.clone());
        if entry.examples.len() < EXAMPLE_LIMIT {
            entry.examples.push(example);
        }
    }
}

#[derive(Default)]
struct Divergence {
    occurrences: usize,
    files: BTreeSet<String>,
    examples: Vec<Example>,
}

#[derive(Clone)]
struct Example {
    file: String,
    offset: usize,
    tag: Option<[u8; 4]>,
    code: Option<u8>,
    original: Vec<u8>,
    canonical: Vec<u8>,
    detail: String,
}

#[derive(Default)]
struct OpaqueRanges {
    count: usize,
    zero_filled: usize,
    nonzero: usize,
    sizes: BTreeMap<usize, usize>,
    contents: BTreeMap<Vec<u8>, usize>,
}

impl OpaqueRanges {
    fn record(&mut self, bytes: &[u8]) {
        self.count += 1;
        if bytes.iter().all(|&byte| byte == 0) {
            self.zero_filled += 1;
        } else {
            self.nonzero += 1;
        }
        *self.sizes.entry(bytes.len()).or_default() += 1;
        *self.contents.entry(bytes.to_vec()).or_default() += 1;
    }
}

struct FileMismatch {
    file: String,
    original: Vec<u8>,
    canonical: Vec<u8>,
}

fn print_report(audit: &Audit) {
    println!("VTBF canonical re-emission audit");
    println!("corpus files: {}", audit.file_count);
    println!(
        "blocks: {}; properties: {}; strings: {}",
        audit.block_count, audit.property_count, audit.string_count
    );
    println!(
        "canonical policy: encode decoded code/type/count/multiplier/value fields; use minimal count and string widths; use the first matching multiplier-table index; recompute block sizes; omit opaque property_tail and trailing bytes"
    );
    println!(
        "overall verdict: {}/{} files byte-identical under structural canonical encoding; {} differ",
        audit.byte_identical_files,
        audit.file_count,
        audit.file_count - audit.byte_identical_files
    );
    println!(
        "with property_tail and trailing bytes copied as an explicit raw-byte side channel: {}/{} byte-identical; {} differ",
        audit.byte_identical_with_opaque_ranges,
        audit.file_count,
        audit.file_count - audit.byte_identical_with_opaque_ranges
    );

    println!("\nDivergence classes");
    println!("{:<48} {:>12} {:>8}", "class", "occurrences", "files");
    for class in DIVERGENCE_CLASSES {
        let divergence = audit.divergences.get(class);
        println!(
            "{:<48} {:>12} {:>8}",
            class,
            divergence.map_or(0, |item| item.occurrences),
            divergence.map_or(0, |item| item.files.len())
        );
    }

    println!("\nCount extensions");
    println!("has_count properties: {}", audit.has_count);
    println!(
        "wider-than-minimal stored-count encodings: {}",
        audit.nonminimal_count_width.len()
    );
    print_examples(
        "Every wider-than-minimal stored-count case",
        &audit.nonminimal_count_width,
        true,
    );
    print_examples(
        "Redundant has_count examples",
        examples_for(audit, "redundant has_count for count=1"),
        false,
    );
    println!("has_multiplier properties: {}", audit.has_multiplier);
    print_examples(
        "Redundant has_multiplier examples",
        examples_for(audit, "redundant has_multiplier for multiplier=1"),
        false,
    );

    println!("\nMultiplier choices");
    println!(
        "properties whose total element count also fits a count-only encoding: {}",
        audit.multiplier_factor_alternative_count
    );
    print!("  multiplier histogram:");
    for (multiplier, count) in &audit.multiplier_factor_alternatives_by_multiplier {
        print!(" {multiplier}:{count}");
    }
    println!();
    println!(
        "These are alternative encodings if only the total is retained. The parsed (count, multiplier) pair distinguishes every nonzero multiplier-table value; only indices 5/6/7 collapse to multiplier=0."
    );
    print_examples(
        "Representative count-only versus count×multiplier alternatives",
        &audit.multiplier_factor_alternative_examples,
        false,
    );
    println!(
        "zero-valued multiplier-table index use: index 5 = {}, index 6 = {}, index 7 = {}",
        audit.zero_multiplier_indices[0],
        audit.zero_multiplier_indices[1],
        audit.zero_multiplier_indices[2]
    );
    print_examples(
        "Non-canonical zero multiplier-index examples",
        examples_for(audit, "non-canonical zero multiplier index"),
        false,
    );

    print_examples(
        "Two-byte string prefix examples for lengths < 0x80",
        examples_for(audit, "two-byte string prefix for length < 0x80"),
        false,
    );
    print_opaque_ranges("property_tail", &audit.property_tails);
    print_opaque_ranges("file trailing", &audit.file_trailers);

    println!("\nSize-field mismatches");
    print_examples(
        "Stored size_field mismatches",
        examples_for(audit, "stored size_field differs from parsed layout"),
        true,
    );

    println!("\nOther ignored extension-bit choices");
    for class in [
        "unused count-width extension bits",
        "unused multiplier-index extension bits",
        "reserved extension bits",
    ] {
        print_examples(class, examples_for(audit, class), false);
    }

    println!("\nFile-level first-byte mismatch examples");
    for mismatch in audit.canonical_file_mismatches.iter().take(EXAMPLE_LIMIT) {
        let offset = first_difference(&mismatch.original, &mismatch.canonical);
        println!(
            "  file={} first_difference=0x{offset:08X} original={} canonical={}",
            mismatch.file,
            hex_window(&mismatch.original, offset),
            hex_window(&mismatch.canonical, offset),
        );
    }
    if !audit.opaque_aided_file_mismatches.is_empty() {
        println!("\nRaw-side-channel-aided mismatches (indicate a non-opaque encoding difference)");
        for mismatch in audit
            .opaque_aided_file_mismatches
            .iter()
            .take(EXAMPLE_LIMIT)
        {
            let offset = first_difference(&mismatch.original, &mismatch.canonical);
            println!(
                "  file={} first_difference=0x{offset:08X} original={} canonical={}",
                mismatch.file,
                hex_window(&mismatch.original, offset),
                hex_window(&mismatch.canonical, offset),
            );
        }
    }
}

fn examples_for<'a>(audit: &'a Audit, class: &str) -> &'a [Example] {
    audit
        .divergences
        .get(class)
        .map_or(&[], |divergence| divergence.examples.as_slice())
}

fn print_examples(title: &str, examples: &[Example], every_case: bool) {
    println!("\n{title}: {}", examples.len());
    if examples.is_empty() {
        return;
    }
    let limit = if every_case {
        examples.len()
    } else {
        EXAMPLE_LIMIT.min(examples.len())
    };
    for example in &examples[..limit] {
        let tag = example
            .tag
            .as_ref()
            .map(|tag| String::from_utf8_lossy(tag).into_owned())
            .unwrap_or_else(|| "-".to_owned());
        let code = example
            .code
            .map(|code| format!("0x{code:02X}"))
            .unwrap_or_else(|| "-".to_owned());
        println!(
            "  file={} offset=0x{:08X} tag={} code={} original=[{}] canonical=[{}] {}",
            example.file,
            example.offset,
            tag,
            code,
            hex(&example.original),
            hex(&example.canonical),
            example.detail,
        );
    }
}

fn print_opaque_ranges(label: &str, ranges: &OpaqueRanges) {
    println!("\n{label}: {} non-empty ranges", ranges.count);
    println!(
        "  zero-filled={} nonzero={}",
        ranges.zero_filled, ranges.nonzero
    );
    if ranges.sizes.is_empty() {
        println!("  size histogram: none");
        return;
    }
    print!("  size histogram:");
    for (size, count) in &ranges.sizes {
        print!(" {size}:{count}");
    }
    println!();
    println!("  exact byte sequences:");
    for (bytes, count) in &ranges.contents {
        println!("    {count} × [{}]", hex(bytes));
    }
}

fn first_difference(original: &[u8], canonical: &[u8]) -> usize {
    original
        .iter()
        .zip(canonical)
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| original.len().min(canonical.len()))
}

fn hex_window(bytes: &[u8], offset: usize) -> String {
    let end = (offset + 16).min(bytes.len());
    if offset >= end {
        "<end>".to_owned()
    } else {
        hex(&bytes[offset..end])
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
