//! Implementation of [`Channel`] for [`std::sync::mpsc`].
use {
    alloc::string::String,
    core::{
        fmt::{self, Display, Formatter},
        marker::PhantomData,
    },
    fehler::throws,
    market::{
        channel::{FiniteChannel, InfiniteChannel, WithdrawnDemand, WithdrawnSupply},
        Agent, Consumer, ConsumptionFlaws, EmptyStock, Failure, Fault, FullStock, Producer,
        ProductionFlaws, Recall,
    },
    std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError, TrySendError},
};

/// Implements [`Consumer`] for goods of type `G` from a channel created by [`mpsc`].
#[derive(Debug)]
pub struct StdReceiver<G> {
    /// Describes the channel.
    name: String,
    /// The [`Receiver`] of the channel.
    receiver: Receiver<G>,
}

impl<G> Agent for StdReceiver<G> {
    type Good = G;
}

impl<G> Consumer for StdReceiver<G> {
    type Flaws = ConsumptionFlaws<WithdrawnSupply>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        self.receiver.try_recv().map_err(|error| match error {
            TryRecvError::Empty => self.failure(Fault::Insufficiency(EmptyStock::default())),
            TryRecvError::Disconnected => self.failure(Fault::Defect(WithdrawnSupply::default())),
        })?
    }
}

impl<G> Display for StdReceiver<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The [`Producer`] for [`InfiniteChannel`] implemented by [`std`].
#[derive(Debug)]
pub struct StdSender<G> {
    /// The name of the sender.
    name: String,
    /// The sender.
    sender: Sender<G>,
}

impl<G> Agent for StdSender<G> {
    type Good = G;
}

impl<G> Display for StdSender<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<G> Producer for StdSender<G> {
    type Flaws = WithdrawnDemand;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        self.sender
            .send(good)
            .map_err(|error| self.recall(Fault::Defect(WithdrawnDemand::default()), error.0))?
    }
}

/// The [`Producer`] of a [`FiniteChannel`] implemented by [`std`].
#[derive(Debug)]
pub struct StdSyncSender<G> {
    /// The name of the sender.
    name: String,
    /// The sender.
    sender: SyncSender<G>,
}

impl<G> Agent for StdSyncSender<G> {
    type Good = G;
}

impl<G> Display for StdSyncSender<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<G> Producer for StdSyncSender<G> {
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

/// An [`InfiniteChannel`] as implemented by a channel from [`mpsc`] with a good of type `G`.
#[derive(Debug)]
pub struct StdInfiniteChannel<G> {
    /// The type of good that is exchanged on the channel.
    good: PhantomData<G>,
}

impl<G> InfiniteChannel<G> for StdInfiniteChannel<G> {
    type Producer = StdSender<G>;
    type Consumer = StdReceiver<G>;

    fn establish<S>(name_str: &S) -> (Self::Producer, Self::Consumer)
    where
        S: AsRef<str> + ?Sized,
    {
        let (sender, receiver) = mpsc::channel();
        let name = String::from(name_str.as_ref());
        (
            StdSender {
                name: name.clone(),
                sender,
            },
            StdReceiver { name, receiver },
        )
    }
}

/// The implementation of [`FiniteChannel`] by [`std`].
#[derive(Debug)]
pub struct StdFiniteChannel<G> {
    /// The type of the good exhanged by the channel.
    good: PhantomData<G>,
}

impl<G> FiniteChannel<G> for StdFiniteChannel<G> {
    type Producer = StdSyncSender<G>;
    type Consumer = StdReceiver<G>;

    fn establish<S>(name_str: &S, size: usize) -> (Self::Producer, Self::Consumer)
    where
        S: AsRef<str> + ?Sized,
    {
        let (sender, receiver) = mpsc::sync_channel(size);
        let name = String::from(name_str.as_ref());
        (
            StdSyncSender {
                name: name.clone(),
                sender,
            },
            StdReceiver { name, receiver },
        )
    }
}
