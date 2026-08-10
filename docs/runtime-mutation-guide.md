# Live `SrPlayer` runtime mutation guide

This guide describes how to change the **live, player-owned Surfride runtime** in
`chusanApp_MATE_2.50.exe`. It covers animation selection and playback, retained
visibility and translation, number/text content, call timing, and reload-safe
mod structure. It does not describe editing an SRD on disk.

## Exact scope

The addresses and layouts below apply only to this executable:

```text
file:      chusanApp_MATE_2.50.exe
SHA-256:   b987a33687cb98b79f4a6dca21924dfd9d10c6b1d3bc5a9985570be40d145763
format:    PE32 / x86
image base: 0x00400000
```

The PE has stripped relocations. Addresses in this document are image virtual
addresses, not source-level symbols or stable APIs. A mod must identify the
exact executable before binding any address and fail closed on a different
build. The recovered functions use the 32-bit MSVC `__thiscall` convention:
`ECX` is the receiver and the callee pops stack arguments.

Names such as `setVisible` and `startAnimation` below are descriptive aliases.
The stripped binary does not preserve the original source declarations.

## Mutate the runtime tree, not the parsed resource

The relevant ownership chain for the audited Common Background object is:

```text
ObjectManager-owned CommonBackGroundObject wrapper
  +0x08 -> CommonBackGroundObject::Impl
              +0x68 embedded projView::SrPlayer
                        +0xE8 -> surfride::SrPlayer::Impl
                                      +0x294 runtime SrScene* vector
                                                -> SrLayer
                                                     -> SrCast
                                                     -> SrAnimation
                                                -> SrAnimationSet
```

`SrResource` retains the parsed `SrProject`. It is source data. Finalization
builds separately allocated `SrScene`, `SrLayer`, `SrCast`, `SrAnimation`, and
`SrAnimationSet` instances under `surfride::SrPlayer::Impl`. Live commands
change those player-owned instances.

Consequences:

- Patching a parsed `SrProject`, `SCN`, `LAYR`, or `ANIM` record does not update
  an already-built player tree.
- Adding/removing scenes, layers, casts, animations, or vector entries is a
  structural change. Release and rebuild the player, or create a separate
  mod-owned render object; do not splice the native vectors.
- Each `SrRefCast` owns an independent copied runtime layer. Changing the
  original runtime layer does not implicitly change its reference copies.
- The outer `projView::SrPlayer` and its `0x2E8`-byte Impl normally survive a
  content release. The inner scene/layer/cast/animation allocations do not.

See [`runtime-architecture.md`](runtime-architecture.md) for construction and
ownership details.

## Canonical command path

Normal game code does not cache and patch inner pointers. It calls a
nonvirtual `projView::SrPlayer` facade:

```text
owner GameObject / local state callback
  -> projView::SrPlayer facade
       -> embedded weak SrCtrl                    scene/set/generic visibility
       -> temporary stack SrLayerCtrl             layer/cast/animation lookup
            -> current player-owned runtime object
```

`SrCtrl` resolves a weak player target. `SrLayerCtrl` is constructed, bound,
used, and destroyed inside each facade call. Both routes decode and
bounds-check packed identities before indexing runtime vectors. Reusing this
path gives a mod the native missing-target behavior and, critically, avoids
retaining an inner pointer across release/reload.

`0x00BA6670` tests whether the embedded `SrCtrl` currently has a live target.
For Common Background, also require wrapper status `2` (`wrapper+0x04`) before
mutating. A status of `1` only means the GameObject is loading; the runtime tree
is not ready merely because a load request was submitted.

## Packed retained identities

Most commands consume one-based indices packed into a 32-bit identity:

| Bits | Layer/cast/individual-animation commands | Scene-animation-set commands |
| ---: | --- | --- |
| `24..29` | scene ordinal | scene ordinal |
| `16..23` | layer ordinal | unused by the recovered set resolver |
| `10..15` | layer-local animation ordinal | unused by the recovered set resolver |
| `0..9` | cast ordinal | scene-local animation-set ordinal |

Each ordinal is decremented and bounds-checked during lookup. Zero means “no
ordinal” for that field; do not pass zero as an intended first element.

