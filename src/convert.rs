//! Implements a [`Producer`] and [`Consumer`] that converts its goods and [`Fault`]s.
use {
    core::{
        convert::TryFrom,
        fmt::{self, Display, Formatter},
        marker::PhantomData,
    },
    fehler::{throw, throws},
    market::{Agent, Blame, Consumer, Failure, Fault, Flaws, Producer, Recall},
};

/// A defect thrown when an attempt to specify and produce fails.
#[derive(Debug)]
#[non_exhaustive]
pub enum SpecificationDefect<D> {
    /// The good could not be specified.
    Unspecifiable,
    /// The specified good could not be produced.
    Improducible(D),
}

/// The [`Flaws`] defining errors thrown when a specifier attempts to specify and produce a good on a [`Producer`] with [`Flaws`] `F`.
#[derive(Debug)]
pub struct SpecificationFlaws<F> {
    /// The type of the [`Flaws`].
    _flaws: PhantomData<F>,
}

impl<F> Flaws for SpecificationFlaws<F>
where
    F: Flaws,
{
    type Insufficiency = F::Insufficiency;
    type Defect = SpecificationDefect<F::Defect>;
}

/// A [`Producer`] that converts its good and [`Fault`] for the producer `P`.
///
/// Converts the generic good `G` to a specific good `P::Good` and converts the specific [`Recall`] thrown by `P` to the generic `Recall<SpecificationFlaws<F>, G>`.
#[derive(Debug)]
pub struct Specifier<P, G, F> {
    /// The actual producer.
    producer: P,
    /// The type of the original good.
    _good: PhantomData<G>,
    /// The type of self's [`Flaws`].
    _flaws: PhantomData<F>,
}

impl<P, G, F> Specifier<P, G, F> {
    /// Creates a new [`Specifier`].
    pub const fn new(producer: P) -> Self {
        Self {
            producer,
            _good: PhantomData,
            _flaws: PhantomData,
        }
    }
}

impl<P, G, F> Agent for Specifier<P, G, F>
where
    P: Producer,
{
    type Good = G;
}

impl<P, G, F> Display for Specifier<P, G, F>
where
    P: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Specifier for {}", self.producer)
    }
}

impl<P, G, F> Producer for Specifier<P, G, F>
where
    P: Producer,
    G: From<P::Good>,
    F: Flaws,
    P::Good: TryFrom<G, Error = G>,
    F::Insufficiency: From<<P::Flaws as Flaws>::Insufficiency>,
    SpecificationDefect<F::Defect>: From<<P::Flaws as Flaws>::Defect>,
{
    type Flaws = SpecificationFlaws<F>;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        match P::Good::try_from(good) {
            Ok(converted_good) => self
                .producer
                .produce(converted_good)
                .map_err(|recall| recall.blame())?,
            Err(original_good) => throw!(self.recall(
                Fault::Defect(SpecificationDefect::Unspecifiable),
                original_good
            )),
        }
    }
}

/// A [`Consumer`] that converts the good and [`Fault`] for the consumer `C`.
///
/// Converts the specific good `C::Good` into `G` and `Failure<C::Flaws>` into `F`.
#[derive(Debug)]
pub struct Adapter<C, G, F> {
    /// The original consumer.
    consumer: C,
    /// The desired type of good.
    _good: PhantomData<G>,
    /// The desired type of [`Flaws`].
    _flaws: PhantomData<F>,
}

impl<C, G, F> Adapter<C, G, F> {
    /// Creates a new [`Adapter`].
    pub const fn new(consumer: C) -> Self {
        Self {
            consumer,
            _good: PhantomData,
            _flaws: PhantomData,
        }
    }
}

impl<C, G, F> Agent for Adapter<C, G, F>
where
    C: Consumer,
{
    type Good = G;
}

impl<C, G, F> Consumer for Adapter<C, G, F>
where
    C: Consumer,
    G: From<C::Good>,
    F: Flaws,
    F::Insufficiency: From<<C::Flaws as Flaws>::Insufficiency>,
    F::Defect: From<<C::Flaws as Flaws>::Defect>,
{
    type Flaws = F;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        self.consumer
            .consume()
            .map(Self::Good::from)
            .map_err(|failure| failure.blame())?
    }
}

impl<C, G, F> Display for Adapter<C, G, F>
where
    C: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Adapter for {}", self.consumer)
    }
}
