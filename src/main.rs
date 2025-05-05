use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{Arc, LazyLock, atomic::AtomicUsize},
    usize,
};

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::header,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
    serve,
};
use serde::Deserialize;
use tokio::{net::TcpListener, sync::Mutex};

static ID_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

#[derive(Debug, Template)]
#[template(path = "index.html", block = "index")]
pub struct Index<'a> {
    pub todos: Todos<'a>,
}

#[derive(Debug, Template, Clone)]
#[template(path = "index.html", block = "todos")]
pub struct Todos<'a> {
    pub todos: Vec<&'a Todo>,
}

#[derive(Debug, Template, Clone)]
#[template(path = "index.html", block = "todo")]
pub struct Todo {
    pub id: usize,
    pub title: String,
    pub done: bool,
}

#[derive(Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

async fn root(State(todos): State<Arc<Mutex<HashMap<usize, Todo>>>>) -> impl IntoResponse {
    let hash_map = todos.lock().await;
    let todos: Vec<&Todo> = hash_map.values().collect();
    let person = Index {
        todos: Todos { todos },
    };
    let res = person.render().unwrap();
    Html(res)
}
async fn create_todo(
    State(todos): State<Arc<Mutex<HashMap<usize, Todo>>>>,
    frm: Form<CreateTodo>,
) -> impl IntoResponse {
    let mut state = todos.lock().await;
    let id = ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let todo = Todo {
        id,
        title: frm.title.clone(),
        done: false,
    };
    let _ = state.insert(id, todo.clone());
    Response::builder()
        .status(201)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(todo.render().unwrap())
        .expect("FATAL: unreachable")
}

async fn done(
    State(todos): State<Arc<Mutex<HashMap<usize, Todo>>>>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    let mut state = todos.lock().await;
    let todo = state.get_mut(&id);
    if let Some(todo) = todo {
        todo.done = !todo.done;
        Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(todo.render().unwrap())
            .expect("FATAL: unreachable")
    } else {
        Response::builder()
            .status(404)
            .body("todo not found".into())
            .expect("FATAL: unreachable")
    }
}

async fn delete_todo(
    State(todos): State<Arc<Mutex<HashMap<usize, Todo>>>>,
    Path(id): Path<usize>,
) -> impl IntoResponse {
    let mut state = todos.lock().await;
    let todo = state.remove(&id);
    if todo.is_some() {
        Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body("".to_string())
            .expect("FATAL: unreachable")
    } else {
        Response::builder()
            .status(404)
            .body("todo not found".to_string())
            .expect("FATAL: unreachable")
    }
}

#[tokio::main]
async fn main() {
    let db: Arc<Mutex<HashMap<usize, Todo>>> = Arc::new(Mutex::new(HashMap::new()));
    let router = Router::new()
        .route("/", get(root))
        .route("/", post(create_todo))
        .route("/{id}", put(done))
        .route("/{id}", delete(delete_todo))
        .with_state(db);

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 42069))
        .await
        .unwrap();
    serve(listener, router.into_make_service()).await.unwrap();
}
