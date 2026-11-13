//! Explicit Effect Typing for Keller-VIR functions.
//!
//! Section 1.1:
//! `EffectSet ::= '[' [ 'alloc' ] [ 'io' ] [ 'panic' ] [ 'thread' ] [ 'pure' ] ']'`

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
    Alloc,
    Io,
    Panic,
    Thread,
    Pure,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectSet {
    pub effects: Vec<Effect>,
}

impl EffectSet {
    pub fn pure() -> Self {
        Self {
            effects: vec![Effect::Pure],
        }
    }

    pub fn is_pure(&self) -> bool {
        self.effects.contains(&Effect::Pure)
            || (self.effects.is_empty()
                || (!self.has_io() && !self.has_thread() && !self.has_alloc() && !self.can_panic()))
    }

    pub fn has_io(&self) -> bool {
        self.effects.contains(&Effect::Io)
    }

    pub fn has_alloc(&self) -> bool {
        self.effects.contains(&Effect::Alloc)
    }

    pub fn can_panic(&self) -> bool {
        self.effects.contains(&Effect::Panic)
    }

    pub fn has_thread(&self) -> bool {
        self.effects.contains(&Effect::Thread)
    }

    pub fn add(&mut self, effect: Effect) {
        if !self.effects.contains(&effect) {
            self.effects.push(effect);
        }
    }
}
