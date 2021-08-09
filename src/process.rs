//! Implements [`Producer`] and [`Consumer`] for a process.
use {
    crate::{
        compose::{ComposeDefect, Composer, Composite},
        io::{ReadDefect, ReadNow, Reader, WriteNow, Writer},
    },
    alloc::{borrow::ToOwned, format, string::String},
    core::{
        cell::RefCell,
        convert::TryFrom,
        fmt::{self, Debug, Display, Formatter},
        marker::PhantomData,
    },
    fehler::{throw, throws},
    market::{Agent, Blame, Consumer, ConsumptionFlaws, EmptyStock, Failure, Fault, Flaws},
    std::{
        io::{Read, Write},
        process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    },
};

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

/// An implementation of [`ChildStdin`] that does not block.
#[derive(Debug)]
pub struct NoWaitChildStdin {
    /// The [`ChildStdin`].
    inner: ChildStdin,
}

#[cfg(windows)]
impl TryFrom<ChildStdin> for NoWaitChildStdin {
    type Error = std::io::Error;

    #[throws(Self::Error)]
    fn try_from(inner: ChildStdin) -> Self {
        #[allow(unsafe_code, clippy::as_conversions)] // Required to make ChildStdin non-blocking.
        if unsafe {
            winapi::um::namedpipeapi::SetNamedPipeHandleState(
                inner.as_raw_handle().cast::<winapi::ctypes::c_void>(),
                &mut 0x1,
                &mut 0,
                &mut 0,
            )
        } == 0_i32
        {
            throw!(std::io::Error::last_os_error());
        }

        Self { inner }
    }
}

impl Write for NoWaitChildStdin {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(windows)]
impl WriteNow for NoWaitChildStdin {}

/// An implementation of [`ChildStdout`] that does not block.
#[derive(Debug)]
struct NoWaitChildStdout {
    /// The [`ChildStdout`].
    inner: ChildStdout,
}

#[cfg(windows)]
impl TryFrom<ChildStdout> for NoWaitChildStdout {
    type Error = std::io::Error;

    #[throws(Self::Error)]
    fn try_from(inner: ChildStdout) -> Self {
        #[allow(unsafe_code, clippy::as_conversions)] // Required to make ChildStdout non-blocking.
        if unsafe {
            winapi::um::namedpipeapi::SetNamedPipeHandleState(
                inner.as_raw_handle().cast::<winapi::ctypes::c_void>(),
                &mut 0x1,
                &mut 0,
                &mut 0,
            )
        } == 0_i32
        {
            throw!(std::io::Error::last_os_error());
        }

        Self { inner }
    }
}

impl Read for NoWaitChildStdout {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(windows)]
impl ReadNow for NoWaitChildStdout {}

/// An implementation of [`ChildStderr`] that does not block.
#[derive(Debug)]
struct NoWaitChildStderr {
    /// The [`ChildStderr`].
    inner: ChildStderr,
}

#[cfg(windows)]
impl TryFrom<ChildStderr> for NoWaitChildStderr {
    type Error = std::io::Error;

    #[throws(Self::Error)]
    fn try_from(inner: ChildStderr) -> Self {
        #[allow(unsafe_code, clippy::as_conversions)] // Required to make ChildStderr non-blocking.
        if unsafe {
            winapi::um::namedpipeapi::SetNamedPipeHandleState(
                inner.as_raw_handle().cast::<winapi::ctypes::c_void>(),
                &mut 0x1,
                &mut 0,
                &mut 0,
            )
        } == 0_i32
        {
            throw!(std::io::Error::last_os_error());
        }

        Self { inner }
    }
}

impl Read for NoWaitChildStderr {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(windows)]
impl ReadNow for NoWaitChildStderr {}

/// An output from a process.
#[derive(Debug)]
#[non_exhaustive]
pub enum Product<O, E> {
    /// The process generated normal output.
    Output(O),
    /// The process generated error output.
    Error(E),
    /// The process exited.
    Exit(ExitStatus),
}

impl<O, E> From<ExitStatus> for Product<O, E> {
    fn from(exit_status: ExitStatus) -> Self {
        Self::Exit(exit_status)
    }
}

/// A defect thrown when attempting to consume the exit status of a process.
#[derive(Debug)]
pub struct WaitDefect(std::io::Error);

impl Display for WaitDefect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A [`Consumer`] of the exit status of a process.
#[derive(Debug)]
pub struct Exiter {
    /// The name of the process.
    name: String,
    /// The [`Child`] process.
    child: RefCell<Child>,
}

impl Agent for Exiter {
    type Good = ExitStatus;
}

impl Consumer for Exiter {
    type Flaws = ConsumptionFlaws<WaitDefect>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        self.child
            .borrow_mut()
            .try_wait()
            .map_err(|error| self.failure(Fault::Defect(WaitDefect(error))))?
            .ok_or_else(|| self.failure(Fault::Insufficiency(EmptyStock::default())))?
    }
}

impl Display for Exiter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Exiter for process `{}`", self.name)
    }
}

