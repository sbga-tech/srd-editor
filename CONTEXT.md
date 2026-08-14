# Surfride Document Editing

The editor exposes the composition, drawable hierarchy, and animation choices encoded by a Surfride document without conflating those concepts in its UI.

## Language

**Project**:
A loaded Surfride document containing scenes and the shared resources they can reference.
_Avoid_: Workspace

**Scene**:
A named composition with its own dimensions, ordered layers, and animation presets.
_Avoid_: Asset folder

**Layer**:
An ordered visual stratum inside a scene. A layer owns a CAST hierarchy and receives one animation slot from each scene animation preset.
_Avoid_: Animation, preset

**CAST**:
A drawable or structural scene node. CASTs form parent/child hierarchies inside layers.
_Avoid_: Animation, preset

**Reference CAST**:
A CAST whose visual subtree is supplied by a referenced project layer.
_Avoid_: Animation reference

**Animation Preset**:
A scene-level playback choice whose positional slots enable layers and select one layer-local animation by name for each layer. It is not part of the scene/CAST asset hierarchy.
_Avoid_: Asset, CAST group

**Layer Animation**:
A named timeline owned by one layer. It coordinates motions on CASTs in that layer and may be selected by that layer's slot in an animation preset. Its name is local to the layer.
_Avoid_: Animation Preset, scene-wide animation

**Motion**:
The portion of a layer animation bound to one CAST. A motion groups the property tracks evaluated for that CAST.
_Avoid_: Layer animation, layer target

**Track**:
A time-varying property channel inside a motion, such as position, rotation, scale, visibility, or opacity.
_Avoid_: Motion, CAST

**Base Pose**:
The scene state obtained from serialized CAST transforms when no animation preset is active.
_Avoid_: Empty animation

**Simple Shader Profile**:
The normalized Ceylon SimpleShader feature values selected for one CAST draw after applying SRD runtime state and proven renderer invariants. It excludes numeric shader inputs and scene/pass features that an independent SRD cannot provide.
_Avoid_: Shader family, WGSL variant, compact key

**Host Render Context**:
Scene- or pass-owned state combined with a CAST draw but not derivable from an independent SRD, such as target dimensions, projection, fog, shadows, distance outputs, or special base environments.
_Avoid_: SRD shader flag