```cpp
using SrId = std::uint32_t;

constexpr SrId layerId(unsigned scene, unsigned layer) {
    // Preconditions: scene 1..63, layer 1..255.
    return (scene << 24) | (layer << 16);
}

constexpr SrId castId(unsigned scene, unsigned layer, unsigned cast) {
    // Preconditions: cast 1..1023.
    return layerId(scene, layer) | cast;
}

constexpr SrId animationId(unsigned scene, unsigned layer, unsigned animation) {
    // Preconditions: animation 1..63.
    return layerId(scene, layer) | (animation << 10);
}

constexpr SrId animationSetId(unsigned scene, unsigned set) {
    // 0x40000000 is present in the generated set identities observed here.
    // The recovered resolver itself consumes only scene bits and low 10 bits.
    return 0x40000000u | (scene << 24) | set;
}
```

Examples:

```text
0x01010000 = scene 1 / layer 1
0x01010800 = scene 1 / layer 1 / animation 2
0x01010008 = scene 1 / layer 1 / cast 8
0x41000001 = scene 1 / animation set 1
0x06014C00 = scene 6 / layer 1 / animation 19
```

The same low ten bits mean a cast or an animation set depending on the command
family. Prefer identities generated from the exact parsed project. Preserve
observed category bits rather than normalizing IDs from another operation.

## Recovered mutation surface

All arguments after `player` are stack arguments to a `__thiscall` receiver.
Type-specific calls require a type-correct cast ID from the exact SRD.

| Image VA | Descriptive signature | Direct effect |
| ---: | --- | --- |
| `0x00BA6670` | `bool targetValid(player)` | Tests the weak `SrCtrl` target. |
| `0x00BA7AE0` | `void setVisible(player, objectId, bool)` | Through `SrCtrl +0x34`, writes scene `+0x75`, layer `+0x168`, or cast `+0x88`, according to ID shape. |
| `0x00BA7A40` | `void setCastVisible(player, layerId, castId, bool)` | Resolves one cast through a temporary `SrLayerCtrl`; writes cast `+0x88`. |
| `0x00BA6D40` | `void playAnimationSet(player, setId)` | Through `SrCtrl +0x48`, selects the set, installs its scene state, applies its initial frame, and writes its positional layer gates. |
| `0x00BA6E40` | `void resetAnimationSet(player, setId)` | Resolves the set and resets/activates its linked animations. It does not select a different set. |
| `0x00BA67D0` | `bool animationSetComplete(player, setId)` | Tests set completion; a disabled retained-player gate is treated as complete. |
| `0x00BA6C80` | `void startAnimation(player, layerId, animationId, float frame, float rate)` | Resolves, resets, writes raw frame `+0x1C`, writes primary factor `+0x24`, then activates the animation. |
| `0x00BA6690` | `bool animationComplete(player, layerId, animationId)` | Tests completion with native missing/disabled/loop handling. |
| `0x00BA7720` | `void setAnimationRate(player, layerId, animationId, float rate)` | Writes the primary playback factor at animation `+0x24`; does not reset or activate. |
| `0x00BA7C40` | `void setAnimationLoop(player, layerId, animationId, bool)` | Sets/clears animation flag bit `1`. |
| `0x00BA7900` | `void setAnimationFrame(player, layerId, animationId, uint32_t frame)` | Clamps to the integerized duration and writes raw frame `+0x1C`; does not reset or activate. |
| `0x00BA8720` | `void setCastTranslation(player, layerId, castId, const Vec3*)` | Copies 12 bytes to the cast local translation at `+0x5C..+0x67`. |
| `0x00BA7D80` | `void setNumber(player, layerId, castId, int32_t whole, double fraction, bool interpolate)` | Dispatches cast virtual `+0xB0`; on `SrNumberCast` this reaches `0x00AE09B0`. |
| `0x00BA8460` | `void setTextMappedB(player, layerId, castId, const char*, bool prependGlobal)` | Applies the second engine replacement table, compares content and length through TextCast channel B, then invokes the rebuild-triggering setter if changed. |
| `0x00BA8240` | `void setTextMappedA(player, layerId, castId, const char*, bool prependGlobal)` | Parallel first replacement-table route through TextCast channel A. |
| `0x00BA8110` | `void setTextLiteral(player, layerId, castId, const char*)` | Builds an engine string and calls TextCast channel-B setter, but only when a current-length prefix comparison differs. |