/// A defect thrown by the consumer of a process.
#[derive(Debug)]
#[non_exhaustive]
pub enum ProcessDefect<O: Composite<u8>, E: Composite<u8>> {
    /// Failed to compose the bytes read on the stdout of the process into an `O`.
    Output(ComposeDefect<ReadDefect, O::Misstep>),
    /// Failed to compose the bytes read on the stderr of the process into an `E`.
    Error(ComposeDefect<ReadDefect, E::Misstep>),
    /// Failed while checking if the process has exited.
    Exit(WaitDefect),
}

#[allow(clippy::type_repetition_in_bounds)] // False positive for <E as Composite<u8>>::Misstep repeating.
impl<O, E> Display for ProcessDefect<O, E>
where
    O: Composite<u8> + Display,
    <O as Composite<u8>>::Misstep: Display,
    E: Composite<u8> + Display,
    <E as Composite<u8>>::Misstep: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Output(ref compose_defect) => write!(f, "Output - {}", compose_defect),
            Self::Error(ref compose_defect) => write!(f, "Error - {}", compose_defect),
            Self::Exit(ref wait_defect) => write!(f, "Exit - {}", wait_defect),
        }
    }
}

impl<O: Composite<u8>, E: Composite<u8>> From<WaitDefect> for ProcessDefect<O, E> {
    fn from(defect: WaitDefect) -> Self {
        Self::Exit(defect)
    }
}

/// The [`Flaws`] of a [`ProductConsumer`].
#[derive(Debug)]
pub struct ProcessFlaws<O, E> {
    /// The type of the process output.
    output_type: PhantomData<O>,
    /// The type of the process error.
    error_type: PhantomData<E>,
}

impl<O, E> Flaws for ProcessFlaws<O, E>
where
    O: Composite<u8>,
    E: Composite<u8>,
{
    type Insufficiency = EmptyStock;
    type Defect = ProcessDefect<O, E>;
}

/// The [`Consumer`] of a process, consuming [`Product`].
#[derive(Debug)]
pub struct ProductConsumer<O, E> {
    /// Consumes the `O`s of the process.
    output_composer: Composer<u8, O, Reader<NoWaitChildStdout>>,
    /// Consumes the `E`s of the process.
    error_composer: Composer<u8, E, Reader<NoWaitChildStderr>>,
    /// Consumes the [`ExitStatus`] of the process.
    exiter: Exiter,
    /// The name of the process.
    name: String,
}

impl<O, E> Agent for ProductConsumer<O, E> {
    type Good = Product<O, E>;
}

impl<O, E> Consumer for ProductConsumer<O, E>
where
    O: Composite<u8>,
    E: Composite<u8>,
{
    type Flaws = ProcessFlaws<O, E>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        match self.output_composer.consume() {
            Ok(output) => Product::Output(output),
            Err(failure) => {
                if failure.is_defect() {
                    throw!(failure.map_defect(ProcessDefect::Output));
                }

                match self.error_composer.consume() {
                    Ok(error) => Product::Error(error),
                    Err(failure) => {
                        if failure.is_defect() {
                            throw!(failure.map_defect(ProcessDefect::Error));
                        }

                        match self.exiter.consume() {
                            Ok(exit_status) => exit_status.into(),
                            Err(failure) => throw!(failure.blame()),
                        }
                    }
                }
            }
        }
    }
}

impl<O, E> Display for ProductConsumer<O, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ProductConsumer of `{}`", self.name)
    }
}

/// Spawns a process running `command`.
///
/// # Errors
///
/// Throws I/O error if spawn or conversion of I/Os fail.
#[throws(std::io::Error)]
pub fn spawn<O, E, S>(
    mut command: Command,
    name_str: &S,
) -> (Writer<NoWaitChildStdin>, ProductConsumer<O, E>)
where
    O: Composite<u8> + 'static,
    E: Composite<u8> + 'static,
    S: AsRef<str> + ?Sized,
{
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let name = name_str.as_ref();

    #[allow(clippy::panic)] // Panics should not occur.
    (
        Writer::new(
            NoWaitChildStdin::try_from(
                child
                    .stdin
                    .take()
                    .unwrap_or_else(|| panic!("retrieving stdin of process `{}`", name)),
            )?,
            format!("stdin writer of process: {}", name),
        ),
        ProductConsumer {
            output_composer: Composer::new(Reader::new(
                NoWaitChildStdout::try_from(
                    child
                        .stdout
                        .take()
                        .unwrap_or_else(|| panic!("retrieving stdout of process `{}`", name)),
                )?,
                format!("stdout reader of process `{}`", name),
            )),
            error_composer: Composer::new(Reader::new(
                NoWaitChildStderr::try_from(
                    child
                        .stderr
                        .take()
                        .unwrap_or_else(|| panic!("retrieving stderr of process `{}`", name)),
                )?,
                format!("stderr reader of process `{}`", name),
            )),
            exiter: Exiter {
                name: String::from(name),
                child: RefCell::new(child),
            },
            name: name.to_owned(),
        },
    )
}
