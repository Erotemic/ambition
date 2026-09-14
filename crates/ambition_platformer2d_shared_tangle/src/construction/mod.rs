//! Content-free construction planning and explicit spawn provenance.
//!
//! Construction decisions are pure values produced before world mutation;
//! execution consumes those plans instead of rediscovering authority mid-build.
//! `SimId` identifies an entity, while provenance records who requested it and
//! how it can be reconstructed without parsing identity strings. A
//! [`ConstructionDomain`] supplies domain parameters/services and one exhaustive
//! dispatch from planned row to recipe/populator.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::{Commands, Component, Entity, World};

use crate::sim_id::SimId;

mod registry;
mod schema_catalog;
#[cfg(test)]
mod tests;

pub use schema_catalog::{ConstructionSchemaCatalog, ConstructionSchemaCatalogError};
pub use registry::{
    ConstructionRegistrationError, ConstructionRegistry, RelationCheck, RelationFn, RelationKind,
    RelationOps, RelationVerifyFn,
};

/// A stable internal identity for a construction recipe.
///
/// Internal on purpose: it names a *way of building something*, which is an
/// engine-side decision, not authored content. Authored data selects a recipe;
/// it never spells one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecipeId(String);

impl RecipeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RecipeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where an entity came from, and what would make it again.
///
/// A `Component`, because reconstruction reads it. The three variants are the
/// three origin categories a world can have: something an author declared,
/// something a provider staged into a room, and something the running
/// simulation minted.
///
/// The recipe is deliberately not repeated here. The doc's sketch carried a
/// `RecipeId` inside two of the variants, but the planned row already names the
/// recipe; storing it twice creates a state where the two can disagree and
/// nothing says which wins.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpawnOrigin {
    /// An authored declaration: `source` is the authored artifact it was
    /// declared in (a room id), `instance` the stable declaration within it (an
    /// LDtk iid, a placement id).
    Authored { source: String, instance: String },
    /// A provider staged this occupant into a room during construction. It is
    /// not in the authored artifact, so its durable identity is the provider
    /// plus the key the provider staged it under.
    ProviderStaged {
        provider: String,
        room: String,
        instance: String,
    },
    /// The running simulation minted this: a projectile, a summoned minion, a dropped item.
    ///
    /// `parent` is not optional. A dynamic entity states which spawner it
    /// descends from or it is unreconstructable, so "dynamic, parent unknown"
    /// is not a state worth being able to spell. A spawn site that cannot name
    /// its spawner's identity must refuse to spawn rather than mint a
    /// provenance that says nothing.
    Dynamic { parent: SimId, sequence: u64 },
}

impl SpawnOrigin {
    /// Stable construction-schema identity for the origin category. Unlike
    /// `Debug`, this is a compatibility contract: it is written into plan dumps
    /// and snapshot blobs.
    pub const fn canonical_kind(&self) -> &'static str {
        match self {
            Self::Authored { .. } => "authored",
            Self::ProviderStaged { .. } => "provider-staged",
            Self::Dynamic { .. } => "dynamic",
        }
    }

    /// The spawner this entity descends from, if any. This is the accessor that
    /// replaces parsing a spawned id's `/`-delimited parent prefix.
    pub const fn parent(&self) -> Option<&SimId> {
        match self {
            Self::Dynamic { parent, .. } => Some(parent),
            Self::Authored { .. } | Self::ProviderStaged { .. } => None,
        }
    }

    /// Byte-stable single-line rendering, tab-delimited like every other
    /// canonical dump in the tree. `-` is the explicit absent-value placeholder.
    pub fn canonical_summary(&self) -> String {
        match self {
            Self::Authored { source, instance } => {
                format!("authored\t{source}\t{instance}")
            }
            Self::ProviderStaged {
                provider,
                room,
                instance,
            } => format!("provider-staged\t{provider}\t{room}\t{instance}"),
            Self::Dynamic { parent, sequence } => format!("dynamic\t{parent}\t{sequence}"),
        }
    }
}

/// The domain a construction plan is written against.
///
/// Core plans, validates, orders, and dumps; the domain supplies the payload and
/// the frozen services its recipes read. Keeping this an associated-type pair
/// rather than type erasure means a recipe never downcasts and a plan cannot be
/// executed against the wrong world.
pub trait ConstructionDomain: Send + Sync + 'static + Sized {
    /// What one planned row carries into its recipe.
    type Parameters: Clone + Send + Sync + 'static;
    /// What one declared RELATION is — its kind and everything the pairing
    /// carries, as ONE value.
    ///
    /// Some relationships are pure adjacency — a grudge is fully described by
    /// who resents whom. Others are not: a limb's slot and its host-local idle
    /// anchor are both stated relative to the host, so they are facts about the
    /// pairing rather than about either body. Storing them on the limb would put
    /// host-relative data on an entity that does not know its host until the
    /// relation is wired, which is the same shape of mistake as a `parent` field
    /// beside a `SpawnOrigin::Dynamic`.
    ///
    /// The kind is now DERIVED from this value by [`ConstructionDomain::dispatch_relation`], so
    /// the mismatch is not a state that can be written down.
    type Relation: Clone + Send + Sync + 'static;
    /// Frozen services recipes read at execution time. Whatever a domain puts
    /// here is captured before the plan commits, so execution has no fallible
    /// lookup left.
    type Services;

    /// Resolve what builds this row: its recipe identity and its executor,
    /// from one exhaustive match.
    ///
    /// Returning both together is the point. This started as two methods — one
    /// deriving a `RecipeId`, one performing construction — and two matches over
    /// the same enum can drift while still compiling: a variant could be
    /// labelled with one recipe's identity and built by another's code, and
    /// nothing would object. One arm now names both, so the label and the
    /// behaviour are chosen in the same place or not at all.
    ///
    /// Exhaustive over `Parameters`, so a new variant with no arm is a compile
    /// error rather than a runtime surprise. And nothing here can fail: every
    /// lookup that could miss resolved in the request builder.
    fn dispatch(parameters: &Self::Parameters) -> RecipeDispatch<Self>;

    /// Resolve what a declared relation IS: its stable kind, its wiring, and
    /// its postcondition check, from one exhaustive match.
    ///
    /// The relation counterpart of [`Self::dispatch`], and it exists for the
    /// same reason plus one more. The same one: a kind chosen in one place and
    /// behaviour chosen in another can drift while still compiling. The extra
    /// one: this is also what makes relation wiring engine-owned. Ops are
    /// resolved here rather than looked up in the registry, so there is no table
    /// an outside plugin can register executable behaviour into — and therefore
    /// no insertion-order race deciding which implementation of a kind runs.
    ///
    /// Exhaustive over [`Self::Relation`], so a new relation variant with no arm
    /// is a compile error. Infallible, like `dispatch`: every fallible lookup
    /// resolved in the request builder.
    fn dispatch_relation(relation: &Self::Relation) -> RelationDispatch<Self>;

    /// Byte-stable one-line rendering of a row's parameters for the plan dump.
    /// Must not include tabs or newlines.
    fn canonical_summary(parameters: &Self::Parameters) -> String;

    /// Byte-stable rendering of a relation's carried facts for the plan dump.
    /// Must not include tabs or newlines.
    ///
    /// In the dump because it is content: two plans whose limbs fill different
    /// slots describe different worlds, and a dump that rendered them
    /// identically would call them the same plan. The KIND is dumped separately
    /// from [`RelationDispatch::kind`], so this renders only what the kind does
    /// not already say.
    fn canonical_relation_summary(relation: &Self::Relation) -> String;
}

/// What one exhaustive relation dispatch yields: the relation's stable identity
/// and the two frozen halves of its behaviour.
pub struct RelationDispatch<D: ConstructionDomain> {
    /// Stable identity for the dump, the registry check, and the fingerprint.
    pub kind: RelationKind,
    pub ops: RelationOps<D>,
}

/// What one exhaustive dispatch decision yields: the row's recipe identity and
/// the function that populates its root.
pub struct RecipeDispatch<D: ConstructionDomain> {
    /// Stable identity for the dump, the registry check, and the fingerprint.
    pub recipe: RecipeId,
    /// Populates the root the executor allocated. The root already exists and
    /// already carries its `SimId` and `SpawnOrigin`; this inserts onto it.
    pub construct: ConstructFn<D>,
}

/// The ONLY surface a recipe gets: its own root, and the two ways to put
/// components on it.
///
/// ⭐⭐ **A10: THIS IS WHY A CANDIDATE IS COMPLETELY ISOLATED RATHER THAN MOSTLY
/// ISOLATED.** `commit_inactive` hides every root the EXECUTOR minted. A recipe
/// that minted one of its own would leave it visible while the rest of the
/// candidate is hidden — a half-visible scene, and exactly what A10's *"complete
/// visibility proof"* is about. **There is no `Commands` in this type, so a
/// recipe cannot spawn anything at all.** The gap is not guarded; it is
/// unexpressible.
///
/// ⚠ MEASURED BEFORE IT WAS NARROWED, across all four files that implement a
/// recipe (`actor_monolith`, `gravity`, `portal2d`, and the toy domain): the
/// entire production surface is `entity(root).insert(..)` and
/// `insert_room_in_session(session, root, ..)`. Both are root-bound, both are
/// here, and **not one production recipe spawns.** So this narrows the type to
/// what recipes already do rather than to what I would like them to do.
///
/// ⇒ `ConstructionExecCtx` still exists and still carries raw `Commands` —
/// RELATION wiring legitimately needs to touch two arbitrary entities and to
/// queue a deferred edit. The two roads are different jobs and now have
/// different surfaces.
pub struct ConstructionRootCtx<'w, 's, 'a, D: ConstructionDomain> {
    root: Entity,
    commands: &'a mut Commands<'w, 's>,
    /// What the plan describes — content generation and room.
    pub scope: &'a ConstructionScope,
    /// Gameplay-session ownership, captured when this commit was requested.
    pub session: crate::lifecycle::SessionSpawnScope,
    pub services: &'a D::Services,
}

impl<'w, 's, 'a, D: ConstructionDomain> ConstructionRootCtx<'w, 's, 'a, D> {
    /// This row's authoritative entity. Readable because a recipe legitimately
    /// needs to name it inside its own components; it is not a licence to reach
    /// for `Commands`, which this type does not have.
    pub fn root(&self) -> Entity {
        self.root
    }

    /// Put components on this row's root.
    pub fn insert(&mut self, bundle: impl bevy::prelude::Bundle) -> &mut Self {
        self.commands.entity(self.root).insert(bundle);
        self
    }

    /// Hand this row's root to a helper that populates it.
    ///
    /// ⭐⭐ **THIS REPLACED `commands_escape`, AND THE DIFFERENCE IS THE WHOLE
    /// PACKET.** The escape returned `&mut Commands`, so a recipe that reached
    /// for it could `spawn()` an authoritative entity the executor never
    /// allocated and `commit_inactive` therefore never hides — a candidate scene
    /// visible in half. [`RootScope`] has no `spawn` of any kind, so the same
    /// nine monolith recipes now delegate through a surface that **cannot**
    /// mint. The hole is closed by TYPE rather than by the
    /// `engine.construction-recipes-do-not-spawn` waiver that used to name its
    /// nine rows.
    ///
    /// ⚠ The scope can still write only to THIS root. A recipe that needs a
    /// deliberate child entity has never had a way to make one here and still
    /// does not; that is relation work, and `ConstructionExecCtx` is where it
    /// lives.
    pub fn root_scope(&mut self) -> crate::construction::RootScope<'w, 's, '_> {
        crate::construction::RootScope::new(self.commands, self.session, self.root)
    }

    /// ⛔⛤ **TEST-ONLY RAW `Commands`, AND ITS ABSENCE IN PRODUCTION IS THE
    /// POINT.**
    ///
    /// The boundary roster verification has to be poisoned by violations a
    /// recipe COULD commit if it held `Commands` — a second body answering to a
    /// planned identity, a root wearing this transaction's ownership that no
    /// plan row named, a despawned root. Production can no longer express any of
    /// them ([`Self::root_scope`] is the whole surface), so the adversarial toy
    /// recipes need a door the shipped code does not have.
    ///
    /// ⚠ **THIS IS A SABOTAGE HOOK, NOT AN ESCAPE HATCH.** It is `cfg(test)` and
    /// `pub(crate)`: it cannot be reached from another crate at all, and it
    /// cannot be reached from a non-test build of this one. If it ever needs to
    /// be, the verification it feeds is what has to change.
    #[cfg(test)]
    pub(crate) fn commands_for_sabotage(&mut self) -> &mut Commands<'w, 's> {
        self.commands
    }

    /// Put components on this row's root, scoped to the gameplay session that
    /// requested the commit — the room-retirement lifetime every placed body
    /// wants.
    pub fn insert_in_session(&mut self, bundle: impl bevy::prelude::Bundle) -> &mut Self {
        use crate::lifecycle::SpawnSessionScopedExt as _;
        self.commands
            .insert_room_in_session(self.session, self.root, bundle);
        self
    }
}

/// A writer bound to ONE already-allocated entity.
///
/// ⭐⭐ **THIS IS HALF OF WHAT DELETES `commands_escape`.** The escape existed
/// because the monolith's recipes delegate to `spawn_*_into` helpers that took
/// `&mut Commands`, and a recipe holding raw `Commands` can mint an
/// authoritative root that `commit_inactive` never hides — a half-visible
/// candidate scene, which is exactly the property A10 is about. Handing those
/// helpers a scope instead makes the mint **unexpressible** rather than
/// policy-checked: there is no `spawn`, no `spawn_empty`, and no way to name any
/// entity other than the one the scope was built around.
///
/// ⚠ **IT IS NOT A CAPABILITY TOKEN AND MUST NOT BE READ AS ONE.** Anyone
/// holding `&mut Commands` can build one for any entity — [`EntityScope::new`]
/// is public because the non-planner spawn roads legitimately allocate their own
/// entity and then populate it. What the type guarantees is about the CALLEE,
/// not the caller: a function that accepts a scope cannot create an entity,
/// however it was handed one.
///
/// ⇒ This is the SESSION-LESS half. Use it for a helper that finishes an entity
/// whose ownership was settled by whoever allocated it — the re-template pass
/// and the match-seat materializer both do exactly that. A helper that needs to
/// STAMP ownership wants [`RootScope`] instead, and the split is the point:
/// `EntityScope` cannot claim an ownership it was never told.
pub struct EntityScope<'w, 's, 'a> {
    entity: Entity,
    commands: &'a mut Commands<'w, 's>,
}

impl<'w, 's, 'a> EntityScope<'w, 's, 'a> {
    /// Bind a writer to an entity that ALREADY EXISTS.
    ///
    /// ⛔ The caller is asserting the entity is allocated; this does not create
    /// it and cannot.
    pub fn new(commands: &'a mut Commands<'w, 's>, entity: Entity) -> Self {
        Self { entity, commands }
    }

    /// The entity every write on this scope lands on.
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Put components on it.
    pub fn insert(&mut self, bundle: impl bevy::prelude::Bundle) -> &mut Self {
        self.commands.entity(self.entity).insert(bundle);
        self
    }

    /// Take components off it.
    ///
    /// Present because a grant that cannot be RETRACTED is a grant no second
    /// writer can stand down from, and the re-template road retracts exactly
    /// what it granted.
    pub fn remove<B: bevy::prelude::Bundle>(&mut self) -> &mut Self {
        self.commands.entity(self.entity).remove::<B>();
        self
    }

