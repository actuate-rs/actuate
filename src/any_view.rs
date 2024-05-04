use crate::View;
use std::{any::Any, borrow::Cow};

pub trait AnyView {
    fn name(&self) -> Cow<'static, str>;

    fn as_any(&self) -> &dyn Any;

    fn any_eq(&self, rhs: &dyn Any) -> bool;

    fn view_any(&self) -> Box<dyn AnyView>;
}

impl<V: View> AnyView for V {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<V>())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn any_eq(&self, rhs: &dyn Any) -> bool {
        self == rhs.downcast_ref::<V>().unwrap()
    }

    fn view_any(&self) -> Box<dyn AnyView> {
        Box::new(self.view())
    }
}