Useful readback entries are `0x00BA65E0` (generic visibility), `0x00BA64F0`
(cast visibility), `0x00BA5EC0` (animation raw frame), `0x00BA5F80`
(animation duration), and `0x00BA5D50` (primary playback factor).

### Animation-set selection is a compound mutation

Selecting an `SrAnimationSet` is not merely assigning a current-set pointer.
The native path:

1. revokes the previous set state;
2. writes the new set to `SrScene+0x70`;
3. initializes the scene/set timeline from parsed `ANMS` values;
4. writes each positional `SANM` enable gate to its paired runtime layer
   `+0x168`;
5. applies the common initial raw frame to every resolved layer animation.

Call `playAnimationSet` before any intentional per-layer visibility override in
the same batch. A later set selection can overwrite that override.

### Individual animation rate and flags

A runtime `SrAnimation` contains:

```text
+0x1C raw current frame
+0x20 duration
+0x24 primary playback factor
+0x28 secondary playback factor
+0x2C tertiary playback factor
+0x30 flags
```

The recovered active update at `0x0071F020` advances:

```text
frame += factor_24 * factor_28 * factor_2C
```

Default factors are `1.0`. `setAnimationRate` changes only `+0x24`, so it is a
multiplier, not a seconds/FPS contract. With all factors at one, one update adds
one raw animation frame. The exact scheduler-to-real-time conversion is not
recovered; do not invent a fixed FPS.

Relevant flags are:

- bit `3` (`0x08`): active update gate;
- bit `2` (`0x04`): non-loop completion boundary reached;
- bit `1` (`0x02`): looping;
- bit `4` (`0x10`): wrap occurred during the current update.

Positive rates advance toward duration. Negative rates run toward zero. At a
boundary, a looping animation wraps and sets bit `4`; a non-looping animation
clamps and sets bit `2`. `startAnimation` resets completion before activating.
Merely changing `+0x24` on an already completed animation does not restart it.

`animationComplete` deliberately treats a missing target, a disabled retained
player, and a looping animation as complete in the recovered path, preventing
owner state machines from waiting forever. Do not use it as an “is currently
moving” predicate.

`0x00BA7550` refreshes controller-owned runtime entries and resets their
`+0x24` factor to `1.0`. Reapply a persistent rate override after owner refresh
and after content reload.

### Visibility and transform precedence

Three gates are distinct:

- scene visibility at `SrScene+0x75`;
- layer enable at `SrLayer+0x168`;
- cast local visibility at `SrCast+0x88`.

A cast is renderable only when all relevant scene/layer/cast and inherited
gates survive later evaluation. Making a cast visible cannot override a layer
disabled by the selected animation set.

Animation channels can overwrite direct retained fields:

- channels `0..2` write cast translation;
- channels `3..5` write rotation;
- channels `6..8` write scale;
- channel `10` writes cast local visibility;
- color/alpha and type-specific channels write other retained cast state.

`setCastTranslation` is therefore stable only when no active animation writes
those translation channels. Graph traversal may evaluate the animation before
world-matrix composition in the same producer frame and replace a direct
translation write. Prefer controlling the responsible animation. An advanced
post-animation/pre-composition hook is possible but is more fragile than the
facade and is not a general transaction point.

Never patch derived cast visibility at `+0xC4` or world matrices at `+0x8C`;
they are recomputed from local retained state during layer/cast evaluation.

### Number mutation

`setNumber` resolves the cast and dispatches virtual slot `+0xB0`. For an
`SrNumberCast`, `0x00AE09B0` stores the integer and fractional parts, computes
their double-precision sum, and updates transition/history state. The Boolean
interpolation request is honored only when the authored CNUM mode is in the
recovered range `5..8`; other modes force it off. Glyph geometry is rebuilt by
the later number-render path.

This facade does not dynamically prove that the ID names an `SrNumberCast`.
Calling it on another cast type dispatches that type's `+0xB0` method. Validate
NODE type `4` from the exact parsed project before calling.

