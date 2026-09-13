use leptos::*;
/// Shared state manager for two-step deletion interactions (arm -> confirm -> reset).
#[derive(Clone)]
pub struct TwoStepDeleteController<T>
where
    T: PartialEq + Clone + 'static,
{
    armed: RwSignal<Option<T>>,
}

impl<T> TwoStepDeleteController<T>
where
    T: PartialEq + Clone + 'static,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            armed: create_rw_signal(None),
        }
    }

    #[allow(dead_code)]
    pub fn arm(&self, target: T) {
        if self.armed.get() == Some(target.clone()) {
            return;
        }
        self.armed.set(Some(target));
    }

    #[allow(dead_code)]
    pub fn is_armed(&self, target: &T) -> bool {
        self.armed.get().as_ref() == Some(target)
    }

    #[allow(dead_code)]
    pub fn confirm_with<F>(&self, target: &T, mut action: F)
    where
        F: FnMut(),
    {
        if self.is_armed(target) {
            action();
            self.reset();
        }
    }

    #[allow(dead_code)]
    pub fn reset(&self) {
        self.armed.set(None);
    }

    #[allow(dead_code)]
    pub fn signal(&self) -> RwSignal<Option<T>> {
        self.armed
    }
}

#[allow(dead_code)]
pub fn use_two_step_delete<T>() -> TwoStepDeleteController<T>
where
    T: PartialEq + Clone + 'static,
{
    TwoStepDeleteController::new()
}
