//! Narrow neutral shell action adapter shared by startup, launcher, loading,
//! and gameplay-to-home presentation.
//!
//! The shell reads NO raw devices. Every device — keyboard, gamepad, touch
//! stick and buttons, mouse wheel — reaches it through [`MenuControlFrame`],
//! the semantic menu intent populated from the persistent input participant's
//! `ActionState` (see `populate_menu_control_frame_from_actions`) and the
//! virtual-device folds. The participant exists from boot, so the frame is
//! live at the startup cards and the launcher with no gameplay actor and no
//! session; keyboard Enter, gamepad South, a virtual touch confirm, and a
//! touch stick flick all arrive as the same one-frame edges.
//!
//! The frame resource is OPTIONAL: an app composing `MinimalShellPlugins`
//! without a host input stack has no participant and no frame, and its shell
//! surfaces are inert to devices (pointer/touch row activation still works
//! through the `MenuActionActivated` bridge). Shell consumers run in
//! `InputSet::Consume`, after every producer — an edge produced this frame
//! is consumed this frame.

use ambition_input::MenuControlFrame;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShellActionEdges {
    pub previous: bool,
    pub next: bool,
    pub confirm: bool,
    pub back: bool,
    /// Open / toggle the in-session pause menu: the semantic Start intent
    /// (keyboard Escape, controller Start, the touch HUD's "Menu" button).
    /// The pause menu it opens carries "Quit to Title" and "Quit to Desktop"
    /// entries, so Start no longer retires; quitting to home is a separate
    /// semantic developer action.
    pub pause: bool,
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
        startup_acknowledge: menu.select,
        loading_continue: menu.select,
    }
}

#[cfg(test)]
mod tests {
    use super::shell_action_edges;
    use ambition_input::MenuControlFrame;

    /// Every shell surface is reachable through the ONE semantic frame — the
    /// state a phone, a keyboard, and a controller all reduce to. Each
    /// assertion names the device-neutral intent that carries it.
    #[test]
    fn the_menu_frame_alone_drives_every_shell_action() {
        // Pre-poison: with no frame, nothing may fire. A permissive adapter
        // would make every assertion below vacuous.
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