### Text mutation

An `SrTextCast` stores its runtime string from `+0x1F8`. Its setter invokes
virtual `+0xBC`, triggering the text-layout/rebuild path; the facade call still
does not draw synchronously.

Use `0x00BA8460` when the engine's channel-B replacement behavior is desired.
It accepts a C string, falls back to that input when no replacement record
matches, compares both bytes and length, and copies into engine-owned storage.
Pass `false` for `prependGlobal` unless the owner path explicitly requires the
global prefix. `0x00BA8240` is the parallel channel-A conversion route.

`0x00BA8110` is closer to a literal channel-B write, but its change test is
`strncmp(new, current, current.length())`: an empty current string or a new
string that extends the current string with the same prefix can be incorrectly
classified as unchanged. Do not use it as a general arbitrary-text setter
without accounting for that exact behavior.

The shipped TextCast path consumes UTF-8 byte strings and Fennel/RFZ control
tokens. Type correctness still matters: use a cast proven to be the Text
factory case (NODE type `1`, CIMG with `TEXT`, and CIMG flag `0x100`).

## Execution order and thread rule

A facade call mutates producer-side retained state synchronously. It does not
issue D3D9 work and does not guarantee that the visible consumer frame changes
before the call returns.

```text
owner/view callback mutates player state
  -> optional inherited scalar update (player virtual +0x10)
  -> GraphManagerDraw producer traversal (GraphNode virtual +0x14)
  -> retained state is sealed by the Ceylon SwapSync handoff
  -> Ceylon DrawThread consumes the sealed frame and submits D3D9 work
```

The separate `GraphManager` worker invokes GraphNode slot `+0x18`; that slot is
a no-op for this player. It is not the meaningful player update.

Rules for an injected mod:

1. Accept commands from UI/network/other threads into a **mod-owned queue**.
2. Drain that queue from a proven owner/producer callback, after the owner's
   own state changes and before the later `GraphManagerDraw` traversal.
3. Apply all related facade calls consecutively in that callback.
4. Never invoke these facades from Ceylon's DrawThread, AIR's resource-loading
   worker, or an arbitrary thread that merely happens to be named “main”.
5. If a direct field is animation-driven, changing call order alone is not a
   lock: later animation evaluation still wins.

There is no recovered generic `SrPlayer` transaction or mutex that makes an
arbitrary-thread batch safe. Lock the mod command queue, not native Surfride
objects; native mutation remains producer-thread confined.

## Practical x86 facade binding

The following sketch is intentionally limited to POD and C-string arguments.
It avoids passing a foreign `std::string` across the module ABI.

```cpp
#include <cstddef>
#include <cstdint>
#include <windows.h>

using SrId = std::uint32_t;

struct Vec3 {
    float x;
    float y;
    float z;
};
static_assert(sizeof(Vec3) == 12);

using TargetValidFn       = bool (__thiscall *)(void* player);
using SetVisibleFn        = void (__thiscall *)(void* player, SrId object, bool);
using PlaySetFn           = void (__thiscall *)(void* player, SrId set);
using StartAnimationFn    = void (__thiscall *)(void* player, SrId layer,
                                                SrId animation, float frame,
                                                float rate);
using SetAnimationRateFn  = void (__thiscall *)(void* player, SrId layer,
                                                SrId animation, float rate);
using SetAnimationLoopFn  = void (__thiscall *)(void* player, SrId layer,
                                                SrId animation, bool loop);
using SetTranslationFn    = void (__thiscall *)(void* player, SrId layer,
                                                SrId cast, const Vec3* value);
using SetNumberFn         = void (__thiscall *)(void* player, SrId layer,
                                                SrId cast, std::int32_t whole,
                                                double fraction,
                                                bool interpolate);
using SetTextMappedFn     = void (__thiscall *)(void* player, SrId layer,
                                                SrId cast, const char* text,
                                                bool prependGlobal);

constexpr std::uintptr_t kPreferredImageBase = 0x00400000;

template <class Fn>
Fn bindImageVa(HMODULE executable, std::uintptr_t imageVa) {
    const auto loadedBase = reinterpret_cast<std::uintptr_t>(executable);
    return reinterpret_cast<Fn>(loadedBase + imageVa - kPreferredImageBase);
}

struct SrPlayerApi {
    TargetValidFn targetValid;
    SetVisibleFn setVisible;
    PlaySetFn playSet;
    StartAnimationFn startAnimation;
    SetAnimationRateFn setAnimationRate;
    SetAnimationLoopFn setAnimationLoop;
    SetTranslationFn setTranslation;
    SetNumberFn setNumber;
    SetTextMappedFn setTextMappedB;
};

SrPlayerApi bindSrPlayerApi(HMODULE executable) {
    return {
        bindImageVa<TargetValidFn>(executable,      0x00BA6670),
        bindImageVa<SetVisibleFn>(executable,       0x00BA7AE0),
        bindImageVa<PlaySetFn>(executable,          0x00BA6D40),
        bindImageVa<StartAnimationFn>(executable,   0x00BA6C80),
        bindImageVa<SetAnimationRateFn>(executable, 0x00BA7720),
        bindImageVa<SetAnimationLoopFn>(executable, 0x00BA7C40),
        bindImageVa<SetTranslationFn>(executable,   0x00BA8720),
        bindImageVa<SetNumberFn>(executable,        0x00BA7D80),
        bindImageVa<SetTextMappedFn>(executable,    0x00BA8460),
    };
}
```

