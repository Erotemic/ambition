//! Rollback declaration owned by `ambition_conversation`.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    // The entity set localizes the two bodies through stable identities; `facts`
    // covers the non-entity semantics that can independently diverge.
    registrar.rollback_resource_clone_entity_set_probed::<crate::ActiveConversation>(
        OWNER,
        "resource.active_conversation",
        |conversation| conversation.referenced_entities(),
        |conversation| {
            use std::hash::{Hash, Hasher};
            let Some(live) = conversation.live() else {
                return 0;
            };
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            live.instance.hash(&mut hasher);
            match live.input_owner {
                crate::ConversationInputOwner::Participant(id) => (1u8, id.slot()).hash(&mut hasher),
                crate::ConversationInputOwner::Primary => (2u8, 0u8).hash(&mut hasher),
                crate::ConversationInputOwner::AllParticipants => (3u8, 0u8).hash(&mut hasher),
            }
            hasher.finish()
        },
    );
    registrar.rollback_resource_map_entities::<crate::ActiveConversation>(
        OWNER,
        "map.resource.active_conversation",
    );
    registrar.clear_message_on_rollback::<crate::ConversationEnded>(
        OWNER,
        "message.conversation_ended",
    );
    registrar.clear_message_on_rollback::<crate::ConversationCutBark>(
        OWNER,
        "message.conversation_cut_bark",
    );
}
