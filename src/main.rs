use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use once_cell::sync::Lazy;
use s3::Client;
use salvo::http::{Method, StatusCode};
use salvo::prelude::*;
use std::{env, process};
use tokio::sync::OnceCell;
use tracing::error;

static CLIENT: OnceCell<Client> = OnceCell::const_new();
static BUCKET_NAME: Lazy<String> = Lazy::new(|| {
    let bucket_name = env::var("S3_BUCKET_NAME").unwrap_or("".to_string());

    if bucket_name.is_empty() {
        error!("Env 'S3_BUCKET_NAME' is empty.");
        process::exit(1);
    }

    bucket_name
});

// Initialize S3 slient
async fn init_client() -> Client {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3_endpoint = env::var("S3_ENDPOINT").unwrap_or("".to_string());

    if s3_endpoint.is_empty() {
        return aws_sdk_s3::Client::new(&config);
    }

    let local_config = aws_sdk_s3::config::Builder::from(&config)
        .endpoint_url(s3_endpoint)
        .force_path_style(
            env::var("S3_FORCE_PATH_STYLE")
                .unwrap_or("".to_string())
                .eq("true"),
        )
        .build();

    return aws_sdk_s3::Client::from_conf(local_config);
}

#[handler]
fn ok_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK);
}

#[handler]
async fn get_handler(req: &mut Request, res: &mut Response) {
    let bucket_name = BUCKET_NAME.to_string();
    let my_client = CLIENT.get().unwrap();

    let result_objects = my_client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(req.uri().path().to_string())
        .send()
        .await;

    res.status_code(StatusCode::OK)
        .render(Text::Plain(format!("{:?}", result_objects)));
}

#[handler]
fn copy_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK).render(Text::Plain("COPY"));
}

#[handler]
fn move_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK).render(Text::Plain("MOVE"));
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    CLIENT.get_or_init(init_client).await;

    let router = Router::with_path("<**path>")
        .get(get_handler)
        .head(ok_handler)
        .put(ok_handler)
        .delete(ok_handler)
        .push(
            Router::new()
                .filter_fn(|req, _| req.method() == Method::from_bytes(b"COPY").unwrap())
                .goal(copy_handler),
        )
        .push(
            Router::new()
                .filter_fn(|req, _| req.method() == Method::from_bytes(b"MOVE").unwrap())
                .goal(move_handler),
        );

    let acceptor = TcpListener::new("0.0.0.0:3000").bind().await;
    Server::new(acceptor).serve(router).await;
}
