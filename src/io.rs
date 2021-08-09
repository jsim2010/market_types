//! Implements [`Producer`] and [`Consumer`] for [`Write`] and [`Read`] trait objects.
use {
    alloc::string::String,
    core::{
        cell::RefCell,
        fmt::{self, Display, Formatter},
    },
    fehler::{throw, throws},
    market::{
        Agent, Consumer, ConsumptionFlaws, EmptyStock, Failure, Fault, FullStock, Producer,
        ProductionFlaws, Recall,
    },
    std::io::{Read, Write},
};

/// Characterizes an item that reads to IO without blocking.
pub trait ReadNow: Read {}

/// Characterizes an item that writes to IO without blocking.
pub trait WriteNow: Write {}

/// The defect thrown when a [`Consumer`] fails to read from an I/O.
#[derive(Debug)]
pub struct ReadDefect(std::io::Error);

impl Display for ReadDefect {
    /// Writes "{error}".
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ReadDefect {}

impl From<std::io::Error> for ReadDefect {
    fn from(error: std::io::Error) -> Self {
        Self(error)
    }
}

/// The defect thrown when a [`Producer`] fails to write to a I/O.
#[derive(Debug)]
pub struct WriteDefect(std::io::Error);

impl Display for WriteDefect {
    /// Writes "{error}".
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WriteDefect {}

impl From<std::io::Error> for WriteDefect {
    fn from(error: std::io::Error) -> Self {
        Self(error)
    }
}

/// Implements [`Consumer`] for an [`Read`].
#[derive(Debug)]
pub struct Reader<R> {
    /// The name of the reader.
    name: String,
    /// The reader.
    reader: RefCell<R>,
}

impl<R> Reader<R> {
    /// Creates a new [`Reader`]
    pub const fn new(reader: R, name: String) -> Self {
        Self {
            reader: RefCell::new(reader),
            name,
        }
    }
}

impl<R> Agent for Reader<R> {
    type Good = u8;
}

impl<R: ReadNow> Consumer for Reader<R> {
    type Flaws = ConsumptionFlaws<ReadDefect>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        let mut buffer = [0; 1];
        if self
            .reader
            .borrow_mut()
            .read(&mut buffer)
            .map_err(|error| self.failure(Fault::Defect(error.into())))?
            == 0
        {
            throw!(self.failure(Fault::Insufficiency(EmptyStock::default())));
        } else {
            buffer[0]
        }
    }
}

impl<R> Display for Reader<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Reader of `{}`", self.name)
    }
}

/// A [`Producer`] that implements [`Write`].
#[derive(Debug)]
pub struct Writer<W> {
    /// The name of the [`Writer`].
    name: String,
    /// The writer.
    writer: RefCell<W>,
}

impl<W> Writer<W> {
    /// Creates a new [`Writer`].
    pub const fn new(writer: W, name: String) -> Self {
        Self {
            writer: RefCell::new(writer),
            name,
        }
    }
}

impl<W> Agent for Writer<W> {
    type Good = u8;
}

impl<W> Display for Writer<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Writer of `{}`", self.name)
    }
}

impl<W: WriteNow> Producer for Writer<W> {
    type Flaws = ProductionFlaws<WriteDefect>;

    #[throws(Recall<Self::Flaws, Self::Good>)]
    fn produce(&self, good: Self::Good) {
        let bytes_written = self
            .writer
            .borrow_mut()
            .write(&[good])
            .map_err(|error| self.recall(Fault::Defect(error.into()), good))?;

        if bytes_written == 0 {
            throw!(self.recall(Fault::Insufficiency(FullStock::default()), good));
        }
    }
}