    /// Queue a mutation of ONE component on this entity, applied when the
    /// command queue flushes and SKIPPED if the component is absent.
    ///
    /// ⭐⭐ **THIS EXISTS SO THE SCOPE DOES NOT NEED `Commands::queue`.** A
    /// queued `&mut World` closure is a complete escape — it can spawn — so
    /// exposing one would give back everything the scope takes away. The only
    /// production use is a deferred component edit on the scope's own entity
    /// (`switch_motion_model`, which must preserve shared body facts rather than
    /// replace the component), and that is exactly what this expresses.
    ///
    /// ⚠ ABSENT IS A NO-OP, deliberately: the caller is asking to adjust a
    /// component it did not insert, and a body that never had one is not a
    /// fault.
    pub fn queue_component_mut<C>(&mut self, edit: impl FnOnce(&mut C) + Send + 'static) -> &mut Self
    where
        C: bevy::prelude::Component<Mutability = bevy::ecs::component::Mutable>,
    {
        let entity = self.entity;
        self.commands.queue(move |world: &mut World| {
            if let Some(mut component) = world.get_mut::<C>(entity) {
                edit(&mut component);
            }
        });
        self
    }

    /// Queue a mutation of ONE component on this entity, CREATING it first when
    /// it is absent.
    ///
    /// ⭐⭐ **THE OTHER HALF OF [`Self::queue_component_mut`], AND THE ONE
    /// `wire_limb` ACTUALLY NEEDS.** A host's `LimbRig` is built by whichever
    /// limb is wired first and extended by the rest, so "adjust it if present"
    /// and "make it if absent" are one operation — expressed as two, a caller
    /// reaches for `Commands::queue` and takes back the whole `&mut World`
    /// escape this scope exists to remove.
    ///
    /// ⚠ `make` RUNS ONLY WHEN THE COMPONENT IS ABSENT, so a caller may put the
    /// expensive construction there; `edit` runs in both cases, so the two
    /// paths cannot disagree about what "wired" means.
    pub fn queue_component_upsert<C>(
        &mut self,
        make: impl FnOnce() -> C + Send + 'static,
        edit: impl FnOnce(&mut C) + Send + 'static,
    ) -> &mut Self
    where
        C: bevy::prelude::Component<Mutability = bevy::ecs::component::Mutable>,
    {
        let entity = self.entity;
        self.commands.queue(move |world: &mut bevy::prelude::World| {
            let Ok(mut entity_ref) = world.get_entity_mut(entity) else {
                return;
            };
            if let Some(mut existing) = entity_ref.get_mut::<C>() {
                edit(&mut existing);
            } else {
                let mut fresh = make();
                edit(&mut fresh);
                entity_ref.insert(fresh);
            }
        });
        self
    }

    /// ⛔⛤ **`rebind` WAS HERE AND IS DELETED, 2026-09-12.** It bound a second
    /// scope to a DIFFERENT entity — justified in its own doc by a cluster road
    /// "where one function writes to each in turn" — and **MEASURED at HEAD, no
    /// production code ever called it.** A recipe holding a `RootScope` could
    /// therefore still manufacture a writer over any entity it could name, which
    /// is most of what removing `Commands` was for. A capability with no caller
    /// is not narrower than one with a caller; it is the same capability,
    /// untested.
    ///
    /// ⇒ If a cluster road ever needs it, it arrives WITH that caller.
    /// Hand this scope to a callee without giving up ownership of it.
    pub fn reborrow(&mut self) -> EntityScope<'w, 's, '_> {
        EntityScope {
            entity: self.entity,
            commands: self.commands,
        }
    }

}

/// An [`EntityScope`] that also knows the gameplay session owning its entity,
/// so it can STAMP that ownership as it writes.
///
/// ⚠ **THE THREE INSERTS ARE THE THREE OWNERSHIP FLAVOURS, NAMED AS THE
/// `SpawnSessionScopedExt` TRAIT NAMES THEM** — `insert`,
/// `insert_session_scoped`, `insert_room_in_session`. They are deliberately NOT
/// collapsed to one defaulted call: a body that is room-scoped when it should be
/// session-scoped survives a room change it should not, and that distinction has
/// no test that can see it, so it is spelled at every site.
pub struct RootScope<'w, 's, 'a> {
    entity: EntityScope<'w, 's, 'a>,
    session: crate::lifecycle::SessionSpawnScope,
}

impl<'w, 's, 'a> RootScope<'w, 's, 'a> {
    /// Bind a writer to an allocated entity and the session that owns it.
    ///
    /// ⛔ A construction recipe never calls this — it receives a scope from
    /// [`ConstructionRootCtx::root_scope`], built around the root the executor
    /// minted.
    pub fn new(
        commands: &'a mut Commands<'w, 's>,
        session: crate::lifecycle::SessionSpawnScope,
        root: Entity,
    ) -> Self {
        Self {
            entity: EntityScope::new(commands, root),
            session,
        }
    }

    /// The entity every write on this scope lands on.
    pub fn root(&self) -> Entity {
        self.entity.entity()
    }

    /// The gameplay session that owns this root.
    pub fn session(&self) -> crate::lifecycle::SessionSpawnScope {
        self.session
    }

    /// Put components on the root, with no ownership stamp of their own.
    pub fn insert(&mut self, bundle: impl bevy::prelude::Bundle) -> &mut Self {
        self.entity.insert(bundle);
        self
    }

    /// Put components on the root and stamp it as owned by this scope's
    /// gameplay session.
    pub fn insert_session_scoped(&mut self, bundle: impl bevy::prelude::Bundle) -> &mut Self {
        use crate::lifecycle::SpawnSessionScopedExt as _;
        let (entity, commands) = (self.entity.entity, &mut *self.entity.commands);
        commands.insert_session_scoped(self.session, entity, bundle);
        self
    }

    /// Put components on the root and stamp it as owned by BOTH the active
    /// authored room and this scope's gameplay session — the retirement
    /// lifetime a placed body wants.
    pub fn insert_room_in_session(&mut self, bundle: impl bevy::prelude::Bundle) -> &mut Self {
        use crate::lifecycle::SpawnSessionScopedExt as _;
        let (entity, commands) = (self.entity.entity, &mut *self.entity.commands);
        commands.insert_room_in_session(self.session, entity, bundle);
        self
    }

    /// Drop to the session-less scope, for a helper that finishes an entity
    /// without claiming to own it.
    pub fn entity_scope(&mut self) -> EntityScope<'w, 's, '_> {
        self.entity.reborrow()
    }

    /// Hand this scope to a callee without giving up ownership of it.
    pub fn reborrow(&mut self) -> RootScope<'w, 's, '_> {
        RootScope {
            entity: self.entity.reborrow(),
            session: self.session,
        }
    }

}

/// Populates one planned row's already-allocated root.
///
/// A recipe cannot choose the entity, return a different one, or hand back
/// something that was already alive — it receives a [`ConstructionRoot`] the
/// executor minted. It also cannot fail: it returns nothing.
pub type ConstructFn<D> = for<'w, 's, 'a> fn(
    &<D as ConstructionDomain>::Parameters,
    &mut ConstructionRootCtx<'w, 's, 'a, D>,
);

/// The authoritative entity the executor allocated for one planned row.
///
/// A recipe receives this instead of creating its own body. The inner `Entity`
/// is reachable — a recipe legitimately needs it to insert components and to
/// parent deliberate child entities — but only [`ConstructionPlan`] can mint
/// one, so a recipe cannot nominate a pre-existing entity as a row's root.
///
/// This is the executor invariant, and it is narrower than "one planned
/// row, one authoritative root". What is mechanically guaranteed is that the
/// executor allocates each nominal planned root and freezes its constructor.
/// What is NOT guaranteed is that the recipe leaves that root alone or refrains
/// from creating authoritative entities beside it: it holds raw `Commands`, so
/// it can despawn the root, restamp it, or spawn ten more. Those are caught
/// after the fact by [`verify_committed_roster`] — the verification invariant —
/// not prevented here. Making every authoritative root an explicit plan row is
/// the future structural invariant, and it is Phase-4 work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstructionRoot(Entity);

impl ConstructionRoot {
    /// The allocated entity, for inserting components onto it.
    pub fn entity(self) -> Entity {
        self.0
    }
}

/// What one plan describes: which content generation, and which room.
///
/// Session ownership is deliberately absent. It is a commit-time fact — one
/// prepared room plan is committed by whichever activation requested it, which
/// is why `PlacementLoweringPlan` also takes its `SessionSpawnScope` at
/// `lower_all` rather than at `plan_room`. A domain that needs it carries it in
/// [`ConstructionDomain::Services`], where it is captured alongside the other
/// frozen facts execution reads.
/// ⛔⛤ **TWO BINDINGS, BECAUSE A REPLACEMENT IS TWO FACTS AND ONE FIELD WAS
/// HOLDING BOTH — MEASURED 2026-09-13.** This was a single `binding`, and its two
/// readers wanted different answers:
///
/// - `transaction()` folds it into every root's `TransactionId`, which is
///   canonical rollback state: that wants the generation the content CAME FROM.
/// - `transaction::close` compares it against the live `ActiveContentBinding` to
///   refuse a stale plan: that wants the generation the plan will be COMMITTED
///   INTO.
///
/// For every road except a content replacement those coincide — a door and a
/// death rebuild a room inside the generation already running — which is exactly
/// why nobody noticed. A materially changed hot reload is the case where they
/// differ, and it ended with content N+1 live and every rebuilt root's
/// `TransactionId` naming **N**.
///
/// ⚠ **THE FIELDS ARE PRIVATE AND THERE ARE TWO CONSTRUCTORS**, so a caller
/// states which SHAPE it is rather than assigning two values that may disagree.
/// [`Self::in_generation`] is the ordinary road and cannot express a split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionScope {
    /// The generation this plan expects to be COMMITTED INTO. The staleness
    /// comparison at the boundary is against this.
    expected_live: ContentBinding,
    /// The generation this plan's content came FROM, and which its roots are
    /// stamped with.
    incoming: ContentBinding,
    /// The room being constructed, when the plan is a room's contents.
    room: Option<String>,
}

impl ConstructionScope {
    /// A plan built from, and committed into, ONE generation — every road except
    /// a content replacement.
    pub fn in_generation(binding: ContentBinding, room: Option<String>) -> Self {
        Self {
            expected_live: binding,
            incoming: binding,
            room,
        }
    }

    /// A plan carrying generation `incoming` into a world still running
    /// `expected_live` — the content-replacement shape.
    ///
    /// ⭐ The asymmetry is the point: the boundary must still recognise the world
    /// it is publishing into as the one the preflight ran against, while the
    /// roots it mints belong to the generation that replaces it.
    pub fn replacing(
        expected_live: ContentBinding,
        incoming: ContentBinding,
        room: Option<String>,
    ) -> Self {
        Self {
            expected_live,
            incoming,
            room,
        }
    }

    /// The generation this plan expects to find live at its commit boundary.
    pub fn expected_live(&self) -> ContentBinding {
        self.expected_live
    }

    /// The generation this plan's roots belong to.
    pub fn incoming(&self) -> ContentBinding {
        self.incoming
    }

    /// The room this plan describes, when it describes one.
    pub fn room(&self) -> Option<&str> {
        self.room.as_deref()
    }
}

/// Whether a plan is bound to a generation of prepared content, and which.
///
/// This replaces a bare `ContentEpoch` whose zero value meant three different
/// things: "a fixture stated nothing", "a reset rebuilds the content already
/// active so states no new generation", and "a summon is not content at all".
/// Only the last is genuinely not content-bound; the other two were content-
/// bound plans that had simply lost track of which generation they belonged to,
/// and no commit boundary could tell them apart from a legitimately generation-
/// free one. Phase 4 turns staleness into a refusal, and a refusal cannot be
/// built on a sentinel that three unrelated callers spell the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentBinding {
    /// Prepared against one exact generation of prepared content.
    Content(ambition_platformer2d_core::ContentEpoch),
    /// Not derived from prepared content at all — a summon, a projectile, a
    /// dropped item. Built and committed inside a single tick, so it cannot
    /// outlive a reload and has no generation to be stale against.
    RuntimeDynamic,
}

impl ContentBinding {
    /// The generation this plan names, for a commit boundary to compare against
    /// the live one. `None` means the plan is not content-derived and staleness
    /// does not apply to it — NOT that its generation is unknown.
    pub const fn content_epoch(self) -> Option<ambition_platformer2d_core::ContentEpoch> {
        match self {
            Self::Content(epoch) => Some(epoch),
            Self::RuntimeDynamic => None,
        }
    }

    /// Byte-stable rendering for the plan dump.
    pub fn canonical_summary(self) -> String {
        match self {
            Self::Content(epoch) => format!("{epoch}"),
            Self::RuntimeDynamic => "runtime-dynamic".to_string(),
        }
    }
}

impl ConstructionScope {
    /// The ownership token every root this scope constructs under `session` will
    /// carry.
    ///
    /// Derived from the scope and the committing session rather than drawn from
    /// a counter, so it needs no clock, no randomness, and no rollback-registered
    /// state: committing the same room, at the same content generation, in the
    /// same session, twice yields the same token — which is what a deterministic
    /// simulation requires, and what makes a same-room reconstruction recognise
    /// its own previous roots instead of calling them foreign.
    ///
    /// The session is part of the key, and it has to be. This was
    /// `binding + room` alone, which is a CONSTRUCTION-SCOPE identity, not a
    /// transaction identity: the shell host runs two gameplay sessions in one
    /// process, so two sessions committing the same room at the same content
    /// epoch minted the same token. Each would then classify the other's roots
    /// as [`ScopeClassification::TransactionAuthoritative`] — its own — and
    /// report every one of them as [`RosterViolation::Unplanned`], while a root
    /// genuinely belonging to the other session would be accepted as this one's.
    /// Session ownership is a commit-time fact, so it enters here at commit time
    /// rather than being folded into [`ConstructionScope`].
    pub fn transaction(&self, session: crate::lifecycle::SessionSpawnScope) -> TransactionId {
        TransactionId(format!(
            "{}\t{}\t{}",
            // ⛔ THE INCOMING GENERATION, NOT THE EXPECTED-LIVE ONE. A root
            // belongs to the content it was built from; the other binding is the
            // boundary's staleness comparison and has no business in an identity
            // the rollback timeline carries. They differ only on a replacement.
            self.incoming.canonical_summary(),
            self.room.as_deref().unwrap_or("-"),
            match session.id() {
                Some(id) => format!("session:{id:?}"),
                None => "unscoped".to_string(),
            },
        ))
    }
}

/// One independently-owned construction lane inside a larger transaction.
///
/// A room may compose several closed, strongly typed construction domains
/// (actor bodies, a portal-gun capability, later items or other capabilities)
/// without erasing their parameter types into a universal executable registry.
/// Each lane gets its own ownership token so one domain's verifier treats the
/// others as foreign scope while the outer room transaction still preflights
/// every plan before mutating and publishes only after every lane verifies.
///
/// [`Self::primary`] preserves the historical transaction token byte-for-byte.
/// Named lanes add one stable suffix, so existing actor roots retain their
/// snapshot/replay identity while newly federated domains cannot be mistaken for
/// actor-owned roots.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstructionLane(String);

impl ConstructionLane {
    /// The historical/default lane. Its ownership token is exactly
    /// [`ConstructionScope::transaction`].
    pub fn primary() -> Self {
        Self(String::new())
    }

    /// A stable named lane. Empty names are rejected by construction rather
    /// than silently collapsing onto the primary lane.
    pub fn named(name: impl Into<String>) -> Self {
        let name = name.into();
        assert!(
            !name.trim().is_empty(),
            "a named construction lane must have a non-empty identity"
        );
        assert!(
            name != "primary",
            "`primary` is reserved for the unnamed construction lane"
        );
        assert!(
            !name.chars().any(|ch| matches!(ch, '\t' | '\n' | '\r')),
            "a construction lane identity must fit the canonical one-line format"
        );
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        if self.0.is_empty() {
            "primary"
        } else {
            &self.0
        }
    }

    fn transaction(
        &self,
        scope: &ConstructionScope,
        session: crate::lifecycle::SessionSpawnScope,
    ) -> TransactionId {
        let base = scope.transaction(session);
        if self.0.is_empty() {
            base
        } else {
            TransactionId(format!("{}\tlane:{}", base.as_str(), self.0))
        }
    }
}

impl Default for ConstructionLane {
    fn default() -> Self {
        Self::primary()
    }
}

