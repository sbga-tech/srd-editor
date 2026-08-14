use super::*;
#[derive(Default)]
pub(super) struct PreviewDocumentResources {
    pub(super) asset_path: Option<PathBuf>,
    pub(super) texture_filenames: Vec<Vec<u8>>,
    pub(super) fennel_assignments: Vec<FennelFontResourceAssignment>,
    pub(super) texture_indices: BTreeSet<usize>,
    pub(super) textures: Option<SrdTextureSetHandle>,
    pub(super) fennel_resources_loaded: bool,
    pub(super) runtime_fonts: BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    pub(super) fennel_atlas_routes: BTreeMap<u32, FennelAtlasRoute>,
}

#[derive(Default)]
struct LoadedFennelResources {
    runtime_fonts: BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    atlas_routes: BTreeMap<u32, FennelAtlasRoute>,
}
pub(super) fn prepare_document_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document: &EditorDocument,
    assignments: &[FennelFontResourceAssignment],
) {
    if resources.asset_path.as_deref() == Some(document.asset_path())
        && resources.texture_filenames.len() == document.textures.textures.len()
        && resources
            .texture_filenames
            .iter()
            .zip(&document.textures.textures)
            .all(|(cached, texture)| cached == &texture.filename)
        && resources.fennel_assignments.as_slice() == assignments
    {
        return;
    }
    release_document_resources(backend, resources);
    resources.asset_path = Some(document.asset_path().to_path_buf());
    resources.texture_filenames = document
        .textures
        .textures
        .iter()
        .map(|texture| texture.filename.clone())
        .collect();
    resources.fennel_assignments.extend_from_slice(assignments);
}

pub(super) fn ensure_fennel_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document_path: &Path,
    assignments: &[FennelFontResourceAssignment],
) -> Result<(), String> {
    if resources.fennel_resources_loaded {
        return Ok(());
    }
    let loaded = if assignments.is_empty() {
        Ok(LoadedFennelResources::default())
    } else {
        let root = find_game_data_root(document_path).ok_or_else(|| {
            format!(
                "could not locate a data directory above {} for RFZ resources",
                document_path.display()
            )
        })?;
        load_fennel_resources(backend, &root, assignments)
    };
    match loaded {
        Ok(LoadedFennelResources {
            runtime_fonts,
            atlas_routes,
        }) => {
            resources.runtime_fonts = runtime_fonts;
            resources.fennel_atlas_routes = atlas_routes;
            resources.fennel_resources_loaded = true;
            Ok(())
        }
        Err(error) => {
            release_document_resources(backend, resources);
            Err(error)
        }
    }
}

pub(super) fn ensure_textures<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document: &EditorDocument,
    required: &BTreeSet<usize>,
) -> Result<(), String> {
    if required.is_subset(&resources.texture_indices) {
        return Ok(());
    }
    if let Some(textures) = resources.textures.take() {
        backend.release_srd_textures(textures);
    }
    resources.texture_indices.clear();
    let root = find_game_data_root(document.asset_path()).ok_or_else(|| {
        format!(
            "could not locate a data directory above {} for DDS resources",
            document.asset_path().display()
        )
    })?;
    let sources =
        SrdTextureSourceSet::load_required(&root, &document.textures, required.iter().copied())
            .map_err(|error| format!("failed to load SRD texture sources: {error}"))?;
    let textures = backend
        .upload_srd_textures(sources)
        .map_err(|error| format!("failed to upload SRD textures: {error}"))?;
    resources.textures = Some(textures);
    resources.texture_indices.clone_from(required);
    Ok(())
}

fn release_document_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
) {
    if let Some(textures) = resources.textures.take() {
        backend.release_srd_textures(textures);
    }
    let atlases = resources
        .fennel_atlas_routes
        .values()
        .map(|route| route.atlas)
        .collect::<BTreeSet<_>>();
    for atlas in atlases {
        backend.release_fennel_atlas(atlas);
    }
    *resources = PreviewDocumentResources::default();
}
fn load_fennel_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    game_data_root: &Path,
    assignments: &[FennelFontResourceAssignment],
) -> Result<LoadedFennelResources, String> {
    let mut uploaded_atlases = BTreeSet::new();
    let resources = (|| {
        let mut runtime_fonts = BTreeMap::new();
        let mut atlas_routes = BTreeMap::new();
        let mut next_texture_token = 1u32;
        for assignment in assignments {
            if !assignment.slot.first_request
                || !assignment.slot.registered
                || runtime_fonts.contains_key(assignment.request.name.as_slice())
                || !assignment
                    .request
                    .name
                    .get(assignment.request.name.len().saturating_sub(4)..)
                    .is_some_and(|suffix| suffix.eq_ignore_ascii_case(b".rfz"))
            {
                continue;
            }
            let name = std::str::from_utf8(&assignment.request.name)
                .map_err(|error| format!("RFZ font name is not UTF-8: {error}"))?;
            let path = game_data_root.join("A000/font").join(name);
            let parsed = RuhunaFont::from_rfz(
                &fs::read(&path)
                    .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
            )
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
            let sources = FennelAtlasSourceSet::from_font(&parsed)
                .map_err(|error| format!("failed to prepare {}: {error}", path.display()))?;
            let page_count = sources.page_count();
            let atlas = backend
                .upload_fennel_atlas(sources)
                .map_err(|error| format!("failed to upload {}: {error}", path.display()))?;
            uploaded_atlases.insert(atlas);
            let mut page_tokens = Vec::with_capacity(page_count);
            for page_index in 0..page_count {
                let texture_token = next_texture_token;
                next_texture_token = next_texture_token.checked_add(1).ok_or_else(|| {
                    "GPU preview Fennel texture-token space exhausted while loading atlases"
                        .to_string()
                })?;
                atlas_routes.insert(texture_token, FennelAtlasRoute { atlas, page_index });
                page_tokens.push(texture_token);
            }
            let owner_token = u32::try_from(assignment.slot.resource_handle).map_err(|_| {
                format!(
                    "Fennel resource handle {} does not fit the runtime glyph token",
                    assignment.slot.resource_handle
                )
            })?;
            let runtime = parsed
                .build_runtime_font(owner_token, |page| {
                    page_tokens.get(usize::from(page)).copied().unwrap_or(0)
                })
                .map_err(|error| format!("failed to build runtime {}: {error}", path.display()))?;
            runtime_fonts.insert(assignment.request.name.clone(), runtime);
        }
        Ok(LoadedFennelResources {
            runtime_fonts,
            atlas_routes,
        })
    })();
    if resources.is_err() {
        for atlas in uploaded_atlases {
            backend.release_fennel_atlas(atlas);
        }
    }
    resources
}
fn find_game_data_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("data"))
        })
        .map(Path::to_path_buf)
}
