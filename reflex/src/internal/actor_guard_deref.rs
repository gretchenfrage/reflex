
use crate::Actor;
use super::{ActorGuardShared, ActorGuardMut};

use std::ops::{Deref, DerefMut};

impl<Act: Actor> Deref for ActorGuardShared<Act> {
    type Target = Act;

    fn deref(&self) -> &Act {
        unsafe {
            &*self.ptr
        }
    }
}

impl<Act: Actor> Deref for ActorGuardMut<Act> {
    type Target = Act;

    fn deref(&self) -> &Act {
        unsafe {
            &*self.ptr
        }
    }
}

impl<Act: Actor> DerefMut for ActorGuardMut<Act> {
    fn deref_mut(&mut self) -> &mut Act {
        unsafe {
            &mut *self.ptr
        }
    }
}