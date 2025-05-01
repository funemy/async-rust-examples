pub trait Repo<'a, T: 'a>: Sized {
    fn insert(&mut self, e: T);
    fn iter(&'a self) -> impl Iterator<Item = &'a T>;
    fn iter_mut(&'a mut self) -> impl Iterator<Item = &'a mut T>;
    fn partition(self, predicate: impl Fn(&T) -> bool) -> (Self, Self);
}

#[derive(Clone, Debug)]
pub struct RVec<T>(Vec<T>);

impl<T> Default for RVec<T> {
    fn default() -> Self {
        RVec(vec![])
    }
}

impl<T> Extend<T> for RVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<T> IntoIterator for RVec<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T: 'a> Repo<'a, T> for RVec<T> {
    fn insert(&mut self, e: T) {
        self.0.push(e);
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.0.iter_mut()
    }

    fn partition(self, predicate: impl Fn(&T) -> bool) -> (Self, Self) {
        self.0.into_iter().partition(predicate)
    }
}
