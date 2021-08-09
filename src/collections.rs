//! Implements [`Producer`] and [`Consumer`] for sequences of agents.
use {
    crate::convert::{Adapter, SpecificationDefect, SpecificationFlaws, Specifier},
    alloc::{boxed::Box, string::String, vec::Vec},
    core::{
        cmp::Eq,
        convert::TryFrom,
        fmt::{self, Debug, Display, Formatter},
        marker::PhantomData,
    },
    fehler::{throw, throws},
    market::{
        Agent, Blame, Consumer, ConsumptionFlaws, EmptyStock, Failure, Fault, Flaws, Producer,
        Recall,
    },
    std::{collections::HashMap, hash::Hash},
};

/// The [`Flaws`] of a [`Distributor`].
#[derive(Debug)]
pub struct DistributionFlaws<F> {
    /// The type of the [`Flaws`].
    _flaws: PhantomData<F>,
}

impl<F> Flaws for DistributionFlaws<F>
where
    F: Flaws,
{
    type Insufficiency = F::Insufficiency;
    type Defect = DistributionDefect<F::Defect>;
}

/// The error thrown by a [`Distributor`].
#[derive(Debug)]
#[non_exhaustive]
pub enum DistributionDefect<F> {
    /// The [`Specifier`] failed.
    Specification(SpecificationDefect<F>),
    /// The given key was not found in the [`Distributor`].
    MissingKey,
}

impl<F> From<SpecificationDefect<F>> for DistributionDefect<F> {
    fn from(defect: SpecificationDefect<F>) -> Self {
        Self::Specification(defect)
    }
}

/// Characterizes an item that can be referenced by a key.
pub trait Keyed<K> {
    /// Returns a reference to the key of `self`.
    fn key(&self) -> &K;
}

/// A [`Producer`] that produces goods of type `G` to multiple [`Producer`]s.
pub struct Distributor<K, G, F> {
    // The Good of each Producer must be G so that all Producers have the same interface and can be elements of a single HashMap.
    /// The [`Producer`]s.
    producers: HashMap<K, Box<dyn Producer<Good = G, Flaws = SpecificationFlaws<F>>>>,
    /// The name of the distributor.
    name: String,
}

impl<K, G: 'static, F: Flaws + 'static> Distributor<K, G, F> {
    /// Creates a new, empty [`Distributor`].
    #[must_use]
    pub fn new<S>(name_str: &S) -> Self
    where
        S: AsRef<str> + ?Sized,
    {
        Self {
            producers: HashMap::new(),
            name: String::from(name_str.as_ref()),
        }
    }

    /// Inserts a mapping of `producer` to `key` into `self`.
    pub fn insert<P: Producer + 'static>(
        &mut self,
        key: K,
        producer: P,
    ) -> Option<Box<dyn Producer<Good = G, Flaws = SpecificationFlaws<F>>>>
    where
        K: Eq + Hash,
        P::Good: TryFrom<G, Error = G>,
        G: From<P::Good>,
        SpecificationDefect<F::Defect>: From<<P::Flaws as Flaws>::Defect>,
        F::Insufficiency: From<<P::Flaws as Flaws>::Insufficiency>,
    {
        self.producers
            .insert(key, Box::new(Specifier::new(producer)))
    }
}

impl<K, G, F> Agent for Distributor<K, G, F> {
    type Good = G;
}

impl<K, G, F> Debug for Distributor<K, G, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Distributor {{ .. }}")
    }
}

impl<K, G, F> Display for Distributor<K, G, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<K, G, F> Producer for Distributor<K, G, F>
where
    F: Flaws,
    G: Keyed<K>,
    K: Eq + Hash,
{
    type Flaws = DistributionFlaws<F>;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        if let Some(producer) = self.producers.get(good.key()) {
            producer.produce(good).map_err(|recall| recall.blame())?;
        } else {
            throw!(self.recall(Fault::Defect(DistributionDefect::MissingKey), good));
        }
    }
}

/// A [`Consumer`] that consumes goods of type `G` from multiple [`Consumer`]s.
pub struct Collector<G, F> {
    /// The [`Consumer`]s.
    consumers: Vec<Box<dyn Consumer<Good = G, Flaws = F>>>,
    /// The name of the collector.
    name: String,
}

impl<G, F: Flaws> Collector<G, F> {
    /// Creates a new, empty [`Collector`].
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            consumers: Vec::new(),
            name,
        }
    }

    /// Adds `consumer` to the end of the [`Consumer`]s held by `self`.
    pub fn push<C: Consumer + 'static>(&mut self, consumer: C)
    where
        F: 'static,
        G: From<C::Good> + 'static,
        F::Insufficiency: From<<C::Flaws as Flaws>::Insufficiency>,
        F::Defect: From<<C::Flaws as Flaws>::Defect>,
    {
        self.consumers.push(Box::new(Adapter::new(consumer)));
    }
}

impl<G, F> Agent for Collector<G, F> {
    type Good = G;
}

impl<G, F: Flaws> Consumer for Collector<G, F>
where
    F::Defect: Flaws,
    EmptyStock: From<F::Insufficiency>,
    <F::Defect as Flaws>::Defect: From<F::Defect>,
{
    type Flaws = ConsumptionFlaws<F::Defect>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        let mut result: Result<Self::Good, Failure<Self::Flaws>> =
            Err(self.failure(Fault::Insufficiency(EmptyStock::default())));

        for consumer in &self.consumers {
            result = consumer.consume().map_err(|failure| failure.blame());

            if let Err(ref failure) = result {
                if failure.is_defect() {
                    break;
                }
            } else {
                break;
            }
        }

        result?
    }
}

impl<G, F> Debug for Collector<G, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Collector {{ .. }}")
    }
}

impl<G, F> Display for Collector<G, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Collector for {}", self.name)
    }
}
