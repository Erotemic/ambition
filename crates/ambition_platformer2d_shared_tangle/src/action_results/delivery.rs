//! Bounded delivery progress, separate from domain application and completion.

use ambition_extension_sdk::action::ActionDelivery;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryProgress {
    pub status: ActionDelivery,
    pub attempts: u32,
    pub last_attempt: Option<u64>,
    pub last_error: Option<String>,
}
impl Default for DeliveryProgress {
    fn default() -> Self {
        Self {status: ActionDelivery::Waiting, attempts: 0, last_attempt: None, last_error: None}
    }
}
impl DeliveryProgress {
    pub fn queue(&mut self, now: u64, written: u64, bound: bool, unbound: &str) -> bool {
        if matches!(self.status, ActionDelivery::Queued | ActionDelivery::Invoked |
            ActionDelivery::Acknowledged | ActionDelivery::Quarantined | ActionDelivery::Retired) { return false; }
        if !bound {
            self.status = ActionDelivery::Unbound;
            self.last_error = Some(unbound.into());
            return false;
        }
        if written >= now || self.last_attempt.is_some_and(|tick| tick >= now) { return false; }
        self.status = ActionDelivery::Queued;
        true
    }
    pub fn started(&mut self, now: u64) {
        if self.status != ActionDelivery::Queued { return; }
        self.status = ActionDelivery::Invoked;
        self.attempts = self.attempts.saturating_add(1);
        self.last_attempt = Some(now);
    }
    pub fn committed(&mut self) {
        if self.status != ActionDelivery::Invoked { return; }
        self.status = ActionDelivery::Acknowledged;
        self.last_error = None;
    }
    pub fn faulted(&mut self, max_attempts: u32) {
        if self.status != ActionDelivery::Invoked { return; }
        self.status = if self.attempts >= max_attempts { ActionDelivery::Quarantined }
            else { ActionDelivery::Faulted };
        self.last_error = Some("handler_fault".into());
    }
    pub fn unavailable(&mut self) {
        if self.status != ActionDelivery::Queued { return; }
        self.status = ActionDelivery::Unbound;
        self.last_error = Some("handler_not_invoked".into());
    }
}
ambition_extension_sdk::port_record! {
    DeliveryProgress { status: ActionDelivery, attempts: u32, last_attempt: Option<u64>, last_error: Option<String> }
}
