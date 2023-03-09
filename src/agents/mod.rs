mod cf;
pub use cf::{ControlFlow, WaitHandle};

mod event;
pub use event::{Bundle, BundledApp, BundledTool, Event};

use crate::game::{Message, Replies};

/// An agent in the system, which can react to events.
///
/// Events are processed in 'rounds'. There's a list of 'current' events, which are fed into every actor at the same
/// time. Then all of the replies are collected, and those are the 'current' events for the next round.
///
/// As that implies, events are inherently ephemeral -- none persist more than one round.
pub trait Agent<M: Message>: Send + Sync {
    /// Called once on (re)start, to queue any starting events/ControlFlow as necessary. This will always be called
    /// before `react`.
    ///
    /// By default, does nothing and returns [`ControlFlow::Continue`] to allow [`Self::react`] to be called, under
    /// the assumption that your interesting code sits there.
    #[cfg_attr(coverage, no_coverage)]
    fn start(&mut self, _replies: &mut Replies<M>) -> ControlFlow {
        ControlFlow::Continue
    }

    /// React to the events of a round, indicating when the agent should be called next and optionally queueing some
    /// more events.
    ///
    /// Limitations on the [`Extend`] trait mean we just use the concrete type `Vec`. **Do not** do anything except
    /// pushing/extending/otherwise adding elements.
    ///
    /// By default, does nothing and returns [`ControlFlow::Kill`], under the assumption that you'd have implemented
    /// `react` if you wanted your agent to stay alive and do things.
    #[cfg_attr(coverage, no_coverage)]
    fn react(&mut self, _event: &M, _replies: &mut Replies<M>) -> ControlFlow {
        ControlFlow::Kill
    }
}

/// An agent which does nothing and immediately dies.
// Big mood, buddy.
pub struct NopAgent;
impl<M: Message> Agent<M> for NopAgent {
    // explicit impls to make sure that Nop keeps Nopping even if I decide to change the defaults
    fn start(&mut self, _replies: &mut Replies<M>) -> ControlFlow {
        ControlFlow::Kill
    }
    fn react(&mut self, _event: &M, _replies: &mut Replies<M>) -> ControlFlow {
        ControlFlow::Kill
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone, Default)]
    struct TestMessage;

    #[coverage_helper::test]
    fn nop_agent_dies_on_start() {
        let mut replies = Replies::<TestMessage>::default();
        assert_eq!(NopAgent.start(&mut replies), ControlFlow::Kill);
        assert_eq!(replies.spawn_len(), 0);
        assert_eq!(replies.queue_len(), 0);
    }

    #[coverage_helper::test]
    fn nop_agent_doesnt_react() {
        let mut replies = Replies::default();
        assert_eq!(
            NopAgent.react(&Event::CommandDone, &mut replies),
            ControlFlow::Kill
        );
        assert_eq!(replies.spawn_len(), 0);
        assert_eq!(replies.queue_len(), 0);
    }
}