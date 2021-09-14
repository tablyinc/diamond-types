use std::fmt::Debug;

pub mod splitable_span;
pub mod merge_iter;

pub trait Searchable {
    type Item: Copy + Debug;

    // This is strictly unnecessary given truncate(), but it makes some code cleaner.
    // fn truncate_keeping_right(&mut self, at: usize) -> Self;

    /// Checks if the entry contains the specified item. If it does, returns the offset into the
    /// item.
    fn contains(&self, loc: Self::Item) -> Option<usize>;

    // I'd use Index for this but the index trait returns a reference.
    // fn at_offset(&self, offset: usize) -> Self::Item;
    fn at_offset(&self, offset: usize) -> Self::Item;
}