/// A root that has been CONSTRUCTED but not PUBLISHED: it exists, it is wired,
/// and **no ordinary query can see it.**
///
/// ⭐⭐ **THIS IS A10's LAST-GOOD-WORLD GUARANTEE IN ONE COMPONENT, AND IT IS NOT
/// A MARKER.** A10's own text warns that *"a `Pending` marker does not isolate a
/// candidate from queries, observers or hooks"* — true, and the reason is that a
/// marker asks every reader to remember it. This is registered with
/// [`register_inactive_candidate_filter`] as a bevy DISABLING component, so
/// `DefaultQueryFilters` excludes it from **every query that does not name it**.
/// The isolation is the engine's, not a convention.
///
/// ⛔ WHAT IT DOES NOT DO, stated so nobody assumes the rest: it does not stop
/// component HOOKS or lifecycle OBSERVERS from firing as the candidate is built.
/// Measured 2026-09-12: this workspace declares ZERO component hooks and THREE
/// lifecycle observers, exactly one of which is an `Add` (a touch surface, not on
/// the construction road). So the residual surface is enumerable and currently
/// empty for construction — but it is a POPULATION FACT, not a boundary, and it
/// is the thing to re-measure before trusting this in a new domain.
///
/// ⇒ The candidate keeps its `SimId`, its `SpawnOrigin`, its [`TransactionId`]
/// and its relationships while invisible, which is what makes publication a
/// component REMOVAL rather than a transfer.
/// ⛔⛤ **`pub(crate)`, AND THE PRIVACY IS THE CONTRACT (`Q123`, 2026-09-13).**
/// Publication removes this marker one entity at a time, so a hook or observer
/// attached to it observes the transaction PARTIALLY PUBLISHED — measured
/// `[3, 2, 1]` still-hidden siblings across a three-root publication. A workspace
/// census found zero such hooks, but a census is a population fact and this type
/// was `pub`, so any crate could add the first one. ⇒ Nobody outside this crate
/// can name it, and therefore nobody outside this crate can hook it. See
/// [`publish_candidate`] for the guarantee this narrows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct InactiveCandidate;

/// Teach this world that [`InactiveCandidate`] hides an entity from ordinary
/// queries.
///
/// ⛔⛔ **WITHOUT THIS CALL THE COMPONENT IS INERT AND EVERY CANDIDATE IS LIVE.**
/// That is the dangerous failure direction — the isolation silently does
/// nothing and the candidate participates in the running world — so
/// [`ConstructionPlan::commit_inactive`] REFUSES rather than committing when the
/// filter is not installed. A composition that builds candidates calls this
/// once, at build.
pub fn register_inactive_candidate_filter(world: &mut World) {
    world.register_disabling_component::<InactiveCandidate>();
}

/// Is the disabling filter installed in this world?
///
/// Asked by `commit_inactive` before it stamps anything, because a candidate
/// that is not actually hidden is worse than no candidate at all.
pub fn inactive_candidate_filter_installed(world: &mut World) -> bool {
    let Some(id) = world.components().component_id::<InactiveCandidate>() else {
        return false;
    };
    world
        .get_resource::<bevy::ecs::entity_disabling::DefaultQueryFilters>()
        .is_some_and(|filters| filters.disabling_ids().any(|disabling| disabling == id))
}

/// Which construction transaction owns an authoritative root.
///
/// Stamped by the executor, on every root it allocates. This is what lets
/// verification ask the WORLD which roots are in scope instead of trusting a
/// caller to list them — a caller that forgets the root a recipe invented is
/// exactly the caller whose transaction most needs checking.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionId(String);

impl TransactionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Rebuild a stamp from its stored string — the snapshot codec's decode
    /// half. Not a way to MINT ownership: the only live producer is
    /// [`ConstructionScope::transaction`] for the primary lane or a prepared
    /// [`ConstructionPlan::transaction`] for a named lane, and a rollback restore reproduces
    /// exactly the stamp the executor wrote.
    pub fn from_raw(raw: String) -> Self {
        Self(raw)
    }
}

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Declares an identity-bearing entity to be deliberately NOT authoritative:
/// a presentation child, a helper body, a visual double.
///
/// Opt-out rather than opt-in, and that asymmetry is the point. An identity-bearing entity is
/// authoritative until something says otherwise, so forgetting to classify is a loud violation
/// instead of a quiet exemption.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationOnly;

/// One requested entity, before validation.
///
/// There is no `recipe` field either. Which recipe builds a row is derived
/// from its parameters by [`ConstructionDomain::dispatch`], so a request that
/// names one recipe while carrying another's payload is not a thing that can be
/// written down.
///
/// There is no `parent` field. The spawner an entity descends from is
/// already stated by [`SpawnOrigin::Dynamic`], and a request that carried it
/// twice would have a state where the validated parent and the recorded
/// provenance disagree — with nothing to say which one reconstruction should
/// believe. Preparation validates [`SpawnOrigin::parent`] directly, so the fact
/// that is checked is the same fact that reaches the world.
pub struct ConstructionRequest<D: ConstructionDomain> {
    pub sim_id: SimId,
    pub origin: SpawnOrigin,
    pub parameters: D::Parameters,
    /// Relations this entity declares onto others. Validated against the plan's
    /// own roster plus the live roster before anything is spawned.
    pub relations: Vec<RelationRequest<D>>,
}

/// A declared relation from the requesting entity onto another identity.
///
/// There is no `kind` field. It is derived from `relation` by
/// [`ConstructionDomain::dispatch_relation`], for the same reason [`ConstructionRequest`] has no
/// `recipe`: a request that names one kind while carrying another's facts is not a thing that can
/// be written down.
pub struct RelationRequest<D: ConstructionDomain> {
    pub to: SimId,
    /// What this relation IS — see [`ConstructionDomain::Relation`].
    pub relation: D::Relation,
}

impl<D: ConstructionDomain> Clone for RelationRequest<D> {
    fn clone(&self) -> Self {
        Self {
            to: self.to.clone(),
            relation: self.relation.clone(),
        }
    }
}

/// One validated entity with its interpreter already resolved.
///
/// The recipe pointer is stored beside the row for the same reason
/// `PlannedPlacement` stores its lowering function: commit does not repeat
/// registry lookup, and cannot discover a missing recipe after the outgoing
/// world has begun to retire.
pub struct PlannedEntity<D: ConstructionDomain> {
    sim_id: SimId,
    /// Resolved once at preparation via [`ConstructionDomain::dispatch`], and
    /// what the dump, the registry check, and the fingerprint all name.
    recipe: RecipeId,
    /// The resolved constructor, frozen beside its identity.
    ///
    /// Commit runs THIS, and never asks the domain again. `dispatch` is
    /// expected to be a pure function of the parameters, but nothing in the
    /// type system makes it one: an implementation may read an atomic, an
    /// environment variable, or any other mutable process state. Re-resolving
    /// at commit therefore allowed a plan to validate recipe A, dump recipe A,
    /// fingerprint recipe A — and execute constructor B. Freezing it here is
    /// what makes "prepared" mean prepared.
    ///
    /// Deliberately absent from every canonical surface: a `fn` address is
    /// runtime execution state, not content identity. The dump and the
    /// fingerprint carry [`Self::recipe`] instead.
    construct: ConstructFn<D>,
    origin: SpawnOrigin,
    parameters: D::Parameters,
}

impl<D: ConstructionDomain> Clone for PlannedEntity<D> {
    fn clone(&self) -> Self {
        Self {
            sim_id: self.sim_id.clone(),
            recipe: self.recipe.clone(),
            construct: self.construct,
            origin: self.origin.clone(),
            parameters: self.parameters.clone(),
        }
    }
}

impl<D: ConstructionDomain> PlannedEntity<D> {
    pub fn sim_id(&self) -> &SimId {
        &self.sim_id
    }
    pub fn recipe(&self) -> &RecipeId {
        &self.recipe
    }
    pub fn origin(&self) -> &SpawnOrigin {
        &self.origin
    }
    /// The spawner this row descends from — read from its provenance, which is
    /// the only place it is stored.
    pub fn parent(&self) -> Option<&SimId> {
        self.origin.parent()
    }
    pub fn parameters(&self) -> &D::Parameters {
        &self.parameters
    }
}

/// One validated relation with both its wiring function and its
/// postcondition check already resolved.
///
/// Frozen at preparation for the same reason [`PlannedEntity::construct`] is: commit runs what
/// the plan validated, not whatever a later lookup returns.
pub struct PlannedRelation<D: ConstructionDomain> {
    from: SimId,
    /// Derived at preparation by [`ConstructionDomain::dispatch_relation`] and
    /// frozen here. Commit never redispatches, so the kind that was dumped,
    /// deduplicated, registry-checked, and ordered is the kind that executes.
    kind: RelationKind,
    to: SimId,
    relation: D::Relation,
    ops: RelationOps<D>,
}

impl<D: ConstructionDomain> Clone for PlannedRelation<D> {
    fn clone(&self) -> Self {
        Self {
            from: self.from.clone(),
            kind: self.kind.clone(),
            to: self.to.clone(),
            relation: self.relation.clone(),
            ops: self.ops,
        }
    }
}

impl<D: ConstructionDomain> PlannedRelation<D> {
    pub fn from(&self) -> &SimId {
        &self.from
    }
    pub fn kind(&self) -> &RelationKind {
        &self.kind
    }
    pub fn to(&self) -> &SimId {
        &self.to
    }
    pub fn relation(&self) -> &D::Relation {
        &self.relation
    }
}

/// Why a construction plan could not be prepared, or a single row could not be
/// re-executed. Every variant is detected before any world mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructionError {
    /// Two rows claim the same identity.
    DuplicateIdentity { sim_id: SimId },
    /// A row claims an identity that a live entity already holds.
    IdentityAlreadyLive { sim_id: SimId },
    /// A row names a recipe the registry does not have.
    UnknownRecipe { sim_id: SimId, recipe: RecipeId },
    /// A row's parent resolves to neither a planned nor a live identity.
    UnresolvedParent { sim_id: SimId, parent: SimId },
    /// A relation's target names nothing this plan knows about.
    ///
    /// Both ends of a relation must be rows in the same plan. A target that is
    /// merely *live* is rejected rather than accepted-and-skipped: commit wires
    /// relations from the identities it just constructed, so it has no entity
    /// for an outsider, and quietly dropping the relation would recreate the
    /// exact silent-skip this planner exists to remove. Relating to a live
    /// entity is a real need — it is Phase 4's, alongside the commit boundary
    /// that will hold a live identity index.
    UnresolvedRelation {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// A relation resolves to a kind the registry does not declare.
    UnknownRelationKind { from: SimId, kind: RelationKind },
    /// Two rows declare the same relation between the same two identities.
    ///
    /// Refused rather than deduplicated, because the two are not the same
    /// outcome. Executing a duplicate runs the wiring TWICE while the receipt —
    /// a `BTreeSet` keyed on exactly this triple — records it once, so the
    /// receipt says "wired" and the world holds it applied twice. For an
    /// accumulating relation that is a real corruption: a limb appended to its
    /// host's rig twice is driven twice per frame, and every "is the limb in the
    /// rig" check still passes.
    DuplicateRelation {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// A single-row re-execution named an identity this plan does not contain.
    NotInPlan { sim_id: SimId },
    /// A partial commit would have cut a relation: exactly one of its two ends
    /// is being rebuilt.
    ///
    /// Both directions are refused, and the reason is that a relation is an `Entity` handle.
    /// Rebuilding the SOURCE alone leaves it unwired, which is obvious. Rebuilding the TARGET alone
    /// is worse and was briefly allowed here on the reasoning that the relation "belongs to" the
    /// untouched source: it does, but what the source holds is a handle to the entity that just
    /// died, so the source is left pointing at a corpse.
    ///
    /// [`ConstructionPlan::relation_closure`] turns a seed set into one that
    /// cannot be refused for this reason.
    RelationCutBySubset {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
}

impl std::fmt::Display for ConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateIdentity { sim_id } => {
                write!(f, "two planned entities claim identity `{sim_id}`")
            }
            Self::IdentityAlreadyLive { sim_id } => write!(
                f,
                "planned identity `{sim_id}` is already held by a live entity"
            ),
            Self::UnknownRecipe { sim_id, recipe } => write!(
                f,
                "`{sim_id}` names construction recipe `{recipe}`, which is not registered"
            ),
            Self::UnresolvedParent { sim_id, parent } => write!(
                f,
                "`{sim_id}` declares parent `{parent}`, which is neither planned nor live"
            ),
            Self::UnresolvedRelation { from, kind, to } => write!(
                f,
                "`{from}` declares relation `{kind}` onto `{to}`, which is neither planned nor live"
            ),
            Self::UnknownRelationKind { from, kind } => write!(
                f,
                "`{from}` declares relation `{kind}`, which no registration declares"
            ),
            Self::DuplicateRelation { from, kind, to } => write!(
                f,
                "relation `{from}` -`{kind}`-> `{to}` is declared more than once: it would be \
                 wired twice and receipted once"
            ),
            Self::NotInPlan { sim_id } => {
                write!(f, "this plan contains no entity `{sim_id}`")
            }
            Self::RelationCutBySubset { from, kind, to } => write!(
                f,
                "this subset cuts relation `{from}` -`{kind}`-> `{to}`: rebuilding one end alone \
                 leaves the other holding a stale entity handle"
            ),
        }
    }
}

impl std::error::Error for ConstructionError {}

/// Execution context handed to a recipe. Mirrors `LoweringCtx`: exactly the
/// facts a recipe needs today, growable by explicit need.
pub struct ConstructionExecCtx<'w, 's, 'a, D: ConstructionDomain> {
    pub commands: &'a mut Commands<'w, 's>,
    /// What the plan describes — content generation and room.
    pub scope: &'a ConstructionScope,
    /// Gameplay-session ownership, captured when this commit was requested.
    ///
    /// Here rather than in [`ConstructionDomain::Services`] for the same reason
    /// `LoweringCtx` carries it beside its context: it is the one fact that
    /// varies between two commits of the SAME frozen plan, so folding it into
    /// the services would force a domain to rebuild them — deep-cloning its
    /// catalogs — once per entity during a reconstruction sweep.
    pub session: crate::lifecycle::SessionSpawnScope,
    pub services: &'a D::Services,
}

/// Everything wiring ONE declared relation may touch: its two endpoints, and
/// the read-only facts of the commit it belongs to.
///
/// ⛔⛤ **A `RelationFn` USED TO RECEIVE `&mut ConstructionExecCtx`, WHOSE
/// `commands` IS PUBLIC — SO IT RECEIVED `&mut World`.** A10 made unplanned
/// minting a TYPE ERROR for recipes (`RootScope`, `ConstructionRootCtx`) and
/// stopped at the relation surface, which a 2026-09-13 review found and called
/// by its right name: the last-good-world guarantee is *"construct N+1 beside N,
/// and on failure keep N intact"*, and a relation could spawn, despawn an
/// unrelated live entity, insert or replace a RESOURCE, or
/// `commands.queue(|world: &mut World|)` its way to anything at all.
/// `commit_inactive` applies that queue BEFORE verification, and
/// `retire_candidate` despawns only this transaction's candidate roots — it
/// cannot undo a resource write. ⇒ **A refused candidate could still have
/// mutated the world it was supposed to leave alone.**
///
/// ⚠ **NOT A LIVE CORRUPTION REPORT, THEN OR NOW.** The three shipped relations
/// (`wire_limb`, `wire_mount`, `wire_grudge`) touch only their declared
/// endpoints. What this removes is the CAPABILITY, so the next registered
/// relation — or an out-of-tree one — cannot violate the guarantee without
/// failing to compile.
///
/// ⭐ **`commands` IS PRIVATE AND THAT IS THE WHOLE MECHANISM.** Everything
/// below hands out [`EntityScope`]s bound to `from` and `to`; there is no
/// method that yields a writer over a third entity, and none that yields
/// `Commands` or `World`.
///
/// ⛔ **WHAT IS DELIBERATELY ABSENT: session-scoped insertion.** The review
/// listed it as a capability a relation *might* want; no shipped relation uses
/// one, and `RootScope::rebind` is the recorded lesson for adding the ones that
/// might — *"a capability with no caller is not narrower than one with a caller;
/// it is the same capability, untested"*. It arrives WITH its caller.
pub struct RelationScope<'w, 's, 'a, D: ConstructionDomain> {
    commands: &'a mut Commands<'w, 's>,
    from: Entity,
    to: Entity,
    /// What the plan describes — content generation and room. Read-only.
    pub scope: &'a ConstructionScope,
    /// Gameplay-session ownership, captured when this commit was requested.
    pub session: crate::lifecycle::SessionSpawnScope,
    /// The domain's own frozen services. Read-only, as they are for a recipe.
    pub services: &'a D::Services,
}