Compile the injected caller as x86 with an ABI that implements MSVC
`__thiscall`. Do not reinterpret these entries as x64 functions, ordinary
`__cdecl` functions, or Rust host functions.

### Locating the audited Common Background player

Only for `CommonBackGroundObject` in this exact build:

```cpp
void* commonBackgroundPlayer(std::byte* wrapper) {
    if (!wrapper)
        return nullptr;

    const auto status = *reinterpret_cast<const std::int32_t*>(wrapper + 0x04);
    if (status != 2)
        return nullptr;

    auto* impl = *reinterpret_cast<std::byte**>(wrapper + 0x08);
    return impl ? static_cast<void*>(impl + 0x68) : nullptr;
}
```

The wrapper is normally obtained as a non-owning handle from `ObjectManager`,
or the player is reached directly from a hook in the concrete owner Impl. Do
not apply this offset recipe to an arbitrary GameObject: embedding an
`SrPlayer` is concrete-object composition, not a `GameObjectBase` contract.

### One producer-side batch

Call this only at the producer hook described above. The IDs for number and
text must already be validated against the exact parsed SRD.

```cpp
struct MutationIds {
    SrId set;          // zero means “do not change the selected set” here
    SrId layer;
    SrId animation;
    SrId visibleCast;
    SrId numberCast;   // proven NODE type 4
    SrId textCast;     // proven SrTextCast factory case
};

bool applyBatch(const SrPlayerApi& api, void* player,
                const MutationIds& id) {
    if (!player || !api.targetValid(player))
        return false;

    // Compound set initialization first: it writes layer gates and frames.
    if (id.set != 0)
        api.playSet(player, id.set);

    // Intentional local gate overrides follow set selection.
    api.setVisible(player, id.layer, true);
    api.setVisible(player, id.visibleCast, true);

    // Configure loop before starting at frame zero with half-rate playback.
    api.setAnimationLoop(player, id.layer, id.animation, true);
    api.startAnimation(player, id.layer, id.animation, 0.0f, 0.5f);

    // Type-specific retained content. Setters copy the supplied payloads.
    api.setNumber(player, id.layer, id.numberCast, 123, 0.5, false);
    api.setTextMappedB(player, id.layer, id.textCast, "MOD TEXT", false);

    return true;
}
```

Do not combine set selection and a separate start of one of that set's linked
animations unless desynchronizing that layer from the set is intentional.

## Release/reload-safe mod state

Content release enters `0x00BA8A50`, removes renderer-side cast bindings,
deletes the player-owned runtime scenes/layers/casts/animations, clears
resource references, and nulls the project link. Finalization at `0x00BA6490`
builds a fresh runtime tree and rebinds `SrCtrl`.

Use this lifecycle policy:

