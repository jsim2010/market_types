//! Implements [`Producer`]s and [`Consumer`]s for queues.
use {
    alloc::{string::String, sync::Arc},
    core::fmt::{self, Display, Formatter},
    crossbeam_queue::{ArrayQueue, SegQueue},
    fehler::throws,
    market::{
        queue::{FiniteQueue, InfiniteQueue},
        Agent, Consumer, EmptyStock, Failure, Fault, Flawless, FullStock, Producer, Recall,
    },
};

/// A [`InfiniteQueue`] implemented by [`crossbeam`].
#[derive(Debug)]
pub struct CrossbeamInfiniteQueue<G> {
    /// The name of the queue.
    name: String,
    /// The queue.
    queue: Arc<SegQueue<G>>,
}

impl<G> Agent for CrossbeamInfiniteQueue<G> {
    type Good = G;
}

impl<G> Consumer for CrossbeamInfiniteQueue<G> {
    type Flaws = EmptyStock;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        self.queue
            .pop()
            .ok_or_else(|| Consumer::failure(self, Fault::Insufficiency(EmptyStock::default())))?
    }
}

impl<G> Display for CrossbeamInfiniteQueue<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<G> InfiniteQueue<G> for CrossbeamInfiniteQueue<G> {
    fn allocate<S>(name_str: &S) -> Self
    where
        S: AsRef<str> + ?Sized,
    {
        Self {
            name: String::from(name_str.as_ref()),
            queue: Arc::new(SegQueue::new()),
        }
    }
}

impl<G> Producer for CrossbeamInfiniteQueue<G> {
    type Flaws = Flawless;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        self.queue.push(good);
    }
}

/// A [`FiniteQueue`] implemented by [`crossbeam`].
#[derive(Debug)]
pub struct CrossbeamFiniteQueue<G> {
    /// The name of the queue.
    name: String,
    /// The queue.
    queue: Arc<ArrayQueue<G>>,
}

impl<G> Agent for CrossbeamFiniteQueue<G> {
    type Good = G;
}

impl<G> Consumer for CrossbeamFiniteQueue<G> {
    type Flaws = EmptyStock;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        self.queue
            .pop()
            .ok_or_else(|| Consumer::failure(self, Fault::Insufficiency(EmptyStock::default())))?
    }
}

impl<G> Display for CrossbeamFiniteQueue<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<G> FiniteQueue<G> for CrossbeamFiniteQueue<G> {
    fn allocate<S>(name_str: &S, size: usize) -> Self
    where
        S: AsRef<str> + ?Sized,
    {
        Self {
            name: String::from(name_str.as_ref()),
            queue: Arc::new(ArrayQueue::new(size)),
        }
    }
}

impl<G> Producer for CrossbeamFiniteQueue<G> {
    type Flaws = FullStock;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        self.queue
            .push(good)
            .map_err(|error| self.recall(Fault::Insufficiency(FullStock::default()), error))?
    }
}
