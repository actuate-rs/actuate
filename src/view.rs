use crate::{Scope, ViewBuilder};

pub trait View: 'static {
    fn body(&self, cx: &Scope) -> impl ViewBuilder;
}
