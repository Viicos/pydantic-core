use crate::{definitions::Definitions, recursion_guard::RecursionGuard};

use super::{CombinedValidator, Extra};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Exactness {
    Lax,
    Strict,
    Exact,
}

pub struct ValidationState<'a> {
    pub recursion_guard: &'a mut RecursionGuard,
    pub definitions: &'a Definitions<CombinedValidator>,
    pub exactness: Option<Exactness>,
    // deliberately make Extra readonly
    extra: Extra<'a>,
}

impl<'a> ValidationState<'a> {
    pub fn new(
        extra: Extra<'a>,
        definitions: &'a Definitions<CombinedValidator>,
        recursion_guard: &'a mut RecursionGuard,
    ) -> Self {
        Self {
            recursion_guard,
            definitions,
            // Don't care about exactness unless doing union validation
            exactness: None,
            extra,
        }
    }

    pub fn with_new_extra<'r, R: 'r>(
        &mut self,
        extra: Extra<'_>,
        f: impl for<'s> FnOnce(&'s mut ValidationState<'_>) -> R,
    ) -> R {
        // TODO: It would be nice to implement this function with a drop guard instead of a closure,
        // but lifetimes get in a tangle. Maybe someone brave wants to have a go at unpicking lifetimes.
        let mut new_state = ValidationState {
            recursion_guard: self.recursion_guard,
            definitions: self.definitions,
            exactness: self.exactness,
            extra,
        };
        let result = f(&mut new_state);
        match new_state.exactness {
            Some(exactness) => self.set_exactness_ceiling(exactness),
            None => self.exactness = None,
        }
        result
    }

    /// Temporarily rebinds the extra field by calling `f` to modify extra.
    ///
    /// When `ValidationStateWithReboundExtra` drops, the extra field is restored to its original value.
    pub fn rebind_extra<'state>(
        &'state mut self,
        f: impl FnOnce(&mut Extra<'a>),
    ) -> ValidationStateWithReboundExtra<'state, 'a> {
        let old_extra = Extra { ..self.extra };
        f(&mut self.extra);
        ValidationStateWithReboundExtra { state: self, old_extra }
    }

    pub fn extra(&self) -> &'_ Extra<'a> {
        &self.extra
    }

    pub fn strict_or(&self, default: bool) -> bool {
        self.extra.strict.unwrap_or(default)
    }

    /// Sets the exactness of this state to unknown.
    ///
    /// In general this de-optimizes union validation by forcing strict & lax validation passes,
    /// so it's better to determine exactness and call `set_exactness_ceiling` when possible.
    pub fn set_exactness_unknown(&mut self) {
        self.exactness = None;
    }

    /// Sets the exactness to the lower of the current exactness
    /// and the given exactness.
    ///
    /// This is designed to be used in union validation, where the
    /// idea is that the "most exact" validation wins.
    pub fn set_exactness_ceiling(&mut self, exactness: Exactness) {
        match self.exactness {
            None | Some(Exactness::Lax) => {}
            Some(Exactness::Strict) => {
                if exactness == Exactness::Lax {
                    self.exactness = Some(Exactness::Lax);
                }
            }
            Some(Exactness::Exact) => self.exactness = Some(exactness),
        }
    }
}

pub struct ValidationStateWithReboundExtra<'state, 'a> {
    state: &'state mut ValidationState<'a>,
    old_extra: Extra<'a>,
}

impl<'a> std::ops::Deref for ValidationStateWithReboundExtra<'_, 'a> {
    type Target = ValidationState<'a>;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<'a> std::ops::DerefMut for ValidationStateWithReboundExtra<'_, 'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl Drop for ValidationStateWithReboundExtra<'_, '_> {
    fn drop(&mut self) {
        std::mem::swap(&mut self.state.extra, &mut self.old_extra);
    }
}
