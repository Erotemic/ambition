//! Anti-clump attack-slot arbitration.
//!
//! Default sandbox enemies all resolve "Chase" to "walk toward the
//! player's body". The result is a clump of overlapping enemies, no
//! visible attack pattern, and an unreadable combat picture. The
//! solution is the same one used in arcade brawlers and 3D
//! action-adventure games: a small number of *attack slots* around
//! the target, allocated cooperatively. Enemies that hold a slot can
//! commit to their attack; the rest hold at the outer ring and wait
//! their turn.
//!
//! Concretely this module owns:
//!
//! - [`CombatSlot`] — a typed offset around the target.
//! - [`CombatSlotBoard`] — the per-target board owned by the sandbox
//!   encounter / room. Holds the slot layout + the current assignment
//!   (`Option<actor_id>` per slot).
//! - [`SlotKind`] — `Melee` (ring at melee radius) vs `Aerial` (arc
//!   above the target).
//! - [`assign_slots`] — pure greedy allocator: nearest qualifying
//!   actor wins each slot. Stable across frames (an actor that already
//!   owns a slot keeps it as long as it's still on the field).
//!
//! The slot board never blocks an enemy from existing. It only
//! decides *which* enemies are allowed to commit to an attack and
//! where they should stand. Enemies without a slot fall back to a
//! holding ring offset further out.

use crate::Vec2;

/// What family of slot this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// On-the-ground attack slot — melee ring around the target.
    Melee,
    /// Aerial attack slot — arc above the target for flyers.
    Aerial,
}

/// One slot definition + its current assignment.
#[derive(Clone, Debug)]
pub struct CombatSlot {
    pub kind: SlotKind,
    /// Position relative to the target (added to the live target
    /// position on read).
    pub offset: Vec2,
    /// Holding-ring fallback for actors who request this slot kind
    /// but don't win an allocation. Larger radius / further away so
    /// they're visible but not in the fight.
    pub holding_offset: Vec2,
    /// Currently assigned actor id, if any.
    pub assigned_to: Option<String>,
}

impl CombatSlot {
    pub fn world_pos(&self, target: Vec2) -> Vec2 {
        target + self.offset
    }
    pub fn holding_pos(&self, target: Vec2) -> Vec2 {
        target + self.holding_offset
    }
}

/// Per-target board of attack slots.
#[derive(Clone, Debug, Default)]
pub struct CombatSlotBoard {
    pub slots: Vec<CombatSlot>,
}

impl CombatSlotBoard {
    /// Build a default board with N melee slots around the target
    /// and M aerial slots arcing above it. Slots are evenly spaced
    /// around / across their respective radii.
    pub fn new(
        melee_slots: usize,
        melee_radius: f32,
        aerial_slots: usize,
        aerial_radius: f32,
        aerial_altitude: f32,
    ) -> Self {
        let mut slots = Vec::with_capacity(melee_slots + aerial_slots);
        for i in 0..melee_slots {
            let theta = if melee_slots == 0 {
                0.0
            } else {
                (i as f32 / melee_slots as f32) * std::f32::consts::TAU
            };
            slots.push(CombatSlot {
                kind: SlotKind::Melee,
                offset: Vec2::new(
                    theta.cos() * melee_radius,
                    -16.0 + theta.sin() * (melee_radius * 0.15),
                ),
                holding_offset: Vec2::new(
                    theta.cos() * (melee_radius + 90.0),
                    -16.0 + theta.sin() * (melee_radius * 0.15),
                ),
                assigned_to: None,
            });
        }
        // Aerial slots: arc above the target, spread across roughly
        // ±60° from straight-up.
        for i in 0..aerial_slots {
            let t = if aerial_slots == 1 {
                0.5
            } else {
                i as f32 / (aerial_slots - 1) as f32
            };
            let theta = -std::f32::consts::FRAC_PI_3 + t * (std::f32::consts::FRAC_PI_3 * 2.0);
            let off_x = theta.sin() * aerial_radius;
            slots.push(CombatSlot {
                kind: SlotKind::Aerial,
                offset: Vec2::new(off_x, -aerial_altitude),
                holding_offset: Vec2::new(off_x * 1.6, -(aerial_altitude + 80.0)),
                assigned_to: None,
            });
        }
        Self { slots }
    }

