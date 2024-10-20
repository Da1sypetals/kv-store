use axum::{
    routing::{delete, get, post},
    Router,
};
use kv_interface::interface::config::start_dir_store;
use std::sync::Arc;

mod service;

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "trace");
    pretty_env_logger::init();

    // build our application with a route
    let ds = Arc::new(start_dir_store("config.toml"));

    let app = Router::new()
        .route(
            "/kv/*path",
            get(service::crud::get)
                .delete(service::crud::delete)
                .post(service::crud::put),
        )
        .route(
            "/batched/kv/:batchname/*path",
            delete(service::batched::delete).post(service::batched::put),
        )
        .route("/batched/new/:batchname", post(service::batched::new))
        .route("/batched/commit/:batchname", post(service::batched::commit))
        .route("/ls", get(service::crud::list_root))
        .route("/ls/*dir", get(service::crud::list))
        .route("/exec", post(service::advanced::exec))
        .route("/merge", post(service::advanced::merge))
        .with_state(ds);

    // let app = Router::new()
    //     .route("/kv/*dir", get().delete().post())
    //     .route("/batched/kv/:batchname/*dir", delete().post())
    //     .route("/batched/new/:batchname", post())
    //     .route("/ls/*dir", get())
    //     .route("/exec", post())
    //     .route("/merge", post());

    // run our app with hyper, listening globally on port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
