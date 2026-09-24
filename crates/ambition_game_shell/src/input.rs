//! Narrow neutral shell action adapter shared by startup, launcher, loading,
//! and gameplay-to-home presentation.
//!
//! The shell reads no raw devices. Every device reaches it through
//! [`MenuControlFrame`], filled from the persistent input participant's
//! `ActionState` (see `populate_menu_control_frame_from_actions`) and the
//! virtual-device folds. The participant exists from boot, so the frame works
//! at the startup cards and the launcher with no session.
//!
//! The frame resource is optional. Without a host input stack, shell surfaces
//! ignore devices, but pointer and touch row activation still work through
//! `MenuActionActivated`. Shell consumers run in `InputSet::Consume`, after
//! every producer, so an edge is consumed in the frame it is produced.

use ambition_input::MenuControlFrame;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShellActionEdges {
    pub previous: bool,
    pub next: bool,
    pub confirm: bool,
    pub back: bool,
    /// Toggle the pause menu: the Start intent (Escape, controller Start, the
    /// touch "Menu" button). Start does not quit; the menu has quit rows.
    pub pause: bool,
    /// Decrease / increase the focused row's value (left/right), as opposed to
    /// moving between rows.
    pub decrease: bool,
    pub increase: bool,
    pub startup_acknowledge: bool,
    pub loading_continue: bool,
}

/// Fold the semantic menu frame into the shell's edge vocabulary. An absent
/// frame (no host input stack) is the neutral element.
pub fn shell_action_edges(menu: Option<&MenuControlFrame>) -> ShellActionEdges {
    let menu = menu.copied().unwrap_or_default();
    ShellActionEdges {
        previous: menu.up,
        next: menu.down,
        confirm: menu.select,
        back: menu.back,
        pause: menu.start,
        decrease: menu.left,
        increase: menu.right,
        startup_acknowledge: menu.select,
        loading_continue: menu.select,
    }
}

#[cfg(test)]
mod tests {
    use super::shell_action_edges;
    use ambition_input::MenuControlFrame;

    /// Every shell action is reachable through the one semantic frame.
    #[test]
    fn the_menu_frame_alone_drives_every_shell_action() {
        // With no frame, nothing fires.
        let idle = shell_action_edges(None);
        assert_eq!(idle, Default::default(), "no menu frame -> no edges");
        assert_eq!(
            shell_action_edges(Some(&MenuControlFrame::default())),
            Default::default(),
            "a neutral frame -> no edges"
        );

        let start = MenuControlFrame {
            start: true,
            ..Default::default()
        };
        assert!(
            shell_action_edges(Some(&start)).pause,
            "the Start intent (Escape / pad Start / touch Menu) opens the pause menu"
        );
        let back = MenuControlFrame {
            back: true,
            ..Default::default()
        };
        assert!(
            shell_action_edges(Some(&back)).back,
            "the Back intent closes an open menu"
        );
        let select = MenuControlFrame {
            select: true,
            ..Default::default()
        };
        let confirmed = shell_action_edges(Some(&select));
        assert!(
            confirmed.confirm && confirmed.startup_acknowledge && confirmed.loading_continue,
            "one Select intent must dismiss startup cards, pick launcher rows, \
             and release a loading ready-hold"
        );
        let down = MenuControlFrame {
            down: true,
            ..Default::default()
        };
        assert!(
            shell_action_edges(Some(&down)).next,
            "directional menu intent moves the cursor"
        );
        let up = MenuControlFrame {
            up: true,
            ..Default::default()
        };
        assert!(shell_action_edges(Some(&up)).previous);
    }
}
