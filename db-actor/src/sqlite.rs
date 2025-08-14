use actix::{Actor, ActorContext, AsyncContext, Context, Handler, Recipient, WrapFuture};
use actor::{ActorError, ActorResultVoid, ActorServiceMessage};
use sqlx::query::Query;
use sqlx::sqlite::SqliteRow;
use sqlx::{Database, Error, Executor, Sqlite};
use std::ops::Index;
use std::time::Duration;
#[derive(Clone)]
pub struct SqLiteQueryActor<T, Q>
where
    Q: Fn() -> Query<'static, Sqlite, <Sqlite as Database>::Arguments<'static>>,
    T: actix::Message + Send + Clone + Unpin + 'static,
    <T as actix::Message>::Result: Send,
{
    key: String,
    pool: sqlx::Pool<Sqlite>,
    duration: Duration,
    query: Q,
    subscribers: Vec<Recipient<T>>,
    _t: std::marker::PhantomData<T>,
}

impl<Q, T> SqLiteQueryActor<T, Q>
where
    Q: Fn() -> Query<'static, Sqlite, <Sqlite as Database>::Arguments<'static>>,
    T: actix::Message + Send + Clone + Unpin + 'static,
    <T as actix::Message>::Result: Send,
{
    pub fn new(key: String, query: Q, duration: Duration, pool: sqlx::Pool<Sqlite>) -> Self {
        Self {
            key,
            pool,
            duration,
            query,
            _t: std::marker::PhantomData,
            subscribers: vec![],
        }
    }

    pub fn subscribe(&mut self, recipient: Recipient<T>) {
        self.subscribers.push(recipient);
    }
}
impl<Q, T> Handler<ActorServiceMessage> for SqLiteQueryActor<T, Q>
where
    Q: Fn() -> Query<'static, Sqlite, <Sqlite as Database>::Arguments<'static>> + Unpin + 'static,
    T: From<Vec<SqliteRow>> + actix::Message + Send + Clone + Unpin + 'static,
    <T as actix::Message>::Result: Send,
{
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ActorServiceMessage::Start => {
                log::info!("[{}] The actor starts during startup.", self.key);
            }
            ActorServiceMessage::Stop => {
                ctx.stop();
            }
        }

        Ok(())
    }
}

impl<Q, T> Actor for SqLiteQueryActor<T, Q>
where
    Q: Fn() -> Query<'static, Sqlite, <Sqlite as Database>::Arguments<'static>> + Unpin + 'static,
    T: From<Vec<SqliteRow>> + actix::Message + Send + Clone + Unpin + 'static,
    <T as actix::Message>::Result: Send,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(self.duration, |act, ctx| {
            let pool = act.pool.clone();
            let query = (act.query)();
            let subscribers = act.subscribers.clone();
            ctx.spawn(
                async move {
                    match query.fetch_all(&pool).await {
                        Ok(r) => {
                            let message: T = r.into();
                            for sub in &subscribers {
                                sub.do_send(message.clone());
                            }
                        }
                        Err(e) => {
                            log::error!("SqLiteQueryWorker error: {:?}", e);
                        }
                    }
                }
                .into_actor(act),
            );
        });
    }
}
