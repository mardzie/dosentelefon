use std::ops::DerefMut;

pub trait CopyFromIterator<T> {
    fn copy_from_iter<I: IntoIterator<Item = T>>(&mut self, iter: I);
}

impl<T, D: DerefMut<Target = [T]>> CopyFromIterator<T> for D {
    /// Fills `self` with copies of `I`s elements.
    fn copy_from_iter<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut iterator = iter.into_iter();
        for slot in self.iter_mut() {
            if let Some(value) = iterator.next() {
                *slot = value;
            }
        }
    }
}

impl<T> CopyFromIterator<T> for [T] {
    /// Fills `self` with copies of `I`s elements.
    fn copy_from_iter<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let mut iterator = iter.into_iter();
        for slot in self.iter_mut() {
            if let Some(value) = iterator.next() {
                *slot = value;
            }
        }
    }
}
