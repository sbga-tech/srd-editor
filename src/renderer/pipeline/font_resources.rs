use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelTextFontRole {
    Primary,
    Ruby,
    Outline,
    OutlineRuby,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelFontResourceRequest {
    pub scene_index: usize,
    pub layer_index: usize,
    pub node_index: usize,
    pub role: FennelTextFontRole,
    pub name: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelFontResourceAssignment {
    pub request: FennelFontResourceRequest,
    pub slot: FennelFontSlotRequest,
}
/// Collects the font-resource requests issued by `sub_AAE6C0` while it walks
/// the player's original runtime scene table.
///
/// The binary traverses scenes, layers, and each layer's CAST vector in their
/// construction order. For every SrTextCast it requests the primary font
/// first, then visits the selected CATR records in source order. Only the
/// exact case-sensitive string keys below call the same four-slot TextCast
/// font loader. Empty strings reach `sub_1088590` but do not issue a resource
/// request, so they are omitted here.
///
/// Independently constructed reference layers do not add requests here.
/// `srd_player_impl_load_project` resolves and constructs those copies before
/// this traversal, but `sub_AAE6C0` still walks only the original runtime scene
/// table. Every copied TextCast was rebuilt from a target LAYR already present
/// in that table and resolves its TextBox through the shared renderer resource
/// tree at draw time.
///
/// This is not an assertion that the process-global FontManager was empty
/// before this player loaded. Apply the returned requests to a registry that
/// already contains any host resources whose earlier lifetime is known.
pub fn collect_fennel_font_resource_requests(
    project: &Project,
) -> Result<Vec<FennelFontResourceRequest>, SrdDrawError> {
    let mut requests = Vec::new();
    for (scene_index, scene) in project.scenes.iter().enumerate() {
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            for node_index in 0..layer.nodes.len() {
                let Some(image) = layer.image_by_node.get(node_index).and_then(Option::as_ref)
                else {
                    continue;
                };
                if !image.creates_text_cast() {
                    continue;
                }
                let Some(text) = image.text.as_ref() else {
                    return Err(SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] creates an SrTextCast without a TEXT definition"
                    )));
                };
                let font_index = usize::try_from(text.font_index.unwrap_or(-1)).map_err(|_| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] has an invalid TextCast font index"
                    ))
                })?;
                let font = project.fonts.get(font_index).ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] TextCast font index {font_index} is outside PROJ"
                    ))
                })?;
                push_fennel_font_resource_request(
                    &mut requests,
                    scene_index,
                    layer_index,
                    node_index,
                    FennelTextFontRole::Primary,
                    &font.name,
                );

                let Some(attribute_list_index) = layer
                    .cast_attribute_list_by_node
                    .get(node_index)
                    .and_then(|index| *index)
                else {
                    continue;
                };
                let attribute_list = layer
                    .cast_attribute_lists
                    .get(attribute_list_index)
                    .ok_or_else(|| {
                        SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] CATR index {attribute_list_index} is outside the parsed list table"
                        ))
                    })?;
                for attribute in &attribute_list.attributes {
                    let Some(role) = (match attribute.name.as_slice() {
                        b"rubyFont" => Some(FennelTextFontRole::Ruby),
                        b"rfzOutlineFont" => Some(FennelTextFontRole::Outline),
                        b"rfzOutlineRubyFont" => Some(FennelTextFontRole::OutlineRuby),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let CastAttributeValue::String(name) = &attribute.value else {
                        return Err(SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] CATR {:?} uses an unported non-string font-resource value",
                            String::from_utf8_lossy(&attribute.name)
                        )));
                    };
                    push_fennel_font_resource_request(
                        &mut requests,
                        scene_index,
                        layer_index,
                        node_index,
                        role,
                        name,
                    );
                }
            }
        }
    }
    Ok(requests)
}
fn push_fennel_font_resource_request(
    requests: &mut Vec<FennelFontResourceRequest>,
    scene_index: usize,
    layer_index: usize,
    node_index: usize,
    role: FennelTextFontRole,
    name: &[u8],
) {
    if !name.is_empty() {
        requests.push(FennelFontResourceRequest {
            scene_index,
            layer_index,
            node_index,
            role,
            name: name.to_vec(),
        });
    }
}
pub fn assign_fennel_font_resource_requests(
    registry: &mut FennelFontSlotRegistry<Vec<u8>>,
    requests: impl IntoIterator<Item = FennelFontResourceRequest>,
) -> Vec<FennelFontResourceAssignment> {
    requests
        .into_iter()
        .map(|request| {
            let slot = registry.request(request.name.clone());
            FennelFontResourceAssignment { request, slot }
        })
        .collect()
}
