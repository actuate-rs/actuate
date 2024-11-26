use crate::prelude::*;
use std::collections::HashMap;

pub use actuate_macros::{data, Data};

/// Composable data.
///
/// For most cases, this trait should be derived with `#[derive(Data)]`.
///
/// # Safety
/// This struct must ensure the lifetime of the data it holds cannot escape while composing children.
///
/// For example, a `RefCell<&'a T>` is unsafe because the compiler will infer the lifetime of a child composable's lifetime (e.g. `'a`)
/// as this struct's lifetime (e.g. `'a`).
pub unsafe trait Data {}

macro_rules! impl_data_for_std {
    ($($t:ty),*) => {
        $(
            unsafe impl Data for $t {}
        )*
    }
}

impl_data_for_std!(
    (),
    bool,
    char,
    f32,
    f64,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    String
);

unsafe impl Data for &str {}

unsafe impl<T: Data> Data for Vec<T> {}

unsafe impl<T: Data, U: Data> Data for HashMap<T, U> {}

unsafe impl<T: Data> Data for &T {}

unsafe impl<T: Data> Data for Option<T> {}

unsafe impl Data for DynCompose<'_> {}

#[doc(hidden)]
pub struct FieldWrap<T>(pub T);

#[doc(hidden)]
pub unsafe trait StateField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StateField for FieldWrap<&T> {}

#[doc(hidden)]
pub unsafe trait FnField<Marker> {
    fn check(&self) {
        let _ = self;
    }
}

macro_rules! impl_data_for_fns {
    ($($t:tt),*) => {
        unsafe impl<$($t: 'static,)* F: Fn($($t,)*)> FnField<fn($($t,)*)> for &FieldWrap<F> {}
    }
}

impl_data_for_fns!();
impl_data_for_fns!(T1);
impl_data_for_fns!(T1, T2);
impl_data_for_fns!(T1, T2, T3);
impl_data_for_fns!(T1, T2, T3, T4);
impl_data_for_fns!(T1, T2, T3, T4, T5);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6, T7);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6, T7, T8);

#[doc(hidden)]
pub unsafe trait DataField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: Data> DataField for &FieldWrap<T> {}

#[doc(hidden)]
pub unsafe trait StaticField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StaticField for &&FieldWrap<T> {}