impl<'w, 's, 'a, D: ConstructionDomain> RelationScope<'w, 's, 'a, D> {
    /// The entity the relation points FROM.
    pub fn from_entity(&self) -> Entity {
        self.from
    }

    /// The entity the relation points TO.
    pub fn to_entity(&self) -> Entity {
        self.to
    }

    /// A writer over the `from` endpoint.
    pub fn from(&mut self) -> EntityScope<'w, 's, '_> {
        EntityScope::new(self.commands, self.from)
    }

    /// A writer over the `to` endpoint.
    ///
    /// ⭐ A BIDIRECTIONAL RELATION WIRES BOTH SIDES, which is why this exists at
    /// all — see [`RelationFn`] on why one function owning both ends is what
    /// makes a half-write unspellable.
    pub fn to(&mut self) -> EntityScope<'w, 's, '_> {
        EntityScope::new(self.commands, self.to)
    }

    /// ⚠ **A SABOTAGE HOOK, NOT AN ESCAPE HATCH** — the same door
    /// [`ConstructionRootCtx::commands_for_sabotage`] opens, and for the same
    /// reason. Relation verification has to be poisoned by things a wiring
    /// function COULD do if it held `Commands`: wire a grudge onto a third
    /// entity it spawned, wire FROM one, overwrite its own write with a later
    /// command in the same flush. Production can no longer express any of them,
    /// so the adversarial toy relations need a door the shipped code does not
    /// have.
    ///
    /// ⛔ `cfg(test)` and `pub(crate)`: unreachable from another crate at all,
    /// and from a non-test build of this one. If it ever needs to be reachable,
    /// the verification it feeds is what has to change.
    #[cfg(test)]
    pub(crate) fn commands_for_sabotage(&mut self) -> &mut Commands<'w, 's> {
        self.commands
    }
}

/// What execution actually committed. Compared against the plan to prove
/// plan-to-world parity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConstructionReceipt {
    committed: BTreeMap<SimId, Entity>,
    relations_wired: BTreeSet<(SimId, RelationKind, SimId)>,
}

impl ConstructionReceipt {
    pub fn committed_ids(&self) -> BTreeSet<SimId> {
        self.committed.keys().cloned().collect()
    }

    pub fn entity(&self, sim_id: &SimId) -> Option<Entity> {
        self.committed.get(sim_id).copied()
    }

    pub fn relations_wired(&self) -> &BTreeSet<(SimId, RelationKind, SimId)> {
        &self.relations_wired
    }

    pub fn len(&self) -> usize {
        self.committed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.committed.is_empty()
    }
}

/// The one prepared artifact for a set of entities and the relations between
/// them. Immutable once prepared: every fallible decision is already made.
pub struct ConstructionPlan<D: ConstructionDomain> {
    scope: ConstructionScope,
    lane: ConstructionLane,
    entities: Vec<PlannedEntity<D>>,
    relations: Vec<PlannedRelation<D>>,
}

impl<D: ConstructionDomain> Clone for ConstructionPlan<D> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope.clone(),
            lane: self.lane.clone(),
            entities: self.entities.clone(),
            relations: self.relations.clone(),
        }
    }
}

/// A plan's `Debug` is its canonical dump. When a plan appears in a failure
/// message, the thing worth reading is the roster it would have committed —
/// not a field-by-field rendering of the machinery around it.
impl<D: ConstructionDomain> std::fmt::Debug for ConstructionPlan<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.deterministic_dump())
    }
}

impl<D: ConstructionDomain> ConstructionPlan<D> {
    /// Validate and freeze a set of requests. Pure: it borrows the registry and
    /// the live roster and mutates neither, so a rejected plan cannot have
    /// touched the world.
    ///
    /// Rows are ordered canonically by identity, not by arrival, so two callers
    /// that request the same set in different orders produce byte-identical
    /// plans. Spawn order therefore does not carry meaning — which is the same
    /// rule `SimId` already imposes on snapshot rows.
    pub fn prepare(
        scope: ConstructionScope,
        requests: impl IntoIterator<Item = ConstructionRequest<D>>,
        live: &BTreeSet<SimId>,
        registry: &ConstructionRegistry<D>,
    ) -> Result<Self, ConstructionError> {
        Self::prepare_in_lane(scope, ConstructionLane::primary(), requests, live, registry)
    }

    /// Validate and freeze requests in one named lane of an outer transaction.
    ///
    /// This is the federation seam: the lane is ownership metadata only. Recipe
    /// dispatch stays closed and typed on `D`; no executable behavior is looked
    /// up through a cross-domain registry.
    pub fn prepare_in_lane(
        scope: ConstructionScope,
        lane: ConstructionLane,
        requests: impl IntoIterator<Item = ConstructionRequest<D>>,
        live: &BTreeSet<SimId>,
        registry: &ConstructionRegistry<D>,
    ) -> Result<Self, ConstructionError> {
        let mut requests: Vec<ConstructionRequest<D>> = requests.into_iter().collect();
        requests.sort_by(|a, b| a.sim_id.cmp(&b.sim_id));

        // Identity first: a duplicate makes every later diagnostic ambiguous.
        let mut planned_ids: BTreeSet<SimId> = BTreeSet::new();
        for request in &requests {
            if live.contains(&request.sim_id) {
                return Err(ConstructionError::IdentityAlreadyLive {
                    sim_id: request.sim_id.clone(),
                });
            }
            if !planned_ids.insert(request.sim_id.clone()) {
                return Err(ConstructionError::DuplicateIdentity {
                    sim_id: request.sim_id.clone(),
                });
            }
        }

        // A PARENT may be live: a summoner outlives the summon it plans. A
        // RELATION target may not — see `ConstructionError::UnresolvedRelation`.
        let parent_resolvable = |id: &SimId| planned_ids.contains(id) || live.contains(id);

        let mut entities = Vec::with_capacity(requests.len());
        let mut relations: Vec<PlannedRelation<D>> = Vec::new();
        let mut declared_relations: BTreeSet<(SimId, RelationKind, SimId)> = BTreeSet::new();
        for request in requests {
            // Derived, not supplied — so it always matches the payload — and
            // resolved together with the executor that will build it.
            let dispatch = D::dispatch(&request.parameters);
            if !registry.has_recipe(&dispatch.recipe) {
                return Err(ConstructionError::UnknownRecipe {
                    sim_id: request.sim_id,
                    recipe: dispatch.recipe,
                });
            }
            // The parent comes from the provenance, not from a second field
            // beside it: the fact validated here is the fact the world receives.
            if let Some(parent) = request.origin.parent() {
                if !parent_resolvable(parent) {
                    return Err(ConstructionError::UnresolvedParent {
                        sim_id: request.sim_id.clone(),
                        parent: parent.clone(),
                    });
                }
            }
            for relation in &request.relations {
                // Derived, not supplied — so the kind always matches what the
                // relation carries — and resolved together with the wiring and
                // the check that will run.
                let dispatch = D::dispatch_relation(&relation.relation);
                if !registry.has_relation(&dispatch.kind) {
                    return Err(ConstructionError::UnknownRelationKind {
                        from: request.sim_id.clone(),
                        kind: dispatch.kind,
                    });
                }
                if !planned_ids.contains(&relation.to) {
                    return Err(ConstructionError::UnresolvedRelation {
                        from: request.sim_id.clone(),
                        kind: dispatch.kind,
                        to: relation.to.clone(),
                    });
                }
                // A duplicate endpoint/kind row is refused HERE, before ordering.
                // Two rows that sort equal would otherwise execute twice and
                // collapse into ONE receipt entry — so the receipt would show a
                // relation wired once while the world had it applied twice, which
                // for an accumulating relation (a limb appended to a rig) is a
                // real corruption that every count-based check passes. Refusing
                // also makes `(from, kind, to)` a total order, so request arrival
                // order cannot reach the dump or the execution sequence.
                if !declared_relations.insert((
                    request.sim_id.clone(),
                    dispatch.kind.clone(),
                    relation.to.clone(),
                )) {
                    return Err(ConstructionError::DuplicateRelation {
                        from: request.sim_id.clone(),
                        kind: dispatch.kind,
                        to: relation.to.clone(),
                    });
                }
                relations.push(PlannedRelation {
                    from: request.sim_id.clone(),
                    kind: dispatch.kind,
                    to: relation.to.clone(),
                    relation: relation.relation.clone(),
                    ops: dispatch.ops,
                });
            }
            entities.push(PlannedEntity {
                sim_id: request.sim_id,
                recipe: dispatch.recipe,
                construct: dispatch.construct,
                origin: request.origin,
                parameters: request.parameters,
            });
        }
        relations.sort_by(|a, b| (&a.from, &a.kind, &a.to).cmp(&(&b.from, &b.kind, &b.to)));

        Ok(Self {
            scope,
            lane,
            entities,
            relations,
        })
    }

    pub fn scope(&self) -> &ConstructionScope {
        &self.scope
    }

    pub fn lane(&self) -> &ConstructionLane {
        &self.lane
    }

    /// Ownership token this exact prepared lane stamps under `session`.
    pub fn transaction(&self, session: crate::lifecycle::SessionSpawnScope) -> TransactionId {
        self.lane.transaction(&self.scope, session)
    }

    pub fn entities(&self) -> &[PlannedEntity<D>] {
        &self.entities
    }

    pub fn relations(&self) -> &[PlannedRelation<D>] {
        &self.relations
    }

    /// The identities this plan NOMINATES.
    ///
    /// Not "the exact committed roster". It is what the plan asked for,
    /// which is a different thing from what the world ends up holding: a recipe
    /// can despawn its root, duplicate an identity onto a second body, or spawn
    /// authoritative entities of its own, and none of that moves this set.
    /// Comparing it to [`ConstructionReceipt::committed_ids`] compares the
    /// executor's bookkeeping against itself and would stay green through every
    /// one of those. [`verify_committed_roster`] compares against the WORLD,
    /// which is the only comparison that can fail for a real reason.
    pub fn planned_ids(&self) -> BTreeSet<SimId> {
        self.entities
            .iter()
            .map(|entity| entity.sim_id.clone())
            .collect()
    }

    pub fn get(&self, sim_id: &SimId) -> Option<&PlannedEntity<D>> {
        self.entities.iter().find(|entity| &entity.sim_id == sim_id)
    }

    /// Grow a seed set until no planned relation crosses its boundary.
    ///
    /// This is the set a caller must despawn and rebuild together for the
    /// result to be correctly wired, and it is what makes
    /// [`ConstructionError::RelationCutBySubset`] a solvable refusal rather
    /// than a dead end: ask for the closure, rebuild that.
    ///
    /// Relations are undirected for this purpose. Rebuilding a target strands
    /// its sources just as surely as rebuilding a source leaves it unwired,
    /// because both sides of the wiring are `Entity` handles minted by the
    /// commit that built them.
    pub fn relation_closure(&self, seeds: &BTreeSet<SimId>) -> BTreeSet<SimId> {
        let mut closed = seeds.clone();
        // Each pass can only add, and the plan is finite, so this terminates in
        // at most one pass per relation.
        loop {
            let mut grew = false;
            for relation in &self.relations {
                let has_from = closed.contains(&relation.from);
                let has_to = closed.contains(&relation.to);
                if has_from != has_to {
                    closed.insert(if has_from {
                        relation.to.clone()
                    } else {
                        relation.from.clone()
                    });
                    grew = true;
                }
            }
            if !grew {
                return closed;
            }
        }
    }

