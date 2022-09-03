use std::{hash::Hash, marker::PhantomData};

use super::*;
use search_tree::*;

pub unsafe trait TranspositionTable<Spec: MCTS>: Sync + Sized {
    /// **If this function inserts a value, it must return `None`.** Failure to follow
    /// this rule will lead to memory safety violation.
    ///
    /// Attempts to insert a key/value pair.
    ///
    /// If the key is not present, the table *may* insert it. If the table does
    /// not insert it, the table may either return `None` or a reference to another
    /// value existing in the table. (The latter is allowed so that the table doesn't
    /// necessarily need to handle hash collisions, but it will negatively affect the accuracy
    /// of the search.)
    ///
    /// If the key is present, the table may either:
    /// - Leave the table unchanged and return `Some(reference to associated value)`.
    /// - Leave the table unchanged and return `None`.
    ///
    /// The table *may* choose to replace old values.
    /// The table is *not* responsible for dropping values that are replaced.
    fn insert<'a>(
        &'a self,
        key: &Spec::State,
        value: &'a SearchNode<Spec>,
    ) -> Option<&'a SearchNode<Spec>>;

    /// Looks up a key.
    ///
    /// If the key is not present, the table *should almost always* return `None`.
    ///
    /// If the key is present, the table *may return either* `None` or a reference
    /// to the associated value.
    fn lookup<'a>(&'a self, key: &Spec::State) -> Option<&'a SearchNode<Spec>>;
}

unsafe impl<Spec: MCTS<TranspositionTable = Self>> TranspositionTable<Spec> for () {
    fn insert<'a>(
        &'a self,
        _: &Spec::State,
        _: &'a SearchNode<Spec>,
    ) -> Option<&'a SearchNode<Spec>> {
        None
    }

    fn lookup<'a>(&'a self, _: &Spec::State) -> Option<&'a SearchNode<Spec>> {
        None
    }
}

pub struct LockFreeHashTable<K, V> {
    inner: lockfree::map::Map<K, usize>,
    _marker: PhantomData<V>,
}

impl<K, V> LockFreeHashTable<K, V> {
    pub fn new() -> Self {
        Self {
            inner: lockfree::map::Map::<K, usize>::new(),
            _marker: PhantomData,
        }
    }
}

pub type ApproxTable<Spec> = LockFreeHashTable<<Spec as MCTS>::State, SearchNode<Spec>>;

unsafe impl<Spec> TranspositionTable<Spec> for ApproxTable<Spec>
where
    Spec::State: Ord + Hash,
    Spec: MCTS,
{
    fn insert<'a>(
        &'a self,
        key: &Spec::State,
        value: &'a SearchNode<Spec>,
    ) -> Option<&'a SearchNode<Spec>> {
        let value = unsafe { mem::transmute::<_, usize>(value) };

        match self.inner.insert(key.clone(), value) {
            Some(removed) => unsafe { Some(mem::transmute::<_, &SearchNode<Spec>>(removed.1)) },
            None => None,
        }
    }

    fn lookup<'a>(&'a self, key: &Spec::State) -> Option<&'a SearchNode<Spec>> {
        match self.inner.get(key) {
            Some(value) => unsafe { Some(mem::transmute::<_, &SearchNode<Spec>>(value.1)) },
            None => None,
        }
    }
}
