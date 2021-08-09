//! Implements [`Consumer`] that composes consumed elements into a composite.
use {
    alloc::vec::Vec,
    core::{
        cell::RefCell,
        fmt::{self, Display, Formatter},
        marker::PhantomData,
        task::Poll,
    },
    fehler::{throw, throws},
    market::{Agent, Blame, Consumer, Failure, Fault, Flaws},
};

/// Characterizes an item that can be composed from a sequence of elements.
pub trait Composite<E> {
    /// Specifies the error thrown when a composition attempt fails.
    type Misstep;

    /// Attempts to create a `Self` from `elements`.
    #[throws(Self::Misstep)]
    fn compose(elements: &mut Vec<E>) -> Poll<Self>
    where
        Self: Sized;
}

/// The fault thrown by [`Composer`].
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ComposeDefect<D, M> {
    /// A defect during consumption.
    Consume(D),
    /// A misstep during composition.
    Compose(M),
}

impl<D: Display, M: Display> Display for ComposeDefect<D, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Consume(ref fault) => write!(f, "{}", fault),
            Self::Compose(ref misstep) => write!(f, "{}", misstep),
        }
    }
}

impl<D, M> From<D> for ComposeDefect<D, M> {
    fn from(defect: D) -> Self {
        Self::Consume(defect)
    }
}

/// Specifies the [`Flaws`] for [`Composer`].
#[derive(Debug)]
pub struct CompositionFlaws<F, M> {
    /// The type of the [`Flaws`].
    _flaws: PhantomData<F>,
    /// The type of the Misstep.
    _misstep: PhantomData<M>,
}

impl<F: Flaws, M> Flaws for CompositionFlaws<F, M> {
    type Insufficiency = F::Insufficiency;
    type Defect = ComposeDefect<F::Defect, M>;
}

/// A [`Consumer`] that converts consumed elements into a composite.
#[derive(Debug)]
pub struct Composer<E, G, C> {
    /// The current sequence of elements that should make the beginning of a composite.
    elements: RefCell<Vec<E>>,
    /// The [`Consumer`] of the elements.
    consumer: C,
    /// The [`Composite`]
    _composite: PhantomData<G>,
}

impl<E, G, C> Composer<E, G, C> {
    /// Creates a new [`Composer`].
    pub const fn new(consumer: C) -> Self {
        Self {
            elements: RefCell::new(Vec::new()),
            consumer,
            _composite: PhantomData,
        }
    }
}

impl<E, G, C> Agent for Composer<E, G, C>
where
    C: Consumer,
{
    type Good = G;
}

impl<E, G, C> Consumer for Composer<E, G, C>
where
    G: Composite<E>,
    C: Consumer<Good = E>,
{
    type Flaws = CompositionFlaws<C::Flaws, G::Misstep>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        let mut elements = self.elements.borrow_mut();

        // Consume until a failure while keeping all the successfully consumed goods.
        let failure = loop {
            match self.consumer.consume() {
                Ok(good) => {
                    elements.push(good);
                }
                Err(failure) => {
                    break failure;
                }
            }
        };

        match G::compose(&mut *elements)
            .map_err(|misstep| self.failure(Fault::Defect(ComposeDefect::Compose(misstep))))?
        {
            Poll::Ready(composite) => composite,
            Poll::Pending => {
                throw!(failure.blame())
            }
        }
    }
}

impl<E, G, C> Display for Composer<E, G, C>
where
    C: Consumer,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Composer of {}", self.consumer)
    }
}