    /// Construct one planned entity through its frozen recipe.
    ///
    /// Reconstruction of a single entity. Refuses — before mutating — if this
    /// row sits at EITHER end of a planned relation, because rebuilding one end
    /// alone strands the other on a dead `Entity` handle; see
    /// [`ConstructionError::RelationCutBySubset`] and
    /// [`ConstructionPlan::relation_closure`].
    pub fn construct_one(
        &self,
        sim_id: &SimId,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Result<Entity, ConstructionError> {
        let subset = BTreeSet::from([sim_id.clone()]);
        let receipt = self.commit_subset(&subset, ctx)?;
        Ok(receipt
            .entity(sim_id)
            .unwrap_or_else(|| unreachable!("a one-row commit that succeeded committed its row")))
    }

    /// Construct every planned entity, then wire every planned relation.
    ///
    /// Relations run second because a relation names identities, and an
    /// identity has no entity until its row has been committed. That ordering
    /// is what lets a plan express a mutual pair (two duellists grudging each
    /// other) without either row needing the other to exist first.
    pub fn commit(&self, ctx: &mut ConstructionExecCtx<'_, '_, '_, D>) -> ConstructionReceipt {
        self.commit_visibility(ctx, false)
    }

    /// [`Self::commit`], but every root is stamped [`InactiveCandidate`] AT
    /// MINT — built, wired and invisible to ordinary queries until
    /// [`publish_candidate`] admits the transaction.
    ///
    /// ⭐⭐ **THIS IS THE DEFERRED-`Commands` SHAPE OF
    /// [`Self::commit_inactive`], and the room road needs it because it does not
    /// have `&mut World`.** `commit_inactive` takes a world so it can ask
    /// [`inactive_candidate_filter_installed`] and REFUSE; this cannot ask, so
    /// **the caller owes that check** — which is why it is not the default and
    /// why its one production caller is a room transaction that registers the
    /// filter itself.
    ///
    /// ⛔ Without the filter the marker is inert and every "candidate" is LIVE,
    /// which is the dangerous direction. See [`InactiveCandidate`].
    pub fn commit_hidden(
        &self,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> ConstructionReceipt {
        self.commit_visibility(ctx, true)
    }

    fn commit_visibility(
        &self,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
        hidden: bool,
    ) -> ConstructionReceipt {
        self.execute(None, ctx, hidden).unwrap_or_else(|error| {
            unreachable!(
                "committing a plan in full names only its own rows and encloses every relation, \
                 so it cannot be refused: {error}"
            )
        })
    }

    /// Commit every row as an INACTIVE CANDIDATE: constructed, wired, and
    /// invisible to ordinary queries until [`publish_candidate`] admits it.
    ///
    /// ⭐⭐ **THIS IS THE STRONGER LAST-GOOD-WORLD GUARANTEE (`Q113`, ruled
    /// 2026-09-12).** The running world is not disturbed to make room for a
    /// candidate: the candidate is built beside it and only becomes visible when
    /// construction has already SUCCEEDED. A candidate that turns out invalid is
    /// retired with [`retire_candidate`] and the live world never knew about it.
    ///
    /// ```text
    /// last-good world N
    ///     ├── commit_inactive  → candidate N+1 exists, unseen
    ///     ├── validate         → refuse → retire_candidate, N untouched
    ///     └── publish_candidate → N+1 visible, then retire N
    /// ```
    ///
    /// ⛔ **IT REFUSES RATHER THAN COMMITTING WHEN THE FILTER IS NOT INSTALLED**,
    /// and that direction is deliberate. An unregistered [`InactiveCandidate`] is
    /// an ordinary inert component: every root would be stamped and every root
    /// would be LIVE, so the failure would be a candidate silently participating
    /// in the running world — the exact outcome this method exists to prevent.
    /// A refusal is loud and costs nothing; the alternative is invisible.
    ///
    /// ⚠ THE ROOTS ARE STAMPED AFTER THE ROWS ARE BUILT, not during. A recipe
    /// runs against the same context it always does and cannot tell whether it
    /// is building a candidate — which is what keeps ONE executor rather than a
    /// candidate-aware fork of every recipe.
    pub fn commit_inactive(
        &self,
        world: &mut World,
        session: crate::lifecycle::SessionSpawnScope,
        services: &D::Services,
    ) -> Result<ConstructionReceipt, InactiveCommitRefused> {
        if !inactive_candidate_filter_installed(world) {
            return Err(InactiveCommitRefused::FilterNotInstalled);
        }
        let receipt = {
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let mut commands = Commands::new(&mut queue, world);
            let mut ctx = ConstructionExecCtx {
                commands: &mut commands,
                scope: &self.scope,
                session,
                services,
            };
            // ⛔⛤ **HIDDEN AT MINT, NOT AFTER THE FLUSH.** This used to call
            // `commit`, apply the queue — making every candidate entity real and
            // visible — and only then loop inserting the marker. Systems cannot
            // observe that window (nothing is scheduled inside an exclusive
            // world call), but component HOOKS and lifecycle OBSERVERS fire
            // during `queue.apply` and are not systems. `execute(.., true)`
            // stamps the marker in the same batch as the root's identity, before
            // the recipe runs.
            let receipt = self
                .execute(None, &mut ctx, true)
                .unwrap_or_else(|error| {
                    unreachable!("a full commit cannot cut a relation: {error}")
                });
            queue.apply(world);
            receipt
        };
        Ok(receipt)
    }

    /// Construct the named rows, and wire exactly the relations that lie wholly
    /// within them.
    ///
    /// ⭐ THERE IS ONE EXECUTOR AND IT IS PRIVATE: [`Self::execute`]. This and
    /// [`Self::commit`] are its only two shapes — `commit` is `execute(None)`,
    /// this is `execute(Some(ids))` — so a full commit is the same code over
    /// every row and a single-entity rebuild is the same code over one.
    /// Ordinary construction and reconstruction cannot drift because there is
    /// nothing for them to drift between.
    ///
    /// ⚠ This line used to read "this is the only executor", sitting here on
    /// `commit_subset`. The STRUCTURE it describes is real and unchanged; the
    /// sentence named the wrong subject, and an authority claim that points at
    /// one of two callers invites someone to add a third beside it.
    ///
    /// A subset containing exactly ONE end of a planned relation is such a refusal, in either
    /// direction: rebuilding the source alone leaves it unwired, and rebuilding the target
    /// alone leaves the untouched source holding a handle to the entity that just died. See
    /// [`ConstructionError::RelationCutBySubset`], and [`ConstructionPlan::relation_closure`]
    /// for the set that cannot be cut.
    pub fn commit_subset(
        &self,
        ids: &BTreeSet<SimId>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
    ) -> Result<ConstructionReceipt, ConstructionError> {
        self.execute(Some(ids), ctx, false)
    }

    /// `None` means every row — which is why a full commit allocates nothing to
    /// describe itself and skips validation that cannot fail. Naming the whole
    /// roster explicitly would clone every `SimId` in the plan, and a
    /// reconstruction sweep calling `construct_one` per entity would pay that
    /// once per entity.
    /// `hidden` stamps [`InactiveCandidate`] on each root AT MINT, in the same
    /// command batch as its identity and provenance.
    ///
    /// ⛔⛤ **`commit_inactive` USED TO STAMP AFTERWARDS, AND THAT IS A WINDOW.**
    /// It applied the whole command queue — at which point every candidate
    /// entity is REAL and VISIBLE — and only then looped inserting the marker.
    /// No scheduled system runs inside an exclusive `&mut World` call, so the
    /// window is invisible to systems; **component hooks and lifecycle observers
    /// are not systems and fire during `queue.apply`.** The old comment on
    /// [`InactiveCandidate`] admitted the residual surface was *"a POPULATION
    /// FACT, not a boundary"* — this makes it a boundary for the one thing it
    /// can: the marker is on the root before anything else is.
    fn execute(
        &self,
        subset: Option<&BTreeSet<SimId>>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
        hidden: bool,
    ) -> Result<ConstructionReceipt, ConstructionError> {
        let included = |id: &SimId| subset.is_none_or(|ids| ids.contains(id));
        if let Some(ids) = subset {
            if let Some(missing) = ids
                .iter()
                .find(|id| !self.entities.iter().any(|e| &e.sim_id == *id))
            {
                return Err(ConstructionError::NotInPlan {
                    sim_id: missing.clone(),
                });
            }
            // A relation must be wholly in or wholly out. Cutting it either way
            // strands an `Entity` handle — see `RelationCutBySubset`.
            for relation in &self.relations {
                if ids.contains(&relation.from) != ids.contains(&relation.to) {
                    return Err(ConstructionError::RelationCutBySubset {
                        from: relation.from.clone(),
                        kind: relation.kind.clone(),
                        to: relation.to.clone(),
                    });
                }
            }
        }

        let mut receipt = ConstructionReceipt::default();
        for planned in self.entities.iter().filter(|e| included(&e.sim_id)) {
            let entity = self.commit_entity(planned, ctx, hidden);
            receipt.committed.insert(planned.sim_id.clone(), entity);
        }
        for relation in self.relations.iter().filter(|r| included(&r.from)) {
            // It must not be swallowed.
            let (Some(from), Some(to)) = (
                receipt.committed.get(&relation.from).copied(),
                receipt.committed.get(&relation.to).copied(),
            ) else {
                unreachable!(
                    "planned relation {} -> {} names an identity this commit did not build",
                    relation.from, relation.to
                )
            };
            // ⛔ THE RELATION GETS ITS TWO ENDPOINTS AND NOTHING ELSE. See
            // `RelationScope`.
            let mut relation_scope = RelationScope {
                commands: &mut *ctx.commands,
                from,
                to,
                scope: ctx.scope,
                session: ctx.session,
                services: ctx.services,
            };
            (relation.ops.wire)(&relation.relation, &mut relation_scope);
            receipt.relations_wired.insert((
                relation.from.clone(),
                relation.kind.clone(),
                relation.to.clone(),
            ));
        }
        Ok(receipt)
    }

    /// Allocate this row's authoritative root, stamp it, and hand it to the
    /// domain to populate.
    ///
    /// That guard was weak in three ways a redesign removes rather than patches: a pre-existing
    /// entity WITHOUT a `SimId` passed it and was silently commandeered; the check ran at flush, so
    /// it was a panic after other rows had queued their mutations rather than a refusal; and
    /// nothing tied the returned entity to this commit at all.
    ///
    /// Allocating here makes freshness structural. `spawn_empty` yields an
    /// entity that by definition nothing else holds, so one planned row is one
    /// distinct new root, and there is no check to get wrong.
    fn commit_entity(
        &self,
        planned: &PlannedEntity<D>,
        ctx: &mut ConstructionExecCtx<'_, '_, '_, D>,
        hidden: bool,
    ) -> Entity {
        let root = ctx.commands.spawn_empty().id();
        // Identity, provenance, and transaction ownership go on before the
        // recipe runs, so a recipe cannot forget them and reconstruction never
        // sees a body without provenance. A recipe that inspects its own root
        // finds them already there. The ownership stamp is what lets
        // verification enumerate this transaction's roots from the world.
        ctx.commands.entity(root).insert((
            planned.sim_id.clone(),
            planned.origin.clone(),
            self.transaction(ctx.session),
        ));
        // ⛔ BEFORE THE RECIPE RUNS AND IN THE SAME BATCH AS THE STAMPS ABOVE,
        // so the root is never briefly a visible member of the live world. See
        // `execute`'s `hidden`.
        if hidden {
            ctx.commands.entity(root).insert(InactiveCandidate);
        }
        // The constructor preparation resolved — NOT a fresh dispatch. A domain
        // whose `dispatch` reads mutable state would otherwise let commit run a
        // different constructor than the one the plan validated and dumped.
        // ⭐ THE RECIPE GETS A ROOT-BOUND SURFACE, NOT THE EXECUTOR'S CONTEXT.
        // See `ConstructionRootCtx`: there is no `Commands` in it, so a recipe
        // cannot spawn an authoritative entity the candidate isolation would
        // miss.
        let mut root_ctx = ConstructionRootCtx {
            root,
            commands: ctx.commands,
            scope: ctx.scope,
            session: ctx.session,
            services: ctx.services,
        };
        (planned.construct)(&planned.parameters, &mut root_ctx);
        root
    }

    /// Byte-stable inspection surface, in the same tab-delimited shape as
    /// `PreparedContent::deterministic_dump`. Two plans over equivalent input
    /// produce identical bytes regardless of request order.
    pub fn deterministic_dump(&self) -> String {
        use std::fmt::Write as _;
        let mut out = format!(
            // ⛔⛤ **BOTH BINDINGS, AND THE SECOND LINE IS WHY THE SCHEMA MOVED
            // TO 5.** This rendered ONE binding. Two plans that carry the same
            // incoming generation into DIFFERENT live worlds are different plans
            // — they will be compared against different boundaries — and a dump
            // that rendered them identically calls them the same plan, which is
            // what the prefetch cache keys on. ⚠ For every non-replacement road
            // the two are equal, so this is a new line rather than a new value.
            "construction-plan-v{CONSTRUCTION_PLAN_SCHEMA_VERSION}\n{}\nexpects\t{}\nroom\t{}\nlane\t{}\n",
            self.scope.incoming.canonical_summary(),
            self.scope.expected_live.canonical_summary(),
            self.scope.room.as_deref().unwrap_or("-"),
            self.lane.as_str(),
        );
        for entity in &self.entities {
            // No separate parent column: `canonical_summary` already carries it
            // for the one origin that has one, and printing it twice would let
            // a dump disagree with itself.
            let _ = writeln!(
                out,
                "entity\t{}\t{}\t{}\t{}",
                entity.sim_id,
                entity.recipe,
                entity.origin.canonical_summary(),
                D::canonical_summary(&entity.parameters),
            );
        }
        for relation in &self.relations {
            let _ = writeln!(
                out,
                "relation\t{}\t{}\t{}\t{}",
                relation.from,
                relation.kind,
                relation.to,
                D::canonical_relation_summary(&relation.relation),
            );
        }
        out
    }
}

/// Check that the world a commit produced is the world the plan described.
///
/// This is a detector, not a preventer, and the distinction is the whole
/// point of having it. The executor allocates each row's root, but a recipe
/// receives raw `Commands` and the root `Entity`, so it can despawn that root,
/// strip or rewrite its `SimId`/`SpawnOrigin`, stamp a second entity with a
/// planned identity, or spawn further authoritative entities of its own. None of
/// that is structurally prevented today, so a transaction that intends to
/// publish a room must ask.
///
/// ⛔⛤ **THIS USED TO ADD *"the giant hand limbs already do the last of these"*
/// AND THAT IS NO LONGER TRUE — re-derived 2026-09-12.** The hands are PLAN ROWS
/// now (`giant_hand_plans` feeds `giant_cluster_rows`), and measured across the
/// whole tree there is **not one `ctx.commands.spawn` in a production recipe** —
/// every occurrence is test code. ⇒ The recipe-spawned authoritative root is a
/// shape this function must still detect, and it currently has **no production
/// instance**. That distinction matters for A10: with no recipe minting its own
/// roots, `commit_inactive` stamping every PLANNED root isolates every candidate
/// this engine actually builds.
///
/// Bevy commands do not roll back. By the time this can run, the
/// construction commands have applied.
///
/// ⭐⭐ **AND THAT USED TO MEAN A VIOLATION COULD NOT BE UNDONE — IT DOES NOT ANY
/// MORE FOR A CANDIDATE (A10, 2026-09-12).** This paragraph read *"a violation
/// here cannot be undone … and leaves the world in whatever state the offending
/// recipe produced"*, which is still exactly true of
/// [`ConstructionPlan::commit`] against the live world. It is NOT true of
/// [`ConstructionPlan::commit_inactive`]: the offending state is a candidate no
/// ordinary query can see, so [`retire_candidate`] removes it and the running
/// world never knew. ⇒ **The limitation this comment described is what the
/// stronger last-good-world guarantee exists to dissolve**, and the difference
/// between the two commits is exactly whether a detector's findings are
/// actionable.
///
/// ⚠ Against a LIVE commit the old sentence stands: better than publishing a room
/// nobody can describe, worse than the structural fix (every authoritative root
/// an explicit plan row).
///
/// The scope is read from the world, not supplied. An earlier version took
/// a caller-curated `&[(SimId, Entity)]`, which made the check exactly as
/// complete as the caller's imagination: the roots most worth catching are the
/// ones nobody thought to list. [`AuthoritativeScope::gather`] queries instead,
/// and treats an unclassified identity-bearing entity as a finding rather than
/// as absent.
pub fn verify_committed_roster<D: ConstructionDomain>(
    plan: &ConstructionPlan<D>,
    receipt: &ConstructionReceipt,
    baseline: &TransactionBaseline,
    scope: &AuthoritativeScope,
    world: &World,
) -> Result<(), Vec<RosterViolation>> {
    let mut violations = Vec::new();
    let live = |entity: Entity| world.get_entity(entity).is_ok();

    // Presentation-only entities are excluded by classification, not by their spelling.
    let mut occupants: BTreeMap<&SimId, Vec<Entity>> = BTreeMap::new();
    for member in scope.members() {
        if member.classification != ScopeClassification::PresentationOnly {
            occupants
                .entry(&member.sim_id)
                .or_default()
                .push(member.entity);
        }
    }
    let occupants_of = |sim_id: &SimId| occupants.get(sim_id).map_or(&[][..], Vec::as_slice);
    // ⛔⛤ **A DECLARED SUPERSESSION'S LIVE BODY IS NOT PART OF THE ROSTER BEING
    // VERIFIED.** It is the world the candidate is being validated AGAINST, and
    // it is standing on purpose. Counting it as an occupant reports the A10
    // invariant working as `Duplicated` — which is precisely what the measured
    // `death_restores_the_checkpoint` refusal was.
    //
    // ⚠ It is subtracted HERE, once, rather than branched around at each of the
    // three places occupants are counted: two spellings of "except the
    // predecessor" is how one of them comes to disagree.
    let candidate_occupants_of = |sim_id: &SimId| -> Vec<Entity> {
        let superseded = baseline.superseded_body(sim_id);
        occupants_of(sim_id)
            .iter()
            .copied()
            .filter(|entity| Some(*entity) != superseded)
            .collect()
    };

    let planned_ids = plan.planned_ids();

    // ── Baseline preservation ────────────────────────────────────────────────
    //
    // Every identity that was live when the transaction opened, and that the transaction did not
    // declare it was retiring or reconstructing, must come out the far side untouched: same
    // identity, one occupant, the SAME entity it started on, and the same provenance.
    for (sim_id, entry) in baseline.entries() {
        if baseline.is_retired(sim_id) {
            if !occupants_of(sim_id).is_empty() {
                violations.push(RosterViolation::RetiredSurvived {
                    sim_id: sim_id.clone(),
                });
            }
            continue;
        }
        if baseline.is_reconstructed(sim_id) {
            // The declared-reconstruction contract: the old body is gone, the
            // new one is the receipt's, and there is exactly one of it.
            if live(entry.entity) {
                violations.push(RosterViolation::ReconstructedOldSurvived {
                    sim_id: sim_id.clone(),
                    stale: entry.entity,
                });
            }
            match occupants_of(sim_id) {
                [] => violations.push(RosterViolation::Missing {
                    sim_id: sim_id.clone(),
                }),
                [found] => {
                    if receipt.entity(sim_id) != Some(*found) {
                        violations.push(RosterViolation::MovedRoot {
                            sim_id: sim_id.clone(),
                        });
                    }
                }
                found => violations.push(RosterViolation::Duplicated {
                    sim_id: sim_id.clone(),
                    count: found.len(),
                }),
            }
            continue;
        }
        if baseline.is_superseded(sim_id) {
            // ⛔ THE DECLARED-SUPERSESSION CONTRACT, AND IT IS THE MIRROR OF THE
            // ONE ABOVE: the old body MUST still be alive, and the candidate
            // must exist. What publishing would leave behind is not askable
            // here — this world still holds both on purpose — so
            // `verify_projected_roster` owns that half.
            if !live(entry.entity) {
                violations.push(RosterViolation::SupersededLiveLost {
                    sim_id: sim_id.clone(),
                    expected: entry.entity,
                });
            }
            // `receipt.entity` is `None` when this identity belongs to a
            // DIFFERENT construction lane of the same room; the room
            // transaction verifies every lane, each against its own receipt,
            // so a missing row here means "not mine", not "not built".
            if let Some(root) = receipt.entity(sim_id) {
                if !live(root) {
                    violations.push(RosterViolation::SupersededCandidateMissing {
                        sim_id: sim_id.clone(),
                    });
                }
            } else if planned_ids.contains(sim_id) && candidate_occupants_of(sim_id).is_empty() {
                violations.push(RosterViolation::SupersededCandidateMissing {
                    sim_id: sim_id.clone(),
                });
            }
            continue;
        }
        // Not retired, not reconstructed, not superseded: preserved.
        if planned_ids.contains(sim_id) {
            violations.push(RosterViolation::PlannedOverBaseline {
                sim_id: sim_id.clone(),
            });
        }
        match occupants_of(sim_id) {
            [] => violations.push(RosterViolation::BaselineLost {
                sim_id: sim_id.clone(),
            }),
            [found] if *found == entry.entity => {
                let now = world.get::<SpawnOrigin>(entry.entity).cloned();
                if now != entry.origin {
                    violations.push(RosterViolation::BaselineProvenanceChanged {
                        sim_id: sim_id.clone(),
                        expected: entry.origin.clone(),
                        found: now,
                    });
                }
            }
            // The identity survived on a DIFFERENT entity: something despawned
            // the baseline body and minted a replacement wearing its name. A
            // set comparison sees a perfectly intact roster here.
            [found] => violations.push(RosterViolation::BaselineReplaced {
                sim_id: sim_id.clone(),
                expected: entry.entity,
                found: *found,
            }),
            found => violations.push(RosterViolation::Duplicated {
                sim_id: sim_id.clone(),
                count: found.len(),
            }),
        }
    }

    // ── Planned rows ─────────────────────────────────────────────────────────
    for planned in plan.entities() {
        let expected_root = receipt.entity(&planned.sim_id);
        if expected_root.is_none() {
            // Not part of this commit's subset; its relations are skipped below
            // for the same reason.
            continue;
        }
        match candidate_occupants_of(&planned.sim_id).as_slice() {
            [] => violations.push(RosterViolation::Missing {
                sim_id: planned.sim_id.clone(),
            }),
            [found] => {
                if expected_root != Some(*found) {
                    violations.push(RosterViolation::MovedRoot {
                        sim_id: planned.sim_id.clone(),
                    });
                }
            }
            found => violations.push(RosterViolation::Duplicated {
                sim_id: planned.sim_id.clone(),
                count: found.len(),
            }),
        }
        if let Some(root) = expected_root {
            if !live(root) {
                violations.push(RosterViolation::Missing {
                    sim_id: planned.sim_id.clone(),
                });
            } else {
                let found = world.get::<SpawnOrigin>(root).cloned();
                if found.as_ref() != Some(&planned.origin) {
                    violations.push(RosterViolation::ProvenanceChanged {
                        sim_id: planned.sim_id.clone(),
                        expected: planned.origin.clone(),
                        found,
                    });
                }
                // Ownership is stamped in the same insert as identity and
                // provenance, so checking those two and not this one left the
                // stamp that DRIVES scope classification as the only part of the
                // executor's mark nothing confirmed.
                let owner = world.get::<TransactionId>(root);
                if owner != Some(scope.transaction()) {
                    violations.push(RosterViolation::OwnershipLost {
                        sim_id: planned.sim_id.clone(),
                        expected: scope.transaction().clone(),
                        found: owner.cloned(),
                    });
                }
            }
        }
    }

    // ── Everything else the world holds in this scope ────────────────────────
    for member in scope.members() {
        if member.classification == ScopeClassification::PresentationOnly {
            continue;
        }
        if planned_ids.contains(&member.sim_id) || baseline.contains(&member.sim_id) {
            continue;
        }
        violations.push(match &member.classification {
            // Stamped by THIS transaction's executor, yet no plan row named it.
            ScopeClassification::TransactionAuthoritative => RosterViolation::Unplanned {
                sim_id: member.sim_id.clone(),
            },
            // Identity-bearing, appeared during this transaction, and nothing in
            // the world says what built it. Every production family is planned.
            ScopeClassification::Unowned => RosterViolation::UnownedIdentity {
                sim_id: member.sim_id.clone(),
            },
            ScopeClassification::ForeignScope(_) | ScopeClassification::PresentationOnly => {
                continue
            }
        });
    }

    // ── Relation postconditions ──────────────────────────────────────────────
    //
    // The receipt records that a wiring function was CALLED. That is a fact
    // about the executor, not about the world: a wiring function that does
    // nothing, writes to the wrong entity, or is overwritten by a later command
    // produces an identical receipt. Each relation's frozen verifier reads the
    // committed components instead.
    for relation in plan.relations() {
        let key = (
            relation.from.clone(),
            relation.kind.clone(),
            relation.to.clone(),
        );
        // Which relations this commit OWED is derived from the identities it
        // actually committed, not from the receipt's own account of what it did.
        // A subset commit encloses every relation it touches (see
        // `RelationCutBySubset`), so both-endpoints-committed is exactly "this
        // relation was in scope" — for a full commit that is every planned
        // relation, and for a subset it is the ones wholly inside it.
        let (Some(from), Some(to)) = (receipt.entity(&relation.from), receipt.entity(&relation.to))
        else {
            continue;
        };
        if !receipt.relations_wired().contains(&key) {
            violations.push(RosterViolation::RelationMissingFromReceipt {
                from: relation.from.clone(),
                kind: relation.kind.clone(),
                to: relation.to.clone(),
            });
            continue;
        }
        if !live(from) || !live(to) {
            violations.push(RosterViolation::DanglingRelation {
                from: relation.from.clone(),
                kind: relation.kind.clone(),
                to: relation.to.clone(),
            });
            continue;
        }
        let check = (relation.ops.verify)(world, from, to, &relation.relation);
        if check != RelationCheck::Installed {
            violations.push(RosterViolation::RelationNotEstablished {
                from: relation.from.clone(),
                kind: relation.kind.clone(),
                to: relation.to.clone(),
                expected: to,
                check,
            });
        }
    }

    violations.sort_by_key(|violation| format!("{violation:?}"));
    violations.dedup();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// How a committed plan failed to match the world it was supposed to build.
///
/// Structured rather than logged, because the caller's correct response is to
/// refuse the transaction, not to carry on with a world it cannot describe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RosterViolation {
    /// A planned row produced no live entity carrying its identity.
    Missing { sim_id: SimId },
    /// More than one live entity carries one identity. This is the case a
    /// `BTreeSet<SimId>` comparison cannot see: the set of identities looks
    /// exactly right while two bodies answer to one of them.
    Duplicated { sim_id: SimId, count: usize },
    /// The identity exists, but not on the entity the executor allocated for
    /// it — the recipe moved it, or despawned the root and rebuilt elsewhere.
    MovedRoot { sim_id: SimId },
    /// The root lost or had rewritten the provenance the executor stamped.
    ProvenanceChanged {
        sim_id: SimId,
        expected: SpawnOrigin,
        found: Option<SpawnOrigin>,
    },
    /// A transaction-scoped authoritative root exists that no plan row named.
    /// Recipes that create authoritative entities internally land here.
    Unplanned { sim_id: SimId },
    /// An identity-bearing entity appeared during this transaction carrying no ownership stamp
    /// and no explicit classification.
    UnownedIdentity { sim_id: SimId },
    /// A planned root does not carry this transaction's ownership stamp.
    ///
    /// An unowned planned root is invisible to the next transaction's scope gathering, so it
    /// would be counted as somebody else's problem forever.
    OwnershipLost {
        sim_id: SimId,
        expected: TransactionId,
        found: Option<TransactionId>,
    },
    /// A planned relation whose two endpoints were both committed does not
    /// appear in the receipt: the executor never wired it.
    ///
    /// Skipped silently before, which made the relation postcondition pass
    /// vacuous for exactly the relations that failed hardest — a relation the
    /// executor never attempted has no receipt entry, so "verify the ones that
    /// were wired" verified everything except the broken one.
    RelationMissingFromReceipt {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// A baseline identity the transaction did not declare it was touching is
    /// no longer in the world.
    BaselineLost { sim_id: SimId },
    /// A baseline identity survived, on a different entity. Something
    /// despawned the original and minted a replacement wearing its name. The
    /// roster is exactly the right length and every identity is present, which
    /// is why identity-only comparison is not enough.
    BaselineReplaced {
        sim_id: SimId,
        expected: Entity,
        found: Entity,
    },
    /// An untouched baseline entity's provenance was rewritten under it.
    ///
    /// Separate from [`Self::ProvenanceChanged`] because a baseline entity may
    /// legitimately carry none — a persistent player is not a construction
    /// product — so `expected` is an `Option` here and is not one there.
    BaselineProvenanceChanged {
        sim_id: SimId,
        expected: Option<SpawnOrigin>,
        found: Option<SpawnOrigin>,
    },
    /// An identity the transaction declared it was retiring is still present.
    RetiredSurvived { sim_id: SimId },
    /// A declared reconstruction left the pre-reconstruction body alive, so two
    /// generations of one identity coexist and dependants may hold either.
    ReconstructedOldSurvived { sim_id: SimId, stale: Entity },
    /// A plan row names an identity that was already live and was not declared
    /// a reconstruction, so committing it creates a second body for it.
    PlannedOverBaseline { sim_id: SimId },
    /// A declared SUPERSESSION destroyed the live body it was supposed to leave
    /// standing.
    ///
    /// ⛔ The A10 invariant stated as a violation: a candidate is prepared BESIDE
    /// the playable world, and a candidate that despawns its own predecessor has
    /// already spent the world it was being validated against, whatever the
    /// verdict turns out to be.
    SupersededLiveLost { sim_id: SimId, expected: Entity },
    /// A declared supersession built no candidate for the identity it named, so
    /// publishing it would leave the identity with nothing.
    SupersededCandidateMissing { sim_id: SimId },
    /// A wired relation names an entity that is not live.
    DanglingRelation {
        from: SimId,
        kind: RelationKind,
        to: SimId,
    },
    /// The wiring function ran and the world does not hold the relation.
    ///
    /// The receipt cannot see this. It records that a function was called,
    /// which a no-op, a write to the wrong entity, and a later overwrite all
    /// satisfy identically.
    RelationNotEstablished {
        from: SimId,
        kind: RelationKind,
        to: SimId,
        expected: Entity,
        check: RelationCheck,
    },
    /// The plan was prepared against a content generation that is not the one the session is
    /// live under. Fatal. Detected at the room boundary because commit cannot be prevented
    /// yet (no staging world); the room is refused publication instead.
    ContentBindingMismatch {
        planned: ContentBinding,
        live: ContentBinding,
    },
    /// A host's committed rig does not match the exact composition the plan
    /// described for it.
    ///
    /// The per-relation postcondition pass cannot see this. That pass asks,
    /// for each planned limb relation, "did MY relation land" — a question a host
    /// whose rig gained an EXTRA limb from a second intent stream answers yes to
    /// for every planned limb while carrying a rig the plan never described. This
    /// is the composition check: exactly the planned slots, each holding the
    /// planned identity, nothing more. `detail` names the specific fault (an
    /// extra slot, a missing slot, a wrong or duplicated occupant, a reverse
    /// disagreement); the domain that owns the rig components composes it, so the
    /// primitive stays free of `LimbRig`/`LimbSlot`. Fatal.
    RigComposition { host: SimId, detail: String },
}

impl std::fmt::Display for RosterViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { sim_id } => {
                write!(f, "planned identity `{sim_id}` is not in the world")
            }
            Self::Duplicated { sim_id, count } => write!(
                f,
                "planned identity `{sim_id}` is on {count} entities; exactly one was expected"
            ),
            Self::MovedRoot { sim_id } => write!(
                f,
                "planned identity `{sim_id}` is not on the root the executor allocated for it"
            ),
            Self::ProvenanceChanged {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "`{sim_id}` should carry provenance `{}` but carries `{}`",
                expected.canonical_kind(),
                found.as_ref().map_or("none", SpawnOrigin::canonical_kind),
            ),
            Self::Unplanned { sim_id } => write!(
                f,
                "authoritative identity `{sim_id}` exists in this transaction but no plan row \
                 named it"
            ),
            Self::UnownedIdentity { sim_id } => write!(
                f,
                "`{sim_id}` appeared during this transaction carrying no construction ownership \
                 and no classification: nothing in the world says what built it"
            ),
            Self::OwnershipLost {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "planned root `{sim_id}` should be owned by transaction `{expected}` but carries \
                 `{}`",
                found.as_ref().map_or("none", TransactionId::as_str),
            ),
            Self::RelationMissingFromReceipt { from, kind, to } => write!(
                f,
                "planned relation `{from}` -`{kind}`-> `{to}` has both endpoints committed but \
                 the executor never wired it"
            ),
            Self::BaselineLost { sim_id } => write!(
                f,
                "`{sim_id}` was live when this transaction opened and this transaction did not \
                 declare it was touching it, but it is gone"
            ),
            Self::BaselineReplaced {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "`{sim_id}` was live on {expected:?} and is now on {found:?}: the original was \
                 replaced by a different entity wearing its identity"
            ),
            Self::BaselineProvenanceChanged {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "`{sim_id}` was untouched by this transaction but its provenance changed from \
                 `{}` to `{}`",
                expected
                    .as_ref()
                    .map_or("none", SpawnOrigin::canonical_kind),
                found.as_ref().map_or("none", SpawnOrigin::canonical_kind),
            ),
            Self::RetiredSurvived { sim_id } => write!(
                f,
                "`{sim_id}` was declared retired by this transaction but is still in the world"
            ),
            Self::ReconstructedOldSurvived { sim_id, stale } => write!(
                f,
                "`{sim_id}` was reconstructed but its previous body {stale:?} is still alive, so \
                 two generations of one identity coexist"
            ),
            Self::PlannedOverBaseline { sim_id } => write!(
                f,
                "`{sim_id}` is a plan row and was already live, without being declared a \
                 reconstruction"
            ),
            Self::SupersededLiveLost { sim_id, expected } => write!(
                f,
                "`{sim_id}` was declared superseded — its live body {expected:?} had to stand \
                 until publication — and construction destroyed it"
            ),
            Self::SupersededCandidateMissing { sim_id } => write!(
                f,
                "`{sim_id}` was declared superseded and this transaction built no candidate \
                 to take its place"
            ),
            Self::DanglingRelation { from, kind, to } => write!(
                f,
                "wired relation `{from}` -`{kind}`-> `{to}` names an entity that is not live"
            ),
            Self::RelationNotEstablished {
                from,
                kind,
                to,
                expected,
                check,
            } => write!(
                f,
                "relation `{from}` -`{kind}`-> `{to}` was wired but the world does not hold it \
                 onto {expected:?}: {check:?}"
            ),
            Self::RigComposition { host, detail } => write!(
                f,
                "host `{host}` committed a rig that does not match its planned composition: {detail}"
            ),
            Self::ContentBindingMismatch { planned, live } => write!(
                f,
                "the plan was prepared against `{}` but the session is live under `{}`",
                planned.canonical_summary(),
                live.canonical_summary(),
            ),
        }
    }
}

