use {
    core::{
        cell::RefCell,
        fmt::{self, Display, Formatter},
        task::Poll,
    },
    fehler::{throw, throws},
    market::{Agent, Blame, Consumer, ConsumptionFlaws, EmptyStock, Failure, Fault},
    markets::compose::{ComposeDefect, Composer, Composite},
    std::collections::VecDeque,
};

#[derive(Debug, PartialEq)]
struct MockMisstep;

#[derive(Clone, Debug, PartialEq)]
struct MockDefect;

#[derive(Clone)]
struct MockConsumer {
    elements: RefCell<VecDeque<Result<u8, Fault<ConsumptionFlaws<MockDefect>>>>>,
}

impl MockConsumer {
    fn new(elements: Vec<Result<u8, Fault<ConsumptionFlaws<MockDefect>>>>) -> Self {
        Self {
            elements: RefCell::new(elements.into()),
        }
    }
}

impl Agent for MockConsumer {
    type Good = u8;
}

impl Consumer for MockConsumer {
    type Flaws = ConsumptionFlaws<MockDefect>;

    #[throws(Failure<Self::Flaws>)]
    fn consume(&self) -> Self::Good {
        if let Some(element) = self.elements.borrow_mut().pop_front() {
            element.map_err(|fault| self.failure(fault))?
        } else {
            throw!(self.failure(Fault::Insufficiency(EmptyStock::default())));
        }
    }
}

impl Display for MockConsumer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "MockConsumer")
    }
}

#[derive(Debug, PartialEq)]
struct MockComposite;

impl Composite<u8> for MockComposite {
    type Misstep = MockMisstep;

    #[throws(Self::Misstep)]
    fn compose(elements: &mut Vec<u8>) -> Poll<Self>
    where
        Self: Sized,
    {
        match elements.get(0) {
            Some(0) => match elements.get(1) {
                Some(1) => match elements.get(2) {
                    Some(2) => {
                        elements.drain(0..3);
                        Poll::Ready(MockComposite)
                    }
                    Some(_) => {
                        elements.drain(0..3);
                        throw!(MockMisstep)
                    }
                    None => Poll::Pending,
                },
                Some(_) => {
                    elements.drain(0..2);
                    throw!(MockMisstep);
                }
                None => Poll::Pending,
            },
            Some(_) => {
                elements.drain(0..1);
                throw!(MockMisstep);
            }
            None => Poll::Pending,
        }
    }
}

#[test]
fn compose_success() {
    let consumer = MockConsumer::new(vec![Ok(0), Ok(1), Ok(2)]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_consume_insufficient_stock() {
    let consumer = MockConsumer::new(vec![
        Err(Fault::Insufficiency(EmptyStock::default())),
        Ok(0),
        Ok(1),
        Ok(2),
    ]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_partial() {
    let consumer = MockConsumer::new(vec![
        Ok(0),
        Ok(1),
        Err(Fault::Insufficiency(EmptyStock::default())),
        Ok(2),
    ]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_consume_fault() {
    let consumer = MockConsumer::new(vec![Err(Fault::Defect(MockDefect)), Ok(0), Ok(1), Ok(2)]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(
        composer.consume(),
        Err(consumer.failure(Fault::Defect(MockDefect)).blame())
    );
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_consume_fault_partial() {
    let consumer = MockConsumer::new(vec![Ok(0), Ok(1), Err(Fault::Defect(MockDefect)), Ok(2)]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(
        composer.consume(),
        Err(consumer.failure(Fault::Defect(MockDefect)).blame())
    );
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_build_error() {
    let consumer = MockConsumer::new(vec![Ok(9), Ok(0), Ok(1), Ok(2)]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(
        composer.consume(),
        Err(composer.failure(Fault::Defect(ComposeDefect::Compose(MockMisstep))))
    );
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_build_error_partial() {
    let consumer = MockConsumer::new(vec![Ok(0), Ok(1), Ok(9), Ok(0), Ok(1), Ok(2)]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(
        composer.consume(),
        Err(composer.failure(Fault::Defect(ComposeDefect::Compose(MockMisstep))))
    );
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}

#[test]
fn compose_multiple() {
    let consumer = MockConsumer::new(vec![Ok(0), Ok(1), Ok(2), Ok(0), Ok(1), Ok(2)]);
    let composer = Composer::new(consumer.clone());

    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(composer.consume(), Ok(MockComposite));
    assert_eq!(
        composer.consume(),
        Err(consumer
            .failure(Fault::Insufficiency(EmptyStock::default()))
            .blame())
    );
}
