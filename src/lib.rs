mod eventual;
pub use eventual::{Eventual, EventualReader, EventualWriter};
pub mod error;
pub use error::Closed;
mod combinators;
pub use combinators::*;

// This is a convenience trait to make it easy to pass either an Eventual or an
// EventualReader into functions.
// TODO: Implement
pub trait IntoReader {
    type Output;
    fn into_reader(self) -> EventualReader<Self::Output>;
}

pub trait Value: 'static + Send + Clone + Eq {}
impl<T> Value for T where T: 'static + Send + Clone + Eq {}

// This is the goal:
/*
fn log_errors(logger: Logger, source: Eventual<Result<T, Err>>) -> Eventual<T> {
    let out = Eventual::new();
    tokio::spawn(async move {
        loop {
            match source.next().await {
                Ok(Ok(v)) => out.write(v),
                Ok(Err(e)) => error!(logger, e),
                Err(_) => break;
            }
        }
    });
    out
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::{join, test};

    // TODO: Much more sophisticated tests are needed.
    #[test]
    async fn it_works() {
        let (mut writer, eventual) = Eventual::<u32>::new();
        let mut read_0 = eventual.subscribe();
        let read_x = eventual.subscribe();
        let mut read_1 = eventual.subscribe();
        let mut read_2 = eventual.subscribe();

        writer.write(5);
        assert_eq!(read_0.next().await.unwrap(), 5);
        drop(read_x);

        let r0 =
            tokio::spawn(
                async move { read_0.next().await.unwrap() + read_0.next().await.unwrap() },
            );

        let r1 = tokio::spawn(async move {
            writer.write(10);
            tokio::time::sleep(Duration::from_millis(10)).await;
            let next = read_1.next();
            writer.write(8);
            tokio::time::sleep(Duration::from_millis(10)).await;
            next.await.unwrap()
        });

        let (r0, r1) = join!(r0, r1);
        assert_eq!(r0.unwrap(), 18);
        assert_eq!(r1.unwrap(), 8);
        assert_eq!(read_2.next().await, Err(Closed));
    }

    #[test]
    async fn basic_map() {
        let (mut writer, eventual) = Eventual::<u32>::new();
        writer.write(5);

        // Format the value and save it in an Arc<String> for
        let format_value = |v| async move { Arc::new(format!("{}", v)) };
        let mut mapped = map(&eventual, format_value).subscribe();

        assert_eq!(&mapped.next().await.ok().unwrap().as_str(), &"5");

        writer.write(10);
        assert_eq!(&mapped.next().await.ok().unwrap().as_str(), &"10");

        drop(writer);
        assert_eq!(mapped.next().await, Err(Closed))
    }
}