impl std::error::Error for RosterViolation {}

/// What one baseline identity was sitting on when the transaction opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineEntry {
    /// The exact entity.
    pub entity: Entity,
    /// Its provenance at capture, so a transaction that quietly rewrites an
    /// untouched entity's origin is a finding rather than a surprise later.
    pub origin: Option<SpawnOrigin>,
}

/// The world a transaction opened against: which identities were live, on
/// which entities, carrying which provenance — plus what the transaction
/// declared it was going to do to them.
///
/// Explicit rather than inferred: nothing here parses a `SimId`. And permission
/// to remove or replace an identity is *declared*, never deduced from the
/// candidate plan — inferring it would mean any plan naming an identity thereby
/// authorised destroying whatever already held it, which is the opposite of a
/// check.
#[derive(Clone, Debug, Default)]
pub struct TransactionBaseline {
    entries: BTreeMap<SimId, BaselineEntry>,
    retired: BTreeSet<SimId>,
    reconstructed: BTreeSet<SimId>,
    superseded: BTreeSet<SimId>,
}

/// Why a baseline could not be captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaselineCaptureError {
    DuplicateIdentity {
        sim_id: SimId,
        entities: Vec<Entity>,
    },
}

impl std::fmt::Display for BaselineCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateIdentity { sim_id, entities } => write!(
                f,
                "identity `{sim_id}` is already on {} entities before this transaction began: \
                 {entities:?}",
                entities.len()
            ),
        }
    }
}

impl std::error::Error for BaselineCaptureError {}

