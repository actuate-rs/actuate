use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::VectorSpace;
use bevy_time::Time;
use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

/// Use an animated value.
pub fn use_animated<T: VectorSpace + 'static>(
    cx: ScopeState,
    make_initial: impl FnOnce() -> T,
) -> UseAnimated<T> {
    let start_cell = use_world_once(cx, |time: Res<Time>| Cell::new(Some(time.elapsed_secs())));

    let (tx, rx) = use_ref(cx, || {
        let (tx, rx) = mpsc::unbounded_channel();
        (tx, Cell::new(Some(rx)))
    });

    let state: &RefCell<Option<(T, T, Duration, Option<oneshot::Sender<()>>)>> =
        use_ref(cx, || RefCell::new(None));

    let out = use_mut(cx, make_initial);

    let time_cell = use_ref(cx, || Cell::new(start_cell.get().unwrap()));
    use_world(cx, |time_res: Res<Time>| {
        time_cell.set(time_res.elapsed_secs());
    });

    use_local_task(cx, move || async move {
        let mut rx = rx.take().unwrap();
        while let Some((to, duration, tx)) = rx.recv().await {
            *state.borrow_mut() = Some((*out, to, duration, Some(tx)));
            start_cell.set(Some(time_cell.get()));
        }
    });

    use_world(cx, move |time: Res<Time>| {
        if let Some(start) = start_cell.get() {
            let mut state = state.borrow_mut();
            if let Some((from, to, duration, oneshot)) = &mut *state {
                let elapsed = time.elapsed_secs() - start;

                if elapsed < duration.as_secs_f32() {
                    SignalMut::set(out, from.lerp(*to, elapsed / duration.as_secs_f32()));
                } else {
                    SignalMut::set(out, *to);
                    oneshot.take().unwrap().send(()).unwrap();
                    *state = None;
                }
            }
        }
    });

    UseAnimated {
        value: SignalMut::as_ref(out),
        tx,
    }
}

/// Hook for [`use_animated`].
pub struct UseAnimated<'a, T> {
    value: Signal<'a, T>,
    tx: &'a mpsc::UnboundedSender<(T, Duration, oneshot::Sender<()>)>,
}

impl<T> UseAnimated<'_, T> {
    /// Animate this value over a duration.
    pub async fn animate(&self, to: T, duration: Duration) {
        let (tx, rx) = oneshot::channel();
        self.tx.send((to, duration, tx)).unwrap();
        rx.await.unwrap()
    }
}

impl<T> Clone for UseAnimated<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for UseAnimated<'_, T> {}

impl<T> Deref for UseAnimated<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