```text
on release entry:
  increment player generation
  invalidate every cached runtime pointer/controller/reference
  discard queued one-shot commands tagged with the old generation

on finalize return, after targetValid == true:
  increment player generation
  resolve IDs against the newly loaded project
  reapply persistent desired state through facade calls
```

Queue values and packed IDs, not `SrScene*`, `SrLayer*`, `SrCast*`,
`SrAnimation*`, `SrCtrl*`, or `SrLayerCtrl*`. Even when the same heap address is
reused after reload, it represents a new lifetime. If a different SRD revision
changes ordering, old numeric IDs are not semantically stable; key persistent
configuration by project identity and names, then regenerate IDs.

At final application shutdown, the ObjectManager destroys the wrapper and both
implementation objects. No player pointer remains valid beyond that point.

## Failure and overwrite cases

| Symptom | Likely cause | Correct response |
| --- | --- | --- |
| Facade call has no effect while loading | Runtime tree/SrCtrl target is not finalized | Wait for wrapper status `2` and `targetValid`. |
| Cast is still hidden | Scene or layer gate is false; active channel `10` rewrote cast visibility | Select/inspect the intended set and control the competing animation. |
| Translation snaps back | Active animation channels `0..2` overwrite local translation | Change the animation/frame/rate or use a proven post-animation hook. |
| Rate returns to `1.0` | Owner called refresh `0x00BA7550`, or content reloaded | Reapply after refresh/finalize. |
| Completion is immediately true | Target is missing, player is disabled, or animation loop bit is set | Use readback/active state, not completion as a motion predicate. |
| Number call corrupts unrelated state | Cast ID does not name an `SrNumberCast` | Validate NODE type `4`; stop dispatching a subtype virtual blindly. |
| Text extension is ignored through `0xBA8110` | Current-length prefix comparison reports equality | Use mapped channel `0xBA8460` or account for the literal route's exact comparison. |
| Mod works until unload/reload | Cached inner pointer or stale numeric ordering | Use generation invalidation and resolve/reapply by IDs. |
| Producer memory looks right but frame is unchanged | No later traversal/handoff occurred, or a later owner write won | Observe after `GraphManagerDraw` and SwapSync; fix producer hook order. |

## Do not do these

- Do not mutate the parsed resource and expect a live player to rebuild itself.
- Do not append to native scene/layer/cast/animation vectors.
- Do not cache inner runtime pointers across release, reload, or shutdown.
- Do not call producer facades from Ceylon's DrawThread or AIR's worker.
- Do not patch D3D9 state as a substitute for changing retained Surfride state.
- Do not patch derived world matrices/visibility that evaluation recomputes.
- Do not assume every GameObject embeds a player at `Impl+0x68`.
- Do not use a number/text facade on an unverified cast subtype.
- Do not treat animation-set IDs and cast IDs as the same low-bit namespace.

## Evidence map

- [`runtime-architecture.md`](runtime-architecture.md): allocations, fields,
  ownership, load/evaluate/render/release lifecycle.
- [`evidence/scene-animation-sets.md`](evidence/scene-animation-sets.md):
  set selection, positional layer gates, initial-frame application, set-ID
  decoding.
- [`evidence/animation-records.md`](evidence/animation-records.md): runtime
  animation layout, frame application, flags, interpolation.
- [`evidence/scene-transform.md`](evidence/scene-transform.md): local retained
  transforms, animation channels, world composition.
- [`evidence/cnum-number-cast.md`](evidence/cnum-number-cast.md): number value,
  formatting, history, and glyph rebuild behavior.
- [`evidence/text-font-records.md`](evidence/text-font-records.md): TextCast
  string state and rebuild path.
- [`../../docs/PROJVIEW_SRPLAYER.md`](../../docs/PROJVIEW_SRPLAYER.md): complete
  facade/control/traversal analysis.
- [`../../docs/SRPLAYER_METHOD_CATALOG.md`](../../docs/SRPLAYER_METHOD_CATALOG.md):
  recovered receiver method surface.
- [`../../docs/COMMON_BACKGROUND_OBJECT.md`](../../docs/COMMON_BACKGROUND_OBJECT.md):
  audited concrete owner offsets, state machine, and release/reload lifecycle.
