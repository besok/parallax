// use crate::error::KernelError;
// use crate::{Res, VoidRes};
// use bevy::ecs::system::{FunctionSystem, SystemParam};
// use bevy::prelude::*;
// use bevy::tasks::{ComputeTaskPool, IoTaskPool, Task};
// use std::future::Future;
// use std::marker::PhantomData;
// use std::sync::mpsc;
// use std::sync::mpsc::{SendError, Sender, TrySendError};
//
// pub mod servers;
// pub mod tags;
// pub mod workers;
//
// /// A handle for sending messages TO an actor. This is the "remote control".
// /// It's a lightweight component safe to query from any system.
// #[derive(Component)]
// pub struct ActorHandle<M>(Sender<M>);
//
// impl<M: Send> ActorHandle<M> {
//     /// Sends a message to the actor synchronously.
//     pub fn send(&self, message: M) -> Result<(), SendError<M>> {
//         self.0.send(message)
//     }
// }
// #[derive(Component)]
// pub struct ActorReceiver<M>(pub mpsc::Receiver<M>);
//
// impl<M> ActorReceiver<M> {
//     pub fn receive(&mut self) -> Result<M, std::sync::mpsc::RecvError> {
//         self.0.recv()
//     }
// }
//
// #[derive(Component)]
// struct ActorTask(Task<()>);
//
// pub trait ActorMessage: Send + 'static {
//     fn error(error: KernelError) -> Self;
// }
//
// pub trait Actor<To: ActorMessage, From: ActorMessage = To> {
//     fn id(&self) -> String;
//     fn start(&mut self) -> impl Future<Output = VoidRes> + Send;
//     fn stop(&mut self) -> impl Future<Output = VoidRes> + Send;
//     fn process(
//         &mut self,
//         message: To,
//         sender: Sender<From>,
//     ) -> impl Future<Output = VoidRes> + Send;
// }
//
// pub fn spawn_actor<To, From, A>(mut actor: A, mut commands: Commands) -> Entity
// where
//     A: Actor<To, From> + Send + 'static,
//     To: ActorMessage,
//     From: ActorMessage,
// {
//     let (to_actor_tx, mut to_actor_rx) = mpsc::channel::<To>();
//     let (from_actor_tx, from_actor_rx) = mpsc::channel::<From>();
//
//     let actor_id = actor.id();
//     let task_pool = IoTaskPool::get();
//     let task = task_pool.spawn(async move {
//         if let Err(e) = actor.start().await {
//             let _ = from_actor_tx.send(From::error(e));
//             return;
//         }
//
//         loop {
//             match to_actor_rx.recv() {
//                 Ok(message) => {
//                     if let Err(e) = actor.process(message, from_actor_tx.clone()).await {
//                         let _ = from_actor_tx.send(From::error(e));
//                     }
//                 }
//                 Err(_) => break, // Channel closed
//             }
//         }
//
//         if let Err(e) = actor.stop().await {
//             let _ = from_actor_tx.send(From::error(e));
//         }
//     });
//
//     commands
//         .spawn((
//             ActorHandle(to_actor_tx),
//             ActorReceiver(from_actor_rx),
//             ActorTask(task),
//             Name::new(actor_id.to_string()),
//         ))
//         .id()
// }
