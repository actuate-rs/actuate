use std::any::Any;

pub trait Stack {
    fn push(&mut self, element: Box<dyn Any>);

    fn update(&mut self) -> &mut dyn Any;

    fn skip(&mut self, n: usize);

    fn remove(&mut self, n: usize);

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct VecStack<T> {
    pub items: Vec<T>,
    pub idx: usize,
}

impl<T> Default for VecStack<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            idx: 0,
        }
    }
}

impl<T: 'static> Stack for VecStack<T> {
    fn push(&mut self, element: Box<dyn Any>) {
        self.items.push(*element.downcast().unwrap());
        self.idx += 1;
    }

    fn update(&mut self) -> &mut dyn Any {
        let idx = self.idx;
        self.idx += 1;

        self.items.get_mut(idx).unwrap()
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn remove(&mut self, n: usize) {
        for i in 0..n {
            self.items.remove(self.idx + i);
        }
        self.idx += n;
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}