impl TransactionBaseline {
    /// Capture every AUTHORITATIVE identity-bearing entity in the world, with
    /// its entity and provenance. Duplicates are a refusal, not a merge.
    ///
    /// [`PresentationOnly`] entities are excluded, and must be: verification
    /// counts occupants with the same exclusion, so a presentation-only entity
    /// admitted here would be looked for among occupants that structurally
    /// cannot contain it and reported as [`RosterViolation::BaselineLost`] every
    /// time. The two filters are the same filter or the baseline is measuring
    /// something the verifier is not.
    pub fn capture(world: &mut World) -> Result<Self, BaselineCaptureError> {
        let mut found: BTreeMap<SimId, Vec<(Entity, Option<SpawnOrigin>)>> = BTreeMap::new();
        let mut query = world.query_filtered::<
            (Entity, &SimId, Option<&SpawnOrigin>),
            bevy::prelude::Without<PresentationOnly>,
        >();
        for (entity, sim_id, origin) in query.iter(world) {
            found
                .entry(sim_id.clone())
                .or_default()
                .push((entity, origin.cloned()));
        }
        Self::from_occupants(found)
    }

    /// Capture from explicit pairs, for fixtures and for callers that already
    /// hold the roster. Duplicates refuse exactly as they do in [`Self::capture`].
    pub fn from_pairs(
        pairs: impl IntoIterator<Item = (SimId, Entity, Option<SpawnOrigin>)>,
    ) -> Result<Self, BaselineCaptureError> {
        let mut found: BTreeMap<SimId, Vec<(Entity, Option<SpawnOrigin>)>> = BTreeMap::new();
        for (sim_id, entity, origin) in pairs {
            found.entry(sim_id).or_default().push((entity, origin));
        }
        Self::from_occupants(found)
    }

    fn from_occupants(
        found: BTreeMap<SimId, Vec<(Entity, Option<SpawnOrigin>)>>,
    ) -> Result<Self, BaselineCaptureError> {
        let mut entries = BTreeMap::new();
        for (sim_id, mut occupants) in found {
            if occupants.len() > 1 {
                occupants.sort_by_key(|(entity, _)| *entity);
                return Err(BaselineCaptureError::DuplicateIdentity {
                    sim_id,
                    entities: occupants.into_iter().map(|(entity, _)| entity).collect(),
                });
            }
            let (entity, origin) = occupants.remove(0);
            entries.insert(sim_id, BaselineEntry { entity, origin });
        }
        Ok(Self {
            entries,
            retired: BTreeSet::new(),
            reconstructed: BTreeSet::new(),
            superseded: BTreeSet::new(),
        })
    }

    /// Declare that this transaction intends to remove these identities without
    /// replacing them.
    pub fn retiring(mut self, ids: impl IntoIterator<Item = SimId>) -> Self {
        self.retired.extend(ids);
        self
    }

    /// Declare that this transaction intends to despawn these identities' bodies
    /// and build new ones for the same identities.
    pub fn reconstructing(mut self, ids: impl IntoIterator<Item = SimId>) -> Self {
        self.reconstructed.extend(ids);
        self
    }

    /// Declare that this transaction is building a HIDDEN CANDIDATE for an
    /// identity whose live body is meant to REMAIN STANDING until publication.
    ///
    /// ⛔⛤ **THIS IS THE A10 OPERATION, AND IT IS NOT [`Self::reconstructing`]
    /// WITH A LATER DEADLINE.** `reconstructing` states *"the old body should
    /// already be gone"*, so a live predecessor is
    /// [`RosterViolation::ReconstructedOldSurvived`] BY DEFINITION — that is the
    /// declaration working, and it is the wrong declaration for a candidate
    /// world. MEASURED 2026-09-13: it is exactly why flipping
    /// `ROOM_CANDIDATE_BRACKET` refused `death_restores_the_checkpoint`, with
    /// `Duplicated { placement:ground_gun_sword, count: 2 }` beside it.
    ///
    /// ⇒ Under this declaration the committed-roster check stops asking the
    /// coexistence to justify itself and asks the two things that are actually
    /// owed: **the live body is still here** (a candidate that destroyed its own
    /// predecessor broke the invariant it exists to keep) and **the candidate
    /// was built**. Whether publishing would leave ONE occupant is a different
    /// question asked of a different world — see
    /// [`verify_projected_roster`], which is the only place that can ask it.
    pub fn superseding(mut self, ids: impl IntoIterator<Item = SimId>) -> Self {
        self.superseded.extend(ids);
        self
    }

    pub fn entries(&self) -> &BTreeMap<SimId, BaselineEntry> {
        &self.entries
    }

    pub fn contains(&self, sim_id: &SimId) -> bool {
        self.entries.contains_key(sim_id)
    }

    pub fn is_retired(&self, sim_id: &SimId) -> bool {
        self.retired.contains(sim_id)
    }

    pub fn is_reconstructed(&self, sim_id: &SimId) -> bool {
        self.reconstructed.contains(sim_id)
    }

    pub fn is_superseded(&self, sim_id: &SimId) -> bool {
        self.superseded.contains(sim_id)
    }

    /// The entity a superseded identity is still standing on, so a verifier can
    /// tell the live predecessor apart from the candidate that will replace it.
    pub fn superseded_body(&self, sim_id: &SimId) -> Option<Entity> {
        if !self.is_superseded(sim_id) {
            return None;
        }
        self.entries.get(sim_id).map(|entry| entry.entity)
    }
}

/// How one identity-bearing entity relates to the transaction being verified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeClassification {
    /// Stamped with this transaction's ownership by the executor.
    TransactionAuthoritative,
    /// Owned by a different construction transaction, and therefore none of
    /// this one's business. Another room's persistent contents live here.
    ForeignScope(TransactionId),
    /// Explicitly declared non-authoritative by [`PresentationOnly`].
    PresentationOnly,
    /// Carries an identity, no ownership stamp, and no explicit classification
    /// at all. Every production construction family is planned, so this is a
    /// construction violation rather than migration residue.
    Unowned,
}

/// Whether an entity is in the PUBLISHED world or is a hidden candidate.
///
/// ⛔⛤ **A SCOPE GATHER SEES BOTH AND DID NOT SAY WHICH — `A10`, 2026-09-14.**
/// [`AuthoritativeScope::gather`] uses `Allow<InactiveCandidate>` precisely so a
/// verifier can see a candidate it is validating. That is right, and it left the
/// two populations indistinguishable in the result: *"one occupant per
/// identity"* read over the union answers DUPLICATED for the legitimate A10 state
/// — live A and hidden B on one identity, which
/// `a_hidden_candidate_may_share_the_live_worlds_identity_and_a_published_one_may_not`
/// measured as ALLOWED.
///
/// ⇒ The projection needs the distinction as DATA, not as a second query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeVisibility {
    /// Ordinary queries can see it: it is part of the authoritative world now.
    Published,
    /// Carries [`InactiveCandidate`], so it is invisible to ordinary queries and
    /// becomes authoritative only if this transaction publishes.
    HiddenCandidate,
}

/// One identity-bearing entity in the world, and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeMember {
    pub sim_id: SimId,
    pub entity: Entity,
    pub classification: ScopeClassification,
    /// ⛔ NOT DEFAULTED. A member whose visibility nobody stated would be counted
    /// as published, which is the direction that turns a legal candidate into a
    /// duplicate-identity violation.
    pub visibility: ScopeVisibility,
}

/// Every identity-bearing entity in the world, classified against one
/// transaction.
///
/// Gathered by querying, never curated. The whole point of reading the
/// world is that the roots most worth catching are the ones a caller would not
/// have thought to list — a recipe that invents an authoritative entity does
/// not also add itself to the caller's array.
#[derive(Clone, Debug)]
pub struct AuthoritativeScope {
    transaction: TransactionId,
    members: Vec<ScopeMember>,
}

/// Why an inactive commit was refused before it built anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InactiveCommitRefused {
    /// [`register_inactive_candidate_filter`] was never called in this world, so
    /// [`InactiveCandidate`] would not hide anything and every "candidate" root
    /// would be live. Refused rather than committed — see
    /// [`ConstructionPlan::commit_inactive`].
    FilterNotInstalled,
}

/// Admit a candidate into the running world: the ONE publication boundary.
///
/// ⭐ **PUBLICATION IS A COMPONENT REMOVAL, SO NOTHING IS COPIED, MOVED OR
/// RE-IDENTIFIED.** The candidate already holds its `SimId`, its provenance, its
/// [`TransactionId`] and every relation it was built with; what changes is
/// whether ordinary queries can see it.
///
/// ⛔⛤ **THIS PARAGRAPH USED TO END *"WHICH IS WHY IT CANNOT HALF-HAPPEN"*, AND
/// THIS REPOSITORY'S OWN EXPERIMENT DISPROVED IT (`Q123`, 2026-09-13).** An
/// `on_remove` hook installed on the marker and asked, during a three-root
/// publication, how many siblings were still hidden observed `[3, 2, 1]` — so the
/// second and third hook invocations ran while the transaction was PARTIALLY
/// PUBLISHED. Bevy runs hooks and lifecycle observers inside `remove`, and this
/// loop removes one entity at a time.
///
/// ⇒ **THE GUARANTEE IS NARROWED TO WHAT IS TRUE: publication is atomic to
/// SCHEDULED SYSTEMS.** Nothing is scheduled inside an exclusive-world call, so
/// no system can observe a half-published transaction. It is NOT atomic to hooks
/// or observers.
///
/// ⚠ **AND THE EXPOSURE IS CLOSED BY ENCAPSULATION RATHER THAN BY A RULE.**
/// `InactiveCandidate` is `pub(crate)`: no crate outside this one can name it,
/// so no crate outside this one can attach a hook or an observer to it. A census
/// finding zero hooks today is a POPULATION FACT that rots; a type nobody can
/// name is a boundary. ⇒ If A10's composability ever needs publication atomic in
/// the presence of arbitrary hooks, entity-by-entity marker removal cannot be the
/// authority switch and the candidate-world design must choose a different one.
///
/// Returns how many roots were admitted, so a caller can assert it published the
/// transaction it built rather than an empty set.
pub fn publish_candidate(world: &mut World, transaction: &TransactionId) -> usize {
    let roots = candidate_roots(world, transaction);
    for entity in &roots {
        world.entity_mut(*entity).remove::<InactiveCandidate>();
    }
    roots.len()
}

/// Retire the live bodies a publication DECLARED it was replacing — the second
/// half of the publication boundary, and it runs only after the first.
///
/// ⛔⛤ **THE ORDER IS THE INVARIANT, NOT AN OPTIMISATION.** Jon's sentence is
/// *"only a validated candidate may become authoritative, and N is retired only
/// after that successful publication"*. Retiring first is the shape A10 exists
/// to delete: a refusal after it leaves the session holding neither world.
/// Calling this before [`publish_candidate`] would restore exactly that hazard,
/// which is why the body it despawns is looked up in the BASELINE — the world as
/// it stood when the transaction opened — rather than by querying for the
/// identity now, when the identity has two holders and the query cannot say
/// which one is the predecessor.
///
/// ⚠ **A DECLARED SUPERSESSION WHOSE BODY IS ALREADY GONE IS SILENT HERE, AND
/// THAT IS NOT A WAIVER.** `verify_committed_roster` refuses that transaction
/// with [`RosterViolation::SupersededLiveLost`] before publication is reached;
/// this function runs only on an admitted one, so it is not the place to
/// re-litigate it.
///
/// ⛔⛤ **AND IT RETIRES ONLY WHAT THIS PUBLICATION HAS AUTHORITY OVER.** A
/// superseded body that is in ANOTHER ENTITY'S CUSTODY — an object in a hand, a
/// rider on a mount — is not the publishing transaction's to despawn.
/// [`InCustodyOf`] is the repository's existing statement of exactly that: a body
/// carrying it *"keeps `RoomScopedEntity`, so reset/scope queries still see it;
/// `RoomResident` excludes it only from room-transition sweeps"*. The room's
/// sweeps already skip it, and so does this.
///
/// ⭐⭐ **MEASURED 2026-09-14, AND IT IS NOT A STYLE PREFERENCE.** Despawning the
/// held body here made `death_restores_the_checkpoint` fail 1/11 and
/// `two_persistence_authorities_for_one_item` fail with `still_owned=1`: the
/// custodian's own authority — `restore_custody_to_checkpoint` — takes the object
/// out of the hand AND despawns it, as one paired operation keyed on the item
/// entity. Reaching in and despawning first does not do half of that job, it
/// destroys the key the other half is found by, and the player walks away still
/// holding a weapon that no longer exists. With the skip: 11/11 and 0.
///
/// ⚠ **THE DEFERRED HALF IS NOT UNCHECKED.** If a custodian never lets go, two
/// bodies keep one identity and the NEXT transaction's
/// [`TransactionBaseline::capture`] refuses with
/// [`BaselineCaptureError::DuplicateIdentity`] — a loud, existing check, one
/// transaction later rather than at this instant.
///
/// Returns how many live bodies were retired, and how many were left to their
/// custodian.
pub fn retire_superseded(
    world: &mut World,
    effects: &PublicationEffects,
    baseline: &TransactionBaseline,
) -> SupersessionRetirement {
    let mut outcome = SupersessionRetirement::default();
    let mut departing: BTreeSet<&SimId> = effects.supersessions().map(|s| &s.live).collect();
    departing.extend(effects.retirements());
    for sim_id in departing {
        let Some(entry) = baseline.entries().get(sim_id) else {
            continue;
        };
        let Ok(entity) = world.get_entity_mut(entry.entity) else {
            continue;
        };
        if entity.contains::<crate::lifecycle::InCustodyOf>() {
            outcome.left_to_custodian += 1;
            continue;
        }
        entity.despawn();
        outcome.retired += 1;
    }
    outcome
}

/// What [`retire_superseded`] did with the bodies a publication declared it was
/// replacing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SupersessionRetirement {
    /// Despawned by this publication.
    pub retired: usize,
    /// In another entity's custody, so its custodian retires it. See the
    /// function's own note for the measurement behind this split.
    pub left_to_custodian: usize,
}

/// Discard a candidate without disturbing the live world.
///
/// ⛔ THE REFUSAL PATH, AND IT IS WHY THE GUARANTEE IS "LAST-GOOD" RATHER THAN
/// "FAIL-CLOSED". Nothing was retired to make room for this candidate, so
/// dropping it needs no recovery and makes no claim about the running world —
/// which is the whole difference from destroying N and then discovering N+1 is
/// invalid.
pub fn retire_candidate(world: &mut World, transaction: &TransactionId) -> usize {
    let roots = candidate_roots(world, transaction);
    for entity in &roots {
        world.entity_mut(*entity).despawn();
    }
    roots.len()
}

/// Every still-inactive root belonging to `transaction`.
///
/// ⛔ **`With<InactiveCandidate>` IS WHAT LETS THIS SEE THEM AT ALL**, and it is
/// doing two jobs: selecting the candidates, and OPTING THIS QUERY OUT of the
/// default filter that hides them. `DefaultQueryFilters` excludes a disabling
/// component only from queries that do not MENTION it, and `With` mentions it.
///
/// ⚠ **I FIRST WROTE `Allow<InactiveCandidate>` BESIDE THE `With` AND CALLED IT
/// LOAD-BEARING. IT WAS REDUNDANT, AND THE POISON IS HOW I KNOW** — removing it
/// left all four guards green, which is a finding about the CLAIM rather than
/// about the code. `Allow` is for a query that wants entities with AND without
/// the component; this one wants only the candidates. ⇒ A reader changing
/// `With` to anything that does not name `InactiveCandidate` silently gets zero
/// roots, `publish_candidate` reports zero, and the candidate stays invisible
/// forever while every call looks like it succeeded — that hazard is real, and
/// it lives on the `With`, not on an extra filter beside it.
fn candidate_roots(world: &mut World, transaction: &TransactionId) -> Vec<Entity> {
    let mut query =
        world.query_filtered::<(Entity, &TransactionId), bevy::prelude::With<InactiveCandidate>>();
    query
        .iter(world)
        .filter(|(_, owner)| *owner == transaction)
        .map(|(entity, _)| entity)
        .collect()
}

