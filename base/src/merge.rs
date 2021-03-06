use std::iter::FusedIterator;

pub fn merge3<F, A: ?Sized, B: ?Sized, C: ?Sized, R>(
    a_original: &A,
    a: Option<A::Owned>,
    b_original: &B,
    b: Option<B::Owned>,
    c_original: &C,
    c: Option<C::Owned>,
    f: F,
) -> Option<R>
where
    A: ToOwned,
    B: ToOwned,
    C: ToOwned,
    F: FnOnce(A::Owned, B::Owned, C::Owned) -> R,
{
    let a_b = merge(a_original, a, b_original, b, |a, b| (a, b));
    merge_fn(
        &(a_original, b_original),
        |_| (a_original.to_owned(), b_original.to_owned()),
        a_b,
        c_original,
        C::to_owned,
        c,
        |(a, b), c| f(a, b, c),
    )
}

/// Merges two values using `f` if either or both them is `Some(..)`.
/// If both are `None`, `None` is returned.
pub fn merge<F, A: ?Sized, B: ?Sized, R>(
    a_original: &A,
    a: Option<A::Owned>,
    b_original: &B,
    b: Option<B::Owned>,
    f: F,
) -> Option<R>
where
    A: ToOwned,
    B: ToOwned,
    F: FnOnce(A::Owned, B::Owned) -> R,
{
    merge_fn(a_original, A::to_owned, a, b_original, B::to_owned, b, f)
}

pub fn merge_fn<'a, 'b, F, G, H, A: ?Sized, B: ?Sized, A1, B1, R>(
    a_original: &'a A,
    g: G,
    a: Option<A1>,
    b_original: &'b B,
    h: H,
    b: Option<B1>,
    merger: F,
) -> Option<R>
where
    F: FnOnce(A1, B1) -> R,
    G: FnOnce(&'a A) -> A1,
    H: FnOnce(&'b B) -> B1,
{
    match (a, b) {
        (Some(a), Some(b)) => Some(merger(a, b)),
        (Some(a), None) => Some(merger(a, h(b_original))),
        (None, Some(b)) => Some(merger(g(a_original), b)),
        (None, None) => None,
    }
}

pub fn merge_tuple_iter<'a, I, F, T, R>(types: I, mut f: F) -> Option<R>
where
    I: IntoIterator<Item = (&'a T, &'a T)>,
    I::IntoIter: FusedIterator + Clone,
    F: FnMut(&'a T, &'a T) -> Option<T>,
    T: Clone + 'a,
    R: std::iter::FromIterator<T>,
{
    merge_collect(types, |(l, r)| f(l, r), |(l, _)| l.clone())
}

pub struct MergeIter<I, F, G, T> {
    types: I,
    clone_types_iter: I,
    action: F,
    converter: G,
    clone_types: usize,
    next: Option<T>,
}

impl<I, F, G, U> Iterator for MergeIter<I, F, G, U>
where
    I: Iterator,
    F: FnMut(I::Item) -> Option<U>,
    G: FnMut(I::Item) -> U,
{
    type Item = U;
    fn next(&mut self) -> Option<Self::Item> {
        if self.clone_types > 0 {
            self.clone_types -= 1;
            self.clone_types_iter.next().map(&mut self.converter)
        } else if let Some(typ) = self.next.take() {
            self.clone_types_iter.next();
            Some(typ)
        } else {
            let action = &mut self.action;
            if let Some((i, typ)) = self
                .types
                .by_ref()
                .enumerate()
                .find_map(|(i, typ)| action(typ).map(|typ| (i, typ)))
            {
                self.clone_types = i;
                self.next = Some(typ);
                self.next()
            } else {
                self.clone_types = usize::max_value();
                self.next()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.clone_types_iter.size_hint()
    }
}

impl<I, F, G, U> ExactSizeIterator for MergeIter<I, F, G, U>
where
    I: ExactSizeIterator,
    F: FnMut(I::Item) -> Option<U>,
    G: FnMut(I::Item) -> U,
{
    fn len(&self) -> usize {
        self.clone_types_iter.len()
    }
}

pub fn merge_collect<I, F, G, U, R>(types: I, action: F, converter: G) -> Option<R>
where
    I: IntoIterator,
    I::IntoIter: FusedIterator + Clone,
    F: FnMut(I::Item) -> Option<U>,
    G: FnMut(I::Item) -> U,
    R: std::iter::FromIterator<U>,
{
    merge_iter(types, action, converter).map(|iter| iter.collect())
}

pub fn merge_iter<I, F, G, U>(
    types: I,
    mut action: F,
    converter: G,
) -> Option<MergeIter<I::IntoIter, F, G, U>>
where
    I: IntoIterator,
    I::IntoIter: FusedIterator + Clone,
    F: FnMut(I::Item) -> Option<U>,
    G: FnMut(I::Item) -> U,
{
    let mut types = types.into_iter();
    let clone_types_iter = types.clone();
    if let Some((i, typ)) = types
        .by_ref()
        .enumerate()
        .find_map(|(i, typ)| action(typ).map(|typ| (i, typ)))
    {
        Some(MergeIter {
            clone_types_iter,
            types,
            action,
            converter,
            clone_types: i,
            next: Some(typ),
        })
    } else {
        None
    }
}
