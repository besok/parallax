use crate::actors::servers::ServerError;
use crate::actors::servers::http::{BaseHttpServer, HttpMessage, default_router};
use crate::actors::{Actor, ActorHandle, ActorMessage, spawn_actor};
use crate::error::KernelError;
use crate::{Res, VoidRes, init_logger};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use bevy::ecs::error::info;
use bevy::prelude::{App, Commands, Entity, Startup, TaskPoolPlugin};
use bevy::tasks::{IoTaskPool, Task};
use bevy::{DefaultPlugins, MinimalPlugins};
use log::info;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

struct StateFullHttpServer {
    items: Arc<Mutex<Vec<String>>>,
    inner: BaseHttpServer,
}

enum HttpMessageExtra {
    AddItem(String),
    Base(HttpMessage),
}

impl ActorMessage for HttpMessageExtra {
    fn error(error: KernelError) -> Self {
        HttpMessageExtra::Base(HttpMessage::Error(error))
    }
}

impl StateFullHttpServer {
    fn new(id: String, host: String, port: u16) -> Self {
        type S = Arc<Mutex<Vec<String>>>;
        let items: S = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new()
            .route(
                "/items",
                get(|state: State<S>| async move {
                    let items_guard = state.lock().unwrap();
                    let items_vec: Vec<_> = items_guard.iter().collect();
                    Json(serde_json::json!({ "items": items_vec }))
                }),
            )
            .with_state(items.clone());

        StateFullHttpServer {
            items,
            inner: BaseHttpServer::new(id, host, port, Some(router)),
        }
    }

    fn add_item(&mut self, v: String) -> VoidRes {
        self.items.lock()?.push(v);
        Ok(())
    }
}

impl Actor<HttpMessageExtra> for StateFullHttpServer {
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    async fn start(&mut self) -> VoidRes {
        self.inner.start().await
    }

    async fn stop(&mut self) -> VoidRes {
        self.inner.stop().await
    }

    async fn process(
        &mut self,
        message: HttpMessageExtra,
        _sender: Sender<HttpMessageExtra>,
    ) -> VoidRes {
        match message {
            HttpMessageExtra::AddItem(v) => {
                log::info!(
                    "StateFullHttpServer [id={}] added item: {}",
                    self.inner.id(),
                    v
                );
                self.add_item(v)
            }
            HttpMessageExtra::Base(mes) => {
                log::info!(
                    "StateFullHttpServer [id={}] received a message for delegate",
                    self.inner.id()
                );
                match mes {
                    HttpMessage::Start => self.inner.start().await,
                    HttpMessage::Stop => self.inner.stop().await,
                    HttpMessage::Error(_) => Ok(()),
                }
            }
        }
    }
}

async fn get_items() -> Res<Vec<String>> {
    let client = reqwest::Client::new();

    let response = client
        .get("http://127.0.0.1:8080/items")
        .send()
        .await
        .map_err(|e| ServerError::ClientError(e.to_string()))?;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ServerError::ClientError(e.to_string()))?;

    Ok(body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_string())
        .collect())
}

fn spawn_serv(commands: Commands) {
    let serv = StateFullHttpServer::new("http_serv".to_string(), "127.0.0.1".to_string(), 8080);
    spawn_actor(serv, commands);
}

#[tokio::test]
async fn smoke_statefull_http() -> VoidRes {
    init_logger();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, spawn_serv);

    // Let the app run a few update cycles so the actor can start
    app.update();
    app.update();

    // Give the server time to start up
    sleep(Duration::from_millis(100)).await;

    // Test initial empty state
    assert!(get_items().await?.is_empty());

    // Get the actor handle to send messages
    let mut query = app.world_mut().query::<&ActorHandle<HttpMessageExtra>>();
    let actor_handle = query.single(&app.world())?;

    // Add items to the server
    actor_handle
        .send(HttpMessageExtra::AddItem("A".to_string()))
        .await?;
    actor_handle
        .send(HttpMessageExtra::AddItem("B".to_string()))
        .await?;
    actor_handle
        .send(HttpMessageExtra::AddItem("C".to_string()))
        .await?;

    // Give time for processing
    sleep(Duration::from_millis(100)).await;

    // Verify items were added
    assert_eq!(
        get_items().await?,
        vec!["A".to_string(), "B".to_string(), "C".to_string()]
    );

    // Stop the server
    actor_handle
        .send(HttpMessageExtra::Base(HttpMessage::Stop))
        .await?;

    sleep(Duration::from_millis(100)).await;
    Ok(())
}