impl AuthoritativeScope {
    /// Query the world for every entity carrying a [`SimId`] and classify each
    /// against `transaction`.
    ///
    /// Classification is by component, never by identity spelling: an entity is
    /// [`ScopeClassification::PresentationOnly`] because it says so, and
    /// authoritative because the executor stamped it, not because its `SimId`
    /// starts with one prefix or another.
    pub fn gather(world: &mut World, transaction: &TransactionId) -> Self {
        let mut members = Vec::new();
        // `Allow<InactiveCandidate>` means *"entities WITH and WITHOUT the
        // component"*, which is what a scope gather wants: the live world AND any
        // candidate being validated. Without it `DefaultQueryFilters` would hide
        // every candidate from the function whose job is to find violations in
        // one.
        //
        // ⛔⛤ **AND I CANNOT CALL IT LOAD-BEARING, BECAUSE THREE POISONS FAILED
        // TO MAKE IT BITE — the honest label is REASONED, not MEASURED.**
        // Removing it left every candidate arm green, and the reason is worth
        // more than the filter: **`verify_committed_roster` reads most of what it
        // checks DIRECTLY BY `Entity` from the receipt**, and bevy's own doc says
        // *"entities with disabling components are still present in the World and
        // can be accessed directly"* — direct access is not filtered at all. So
        // the receipt-driven checks (provenance, identity, liveness of a planned
        // root) see a candidate with or without this.
        //
        // ⇒ What it can only matter for is the half that is NOT receipt-driven:
        // strays, duplicates and unowned identities found by QUERYING the world
        // — *"the roots most worth catching are the ones nobody thought to
        // list"*. I could not build a candidate-internal case of that, because a
        // recipe-spawned stray is not stamped and so is visible anyway (see
        // `a_recipe_that_spawns_its_own_entity_escapes_the_candidate_isolation`).
        // ⚠ It stays because a scope that cannot see what it is scoping is wrong
        // on its face; it is documented as unproven rather than asserted.
        let mut query = world.query_filtered::<(
            Entity,
            &SimId,
            Option<&TransactionId>,
            Option<&PresentationOnly>,
            // ⛔ NAMED SO THE GATHER CAN REPORT IT. `Allow` opts this query out of
            // the default filter; reading the component is what lets the result
            // say WHICH population each member is in. See [`ScopeVisibility`].
            Option<&InactiveCandidate>,
        ), bevy::ecs::query::Allow<InactiveCandidate>>();
        for (entity, sim_id, owner, presentation, hidden) in query.iter(world) {
            let classification = if presentation.is_some() {
                ScopeClassification::PresentationOnly
            } else {
                match owner {
                    Some(owner) if owner == transaction => {
                        ScopeClassification::TransactionAuthoritative
                    }
                    Some(other) => ScopeClassification::ForeignScope(other.clone()),
                    None => ScopeClassification::Unowned,
                }
            };
            members.push(ScopeMember {
                sim_id: sim_id.clone(),
                entity,
                classification,
                visibility: if hidden.is_some() {
                    ScopeVisibility::HiddenCandidate
                } else {
                    ScopeVisibility::Published
                },
            });
        }
        // Query iteration order is not stable across runs; violations derived
        // from this must be, so sort by the pair that is.
        members.sort_by(|a, b| (&a.sim_id, a.entity).cmp(&(&b.sim_id, b.entity)));
        Self {
            transaction: transaction.clone(),
            members,
        }
    }

    /// Build a scope from explicit members, for fixtures.
    pub fn from_members(transaction: TransactionId, members: Vec<ScopeMember>) -> Self {
        let mut members = members;
        members.sort_by(|a, b| (&a.sim_id, a.entity).cmp(&(&b.sim_id, b.entity)));
        Self {
            transaction,
            members,
        }
    }

    pub fn transaction(&self) -> &TransactionId {
        &self.transaction
    }

    pub fn members(&self) -> &[ScopeMember] {
        &self.members
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// A10: THE PROJECTED POST-PUBLICATION ROSTER
// ═══════════════════════════════════════════════════════════════════════════
//
// ⛔⛤ **THE VERIFIER A10 NEEDS DOES NOT ASK WHETHER THE LIVE WORLD IS ALREADY
// RIGHT.** `verify_committed_roster` above judges the world AS IT IS, at the
// close of a transaction that has already mutated it — and under that question a
// still-live predecessor is `ReconstructedOldSurvived`, a violation. A10's whole
// invariant is that the predecessor IS still live while its replacement is being
// judged, so the question has to change rather than the answer:
//
// > **What would the authoritative roster be if this candidate published?**
// > Validate THAT.
//
// ⚠ `TransactionBaseline::reconstructing` KEEPS ITS MEANING — *the old body
// should already be gone, the new one should exist*. This is a different
// operation and gets different vocabulary rather than an exception branch inside
// that one.

/// One staged replacement: `candidate` takes `live`'s place when this publishes.
///
/// ⚠ **`live` AND `candidate` MAY BE THE SAME `SimId`, AND THAT IS THE CENTRAL
/// CASE.** A candidate session root carries the live root's identity by design —
/// measured in
/// `a_hidden_candidate_may_share_the_live_worlds_identity_and_a_published_one_may_not`
/// — so a supersession is not *"a rename"*; it is *"this entity replaces that
/// one"*, and the identities are free to agree.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Supersession {
    /// The published identity that goes away when this publishes.
    pub live: SimId,
    /// The hidden candidate identity that takes its place.
    pub candidate: SimId,
}

/// What a candidate transaction DECLARES it will do to the live world if it is
/// admitted.
///
/// ⭐ **FOUR DECLARATIONS, NOT FIVE SPECIAL CASES.** Additions come from the plan;
/// retirements and supersessions are stated here; everything else in the live
/// world is RETAINED by omission. Checkpoint custody restoration, candidate room
/// actors, removals, reauthored occurrences and changed relations are all the
/// same four statements rather than a branch each.
///
/// ⛔ **OMISSION MEANS RETAINED, DELIBERATELY.** The alternative — requiring every
/// surviving identity to be listed — makes the declaration a second copy of the
/// world that has to be kept in step with it, which is the synchronisation
/// architecture this design exists to avoid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PublicationEffects {
    supersedes: BTreeSet<Supersession>,
    retires: BTreeSet<SimId>,
    owners: BTreeSet<TransactionId>,
}

impl PublicationEffects {
    pub fn new() -> Self {
        Self::default()
    }

    /// Name a transaction whose hidden candidates this publication is entitled
    /// to make authoritative.
    ///
    /// ⛔⛤ **A PUBLICATION IS NOT ALWAYS ONE TRANSACTION, AND ASSUMING IT WAS
    /// WOULD HAVE REPORTED EVERY OTHER LANE'S CANDIDATE AS STOLEN.** A room
    /// commits through `construction_transactions` — the actor lane plus one per
    /// capability lane — under a single verdict. The owning set is therefore
    /// DECLARED with the rest of the effects rather than read off whichever
    /// single transaction happened to be used to gather the scope.
    pub fn owned_by(mut self, transaction: TransactionId) -> Self {
        self.owners.insert(transaction);
        self
    }

    /// The transactions whose candidates this publication may admit. Empty means
    /// it may admit NONE — an unstated owner is not a wildcard.
    pub fn owners(&self) -> &BTreeSet<TransactionId> {
        &self.owners
    }

    /// `candidate` replaces `live` at publication; `live` is legal until then.
    pub fn superseding(mut self, live: SimId, candidate: SimId) -> Self {
        self.supersedes.insert(Supersession { live, candidate });
        self
    }

    /// `live` goes at publication and nothing replaces it.
    pub fn retiring(mut self, live: SimId) -> Self {
        self.retires.insert(live);
        self
    }

    pub fn supersessions(&self) -> impl Iterator<Item = &Supersession> {
        self.supersedes.iter()
    }

    pub fn retirements(&self) -> impl Iterator<Item = &SimId> {
        self.retires.iter()
    }

    /// Every published identity this declares will be gone afterwards.
    fn departing(&self) -> BTreeSet<&SimId> {
        self.retires
            .iter()
            .chain(self.supersedes.iter().map(|s| &s.live))
            .collect()
    }
}

/// Where a projected occupant comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectedSource {
    /// Live now, not declared departing, so it is still there afterwards.
    RetainedLive,
    /// A hidden candidate that publication makes authoritative.
    Candidate,
}

/// One entity the authoritative world WOULD hold after this candidate published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedOccupant {
    pub entity: Entity,
    pub source: ProjectedSource,
}

/// The authoritative roster this candidate's declared effects would produce.
///
/// ⛔ **A PROJECTION, NEVER A MUTATION.** Nothing here touches the world; the
/// live world stays exactly as it is while its successor is judged, which is the
/// A10 guarantee restated as a data structure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectedRoster {
    occupants: BTreeMap<SimId, Vec<ProjectedOccupant>>,
}

impl ProjectedRoster {
    pub fn occupants_of(&self, sim_id: &SimId) -> &[ProjectedOccupant] {
        self.occupants.get(sim_id).map_or(&[][..], Vec::as_slice)
    }

    pub fn identities(&self) -> impl Iterator<Item = &SimId> {
        self.occupants.keys()
    }

    pub fn len(&self) -> usize {
        self.occupants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.occupants.is_empty()
    }
}

/// Build the roster publication WOULD produce, without producing it.
///
/// ```text
/// ProjectedRoster = published members
///                 - declared retirements
///                 - superseded published members
///                 + hidden candidate members
/// ```
///
/// ⚠ **PRESENTATION-ONLY MEMBERS ARE EXCLUDED**, by classification rather than by
/// spelling, for the same reason `verify_committed_roster` excludes them: they
/// carry an identity and no authority.
pub fn project_post_publication_roster(
    scope: &AuthoritativeScope,
    effects: &PublicationEffects,
) -> ProjectedRoster {
    let departing = effects.departing();
    let mut occupants: BTreeMap<SimId, Vec<ProjectedOccupant>> = BTreeMap::new();
    for member in scope.members() {
        if member.classification == ScopeClassification::PresentationOnly {
            continue;
        }
        let source = match member.visibility {
            ScopeVisibility::Published => {
                if departing.contains(&member.sim_id) {
                    continue;
                }
                ProjectedSource::RetainedLive
            }
            ScopeVisibility::HiddenCandidate => ProjectedSource::Candidate,
        };
        occupants
            .entry(member.sim_id.clone())
            .or_default()
            .push(ProjectedOccupant {
                entity: member.entity,
                source,
            });
    }
    for entries in occupants.values_mut() {
        entries.sort_by_key(|occupant| occupant.entity);
    }
    ProjectedRoster { occupants }
}

/// Why a candidate may not publish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionViolation {
    /// Two entities would hold one identity in the published world. The A10
    /// failure this verifier exists for: a candidate that supersedes nothing,
    /// or supersedes the wrong predecessor, lands beside it.
    Duplicated {
        sim_id: SimId,
        count: usize,
    },
    /// A declared supersession names a live identity that is not live.
    SupersededNotLive {
        sim_id: SimId,
    },
    /// A declared supersession names a candidate identity this transaction did
    /// not build.
    SupersedingCandidateMissing {
        sim_id: SimId,
    },
    /// A declared retirement names a live identity that is not live.
    RetiredNotLive {
        sim_id: SimId,
    },
    /// A hidden candidate carries somebody else's transaction stamp, or none.
    CandidateNotOwned {
        sim_id: SimId,
        /// Every transaction this publication declared it owns — see
        /// [`PublicationEffects::owned_by`] for why it is a set.
        expected: BTreeSet<TransactionId>,
        found: Option<TransactionId>,
    },
    /// The projected world would lose an identity nothing declared departing.
    ///
    /// ⛔ It cannot arise from the projection's arithmetic — omission means
    /// retained — so it reports a candidate that REPLACED a live entity during
    /// construction, which is the destructive mutation A10 forbids before
    /// publication.
    LiveLostWithoutDeclaration {
        sim_id: SimId,
    },
}

impl std::fmt::Display for ProjectionViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duplicated { sim_id, count } => write!(
                f,
                "publishing would leave {count} entities on `{sim_id}`; a candidate \
                 must supersede the live holder of an identity it takes"
            ),
            Self::SupersededNotLive { sim_id } => write!(
                f,
                "the candidate declares it supersedes `{sim_id}`, which is not live"
            ),
            Self::SupersedingCandidateMissing { sim_id } => write!(
                f,
                "the candidate declares `{sim_id}` as a replacement and did not build it"
            ),
            Self::RetiredNotLive { sim_id } => write!(
                f,
                "the candidate declares it retires `{sim_id}`, which is not live"
            ),
            Self::CandidateNotOwned {
                sim_id,
                expected,
                found,
            } => write!(
                f,
                "candidate `{sim_id}` is stamped {found:?}, which is none of this \
                 publication's transactions {expected:?}"
            ),
            Self::LiveLostWithoutDeclaration { sim_id } => write!(
                f,
                "`{sim_id}` was live when the candidate opened and would be gone after \
                 publication without being declared retired or superseded — construction \
                 destroyed part of the live world"
            ),
        }
    }
}

impl std::error::Error for ProjectionViolation {}

/// Would the authoritative world be valid if this candidate published?
///
/// ⛔⛤ **THIS IS THE QUESTION, AND IT IS NOT *"IS THE LIVE WORLD ALREADY
/// RIGHT?"***. The live world is deliberately still the OLD one here. A refusal
/// costs nothing to recover from because nothing was retired to make room.
///
/// ⚠ **THE BASELINE IS WHAT MAKES `LiveLostWithoutDeclaration` POSSIBLE.** The
/// projection alone cannot see a live entity that construction destroyed —
/// omission means retained, so a destroyed one is simply absent from both sides.
/// Comparing against what was live when the transaction OPENED is what turns
/// that silence into a violation.
pub fn verify_projected_roster(
    projection: &ProjectedRoster,
    effects: &PublicationEffects,
    baseline: &TransactionBaseline,
    scope: &AuthoritativeScope,
    world: &World,
) -> Result<(), Vec<ProjectionViolation>> {
    let mut violations = Vec::new();

    let published: BTreeSet<&SimId> = scope
        .members()
        .iter()
        .filter(|member| {
            member.visibility == ScopeVisibility::Published
                && member.classification != ScopeClassification::PresentationOnly
        })
        .map(|member| &member.sim_id)
        .collect();
    let candidates: BTreeSet<&SimId> = scope
        .members()
        .iter()
        .filter(|member| member.visibility == ScopeVisibility::HiddenCandidate)
        .map(|member| &member.sim_id)
        .collect();

    for supersession in effects.supersessions() {
        if !published.contains(&supersession.live) {
            violations.push(ProjectionViolation::SupersededNotLive {
                sim_id: supersession.live.clone(),
            });
        }
        if !candidates.contains(&supersession.candidate) {
            violations.push(ProjectionViolation::SupersedingCandidateMissing {
                sim_id: supersession.candidate.clone(),
            });
        }
    }
    for retired in effects.retirements() {
        if !published.contains(retired) {
            violations.push(ProjectionViolation::RetiredNotLive {
                sim_id: retired.clone(),
            });
        }
    }

    // ⛔ THE CORE INVARIANT: one authoritative holder per identity, asked of the
    // world publication WOULD produce rather than of the one that exists.
    for (sim_id, entries) in &projection.occupants {
        if entries.len() > 1 {
            violations.push(ProjectionViolation::Duplicated {
                sim_id: sim_id.clone(),
                count: entries.len(),
            });
        }
    }

    // Every candidate this transaction would publish must be its own.
    for member in scope.members() {
        if member.visibility != ScopeVisibility::HiddenCandidate {
            continue;
        }
        let owner = world.get::<TransactionId>(member.entity);
        if !owner.is_some_and(|owner| effects.owners().contains(owner)) {
            violations.push(ProjectionViolation::CandidateNotOwned {
                sim_id: member.sim_id.clone(),
                expected: effects.owners().clone(),
                found: owner.cloned(),
            });
        }
    }

    // ⛔ AND NOTHING THE LIVE WORLD HELD MAY HAVE VANISHED UNDECLARED.
    let departing = effects.departing();
    for sim_id in baseline.entries().keys() {
        if departing.contains(sim_id) {
            continue;
        }
        if projection.occupants_of(sim_id).is_empty() {
            violations.push(ProjectionViolation::LiveLostWithoutDeclaration {
                sim_id: sim_id.clone(),
            });
        }
    }

    violations.sort_by_key(|violation| format!("{violation:?}"));
    violations.dedup();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Bumped when the plan dump's shape changes. The dump is an inspection and
/// comparison surface, so its shape is a compatibility contract.
pub const CONSTRUCTION_PLAN_SCHEMA_VERSION: u32 = 5;
