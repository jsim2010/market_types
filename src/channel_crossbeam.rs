//! Implements [`Producer`] and [`Consumer`] for channels implemented by [`crossbeam`].
use {
    alloc::string::String,
    core::{
        fmt::{self, Display, Formatter},
        marker::PhantomData,
    },
    crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError},
    fehler::throws,
    market::{
        channel::{FiniteChannel, InfiniteChannel, WithdrawnDemand, WithdrawnSupply},
        Agent, Consumer, ConsumptionFlaws, EmptyStock, Failure, Fault, FullStock, Producer,
        ProductionFlaws, Recall,
    },
};

/// Implements [`Consumer`] for goods of type `G` from a crossbeam channel.
#[derive(Debug)]
pub struct CrossbeamReceiver<G> {
    /// Describes the channel.
    name: String,
    /// The [`Receiver`] of the channel.
    receiver: Receiver<G>,
}

impl<G> Agent for CrossbeamReceiver<G> {
    type Good = G;
}

impl<G> Consumer for CrossbeamReceiver<G> {
    type Flaws = ConsumptionFlaws<WithdrawnSupply>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        self.receiver.try_recv().map_err(|error| match error {
            TryRecvError::Empty => self.failure(Fault::Insufficiency(EmptyStock::default())),
            TryRecvError::Disconnected => self.failure(Fault::Defect(WithdrawnSupply::default())),
        })?
    }
}

impl<G> Display for CrossbeamReceiver<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The [`Producer`] for the implementation of [`FiniteChannel`] for [`crossbeam`].
#[derive(Debug)]
pub struct CrossbeamFiniteSender<G> {
    /// Describes the channel.
    name: String,
    /// The sender.
    sender: Sender<G>,
}

impl<G> Agent for CrossbeamFiniteSender<G> {
    type Good = G;
}

impl<G> Display for CrossbeamFiniteSender<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<G> Producer for CrossbeamFiniteSender<G> {
    type Flaws = ProductionFlaws<WithdrawnDemand>;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        self.sender.try_send(good).map_err(|error| match error {
            TrySendError::Full(good) => {
                self.recall(Fault::Insufficiency(FullStock::default()), good)
            }
            TrySendError::Disconnected(good) => {
                self.recall(Fault::Defect(WithdrawnDemand::default()), good)
            }
        })?
    }
}

/// The [`Producer`] for the implementation of [`InfiniteChannel`] for [`crossbeam`].
#[derive(Debug)]
pub struct CrossbeamInfiniteSender<G> {
    /// The name of the sender.
    name: String,
    /// The sender.
    sender: Sender<G>,
}

impl<G> Agent for CrossbeamInfiniteSender<G> {
    type Good = G;
}

impl<G> Display for CrossbeamInfiniteSender<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<G> Producer for CrossbeamInfiniteSender<G> {
    type Flaws = WithdrawnDemand;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        self.sender.send(good).map_err(|error| {
            self.recall(
                Fault::Defect(WithdrawnDemand::default()),
                error.into_inner(),
            )
        })?
    }
}

/// The [`InfiniteChannel`] implemented by [`crossbeam`].
#[derive(Debug)]
pub struct CrossbeamInfiniteChannel<G> {
    /// The tpe of the good that is exhanged on the channel.
    good: PhantomData<G>,
}

impl<G> InfiniteChannel<G> for CrossbeamInfiniteChannel<G> {
    type Producer = CrossbeamInfiniteSender<G>;
    type Consumer = CrossbeamReceiver<G>;

    fn establish<S>(name_str: &S) -> (Self::Producer, Self::Consumer)
    where
        S: AsRef<str> + ?Sized,
    {
        let name = String::from(name_str.as_ref());
        let (sender, receiver) = crossbeam_channel::unbounded();
        (
            CrossbeamInfiniteSender {
                name: name.clone(),
                sender,
            },
            CrossbeamReceiver { name, receiver },
        )
    }
}

/// The [`FiniteChannel`] implemented by [`crossbeam`].
#[derive(Debug)]
pub struct CrossbeamFiniteChannel<G> {
    /// The type of the good that is exchanged on the channel.
    good: PhantomData<G>,
}

impl<G> FiniteChannel<G> for CrossbeamFiniteChannel<G> {
    type Producer = CrossbeamFiniteSender<G>;
    type Consumer = CrossbeamReceiver<G>;

    fn establish<S>(name_str: &S, size: usize) -> (Self::Producer, Self::Consumer)
    where
        S: AsRef<str> + ?Sized,
    {
        let name = String::from(name_str.as_ref());
        let (sender, receiver) = crossbeam_channel::bounded(size);
        (
            CrossbeamFiniteSender {
                name: name.clone(),
                sender,
            },
            CrossbeamReceiver { name, receiver },
        )
    }
}