    /// Clear every assignment. Used on room reload.
    pub fn clear_assignments(&mut self) {
        for slot in &mut self.slots {
            slot.assigned_to = None;
        }
    }

    /// Drop assignments for actors that no longer appear in
    /// `live_ids`. Called once per tick before assignment so dead /
    /// dismounted actors free their slots.
    pub fn release_missing(&mut self, live_ids: &[&str]) {
        for slot in &mut self.slots {
            if let Some(id) = &slot.assigned_to {
                if !live_ids.contains(&id.as_str()) {
                    slot.assigned_to = None;
                }
            }
        }
    }

    /// Look up the slot currently assigned to `actor_id`, if any.
    pub fn slot_for(&self, actor_id: &str) -> Option<&CombatSlot> {
        self.slots
            .iter()
            .find(|s| s.assigned_to.as_deref() == Some(actor_id))
    }
}

/// One actor's request: who they are, where they are, and which slot
/// kind they want.
#[derive(Clone, Debug)]
pub struct SlotRequest<'a> {
    pub actor_id: &'a str,
    pub actor_pos: Vec2,
    pub kind: SlotKind,
}

/// Pure greedy allocator: assign each requested actor to the nearest
/// available slot of its requested kind. Actors that already hold a
/// matching slot keep it (stability). Returns no value — the board's
/// `assigned_to` fields are the result.
pub fn assign_slots(board: &mut CombatSlotBoard, target_pos: Vec2, requests: &[SlotRequest]) {
    let live_ids: Vec<&str> = requests.iter().map(|r| r.actor_id).collect();
    board.release_missing(&live_ids);

    // Build a (distance, request_idx, slot_idx) triple list so we can
    // greedily pick the closest pairing first. This isn't optimal
    // (the Hungarian algorithm would be), but for small slot counts
    // (≤8) and small enemy counts (≤8) it's plenty good and
    // perfectly deterministic.
    let mut candidates: Vec<(f32, usize, usize)> = Vec::new();
    for (ri, req) in requests.iter().enumerate() {
        for (si, slot) in board.slots.iter().enumerate() {
            if slot.kind != req.kind {
                continue;
            }
            let dist = (slot.world_pos(target_pos) - req.actor_pos).length_squared();
            candidates.push((dist, ri, si));
        }
    }
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut assigned_actor = vec![false; requests.len()];
    // Preserve stable assignment: actors who already own a slot of
    // the right kind keep it without going through the candidate sort.
    for (ri, req) in requests.iter().enumerate() {
        for slot in &board.slots {
            if slot.assigned_to.as_deref() == Some(req.actor_id) && slot.kind == req.kind {
                assigned_actor[ri] = true;
                break;
            }
        }
    }

    let mut assigned_slot = vec![false; board.slots.len()];
    for (si, slot) in board.slots.iter().enumerate() {
        if slot.assigned_to.is_some() {
            assigned_slot[si] = true;
        }
    }

    for (_dist, ri, si) in candidates {
        if assigned_actor[ri] || assigned_slot[si] {
            continue;
        }
        board.slots[si].assigned_to = Some(requests[ri].actor_id.to_string());
        assigned_actor[ri] = true;
        assigned_slot[si] = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board() -> CombatSlotBoard {
        CombatSlotBoard::new(3, 80.0, 2, 200.0, 160.0)
    }

    #[test]
    fn board_has_expected_slot_counts() {
        let b = board();
        assert_eq!(
            b.slots.iter().filter(|s| s.kind == SlotKind::Melee).count(),
            3
        );
        assert_eq!(
            b.slots
                .iter()
                .filter(|s| s.kind == SlotKind::Aerial)
                .count(),
            2
        );
    }

    #[test]
    fn assigns_three_melee_actors_to_three_melee_slots() {
        let mut b = board();
        let target = Vec2::ZERO;
        let actors = ["a", "b", "c"];
        let reqs: Vec<SlotRequest> = actors
            .iter()
            .enumerate()
            .map(|(i, id)| SlotRequest {
                actor_id: id,
                actor_pos: Vec2::new(i as f32 * 40.0 + 60.0, 0.0),
                kind: SlotKind::Melee,
            })
            .collect();
        assign_slots(&mut b, target, &reqs);
        for id in actors {
            assert!(b.slot_for(id).is_some(), "{id} did not get a slot");
        }
    }

    #[test]
    fn extra_actors_do_not_get_slots() {
        let mut b = board(); // 3 melee slots
        let reqs: Vec<SlotRequest> = (0..5)
            .map(|i| SlotRequest {
                actor_id: ["a", "b", "c", "d", "e"][i],
                actor_pos: Vec2::new(i as f32 * 30.0, 0.0),
                kind: SlotKind::Melee,
            })
            .collect();
        assign_slots(&mut b, Vec2::ZERO, &reqs);
        let assigned: Vec<&str> = b
            .slots
            .iter()
            .filter_map(|s| s.assigned_to.as_deref())
            .collect();
        assert_eq!(assigned.len(), 3);
    }

    #[test]
    fn aerial_requests_get_aerial_slots_only() {
        let mut b = board();
        let reqs = [
            SlotRequest {
                actor_id: "flyer1",
                actor_pos: Vec2::new(0.0, -180.0),
                kind: SlotKind::Aerial,
            },
            SlotRequest {
                actor_id: "flyer2",
                actor_pos: Vec2::new(30.0, -180.0),
                kind: SlotKind::Aerial,
            },
        ];
        assign_slots(&mut b, Vec2::ZERO, &reqs);
        for r in &reqs {
            let slot = b.slot_for(r.actor_id).expect("expected aerial slot");
            assert_eq!(slot.kind, SlotKind::Aerial);
        }
    }

    #[test]
    fn dropped_actor_frees_slot_next_tick() {
        let mut b = board();
        let reqs1 = [SlotRequest {
            actor_id: "a",
            actor_pos: Vec2::new(80.0, 0.0),
            kind: SlotKind::Melee,
        }];
        assign_slots(&mut b, Vec2::ZERO, &reqs1);
        assert!(b.slot_for("a").is_some());

        // Next tick: actor "a" is gone, only "b" requests.
        let reqs2 = [SlotRequest {
            actor_id: "b",
            actor_pos: Vec2::new(80.0, 0.0),
            kind: SlotKind::Melee,
        }];
        assign_slots(&mut b, Vec2::ZERO, &reqs2);
        assert!(b.slot_for("a").is_none());
        assert!(b.slot_for("b").is_some());
    }

    #[test]
    fn stable_assignment_preserves_slot_for_existing_holder() {
        let mut b = board();
        let target = Vec2::ZERO;
        // Tick 1: "a" gets the closest slot.
        let reqs = [SlotRequest {
            actor_id: "a",
            actor_pos: Vec2::new(80.0, 0.0),
            kind: SlotKind::Melee,
        }];
        assign_slots(&mut b, target, &reqs);
        let slot1 = b.slot_for("a").unwrap().offset;
        // Tick 2: a moved + a new actor "b" appeared. "a" should keep their slot.
        let reqs = [
            SlotRequest {
                actor_id: "a",
                actor_pos: Vec2::new(0.0, 80.0),
                kind: SlotKind::Melee,
            },
            SlotRequest {
                actor_id: "b",
                actor_pos: Vec2::new(80.0, 0.0),
                kind: SlotKind::Melee,
            },
        ];
        assign_slots(&mut b, target, &reqs);
        let slot1_after = b.slot_for("a").unwrap().offset;
        assert_eq!(slot1, slot1_after, "actor a's slot moved between ticks");
    }
}
