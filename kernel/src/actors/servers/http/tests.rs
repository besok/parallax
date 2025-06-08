use crate::actors::servers::ServerError;
use crate::actors::servers::http::{BaseHttpServer, HttpMessage, default_router};
use crate::actors::{Actor, spawn_actor};
use crate::{Res, VoidRes, init_logger};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

struct StateFullHttpServer {
    items: Arc<Mutex<Vec<String>>>,
    inner: BaseHttpServer,
}

enum HttpMessageExtra {
    AddItem(String),
    Base(HttpMessage),
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
        self.items.lock().unwrap().push(v);
        Ok(())
    }
}

impl Actor<HttpMessageExtra> for StateFullHttpServer {
    async fn start(&mut self) -> VoidRes {
        self.inner.start().await
    }

    async fn stop(&mut self) -> VoidRes {
        self.inner.stop().await
    }

    async fn process(&mut self, message: HttpMessageExtra) -> VoidRes {
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
                self.inner.process(mes).await
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

#[tokio::test]
async fn smoke_statefull_http() -> VoidRes {
    init_logger();
    let serv =
        StateFullHttpServer::new("statefull_http".to_string(), "127.0.0.1".to_string(), 8080);

    let serv_handle = spawn_actor(serv, None).await?;

    assert!(get_items().await?.is_empty());

    serv_handle
        .send(HttpMessageExtra::AddItem("A".to_string()))
        .await?;

    serv_handle
        .send(HttpMessageExtra::AddItem("B".to_string()))
        .await?;

    serv_handle
        .send(HttpMessageExtra::AddItem("C".to_string()))
        .await?;

    assert_eq!(
        get_items().await?,
        vec!["A".to_string(), "B".to_string(), "C".to_string()]
    );

    serv_handle
        .send(HttpMessageExtra::Base(HttpMessage::Stop))
        .await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}
