use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use dotenv::dotenv;
use s3::Client;
use salvo::http::{Method, StatusCode};
use salvo::prelude::*;
use std::env;
use tokio::sync::OnceCell;

static CLIENT: OnceCell<Client> = OnceCell::const_new();

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
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
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

#[handler]
fn propfind_handler(_req: &mut Request, res: &mut Response) {
    // see: https://learn.microsoft.com/en-us/previous-versions/office/developer/exchange-server-2003/aa142960(v=exchg.65)

    res.status_code(StatusCode::OK)
        .render(Text::Plain("propfind_handler"));
}

#[handler]
fn mkcol_handler(_req: &mut Request, res: &mut Response) {
    /*
    MKCOL creates a new collection resource at the location specified by the Request-URI.
    If the Request-URI is already mapped to a resource, then the MKCOL MUST fail.
    During MKCOL processing, a server MUST make the Request-URI an internal member of its parent collection,
    unless the Request-URI is “/”. If no such ancestor exists, the method MUST fail.

    When the MKCOL operation creates a new collection resource, all ancestors MUST already exist,
    or the method MUST fail with a 409 (Conflict) status code.
    (RFC 4918: HTTP Extensions for Web Distributed Authoring and Versioning (WebDAV))
     */

    res.status_code(StatusCode::OK).render(Text::Plain("mkcol"));
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();
    CLIENT.get_or_init(init_client).await;

    // PUT http://localhost:3000/Enpass/vault.enpassdbsync
    // PROPFIND http://localhost:3000/Enpass/vault.enpassdbsync
    // PROPFIND http://localhost:3000/
    // MKCOL http://localhost:3000/Enpass/

    let router = Router::with_path("<bucket>/<**path>")
        .get(get_handler)
        .head(ok_handler)
        .put(ok_handler)
        .delete(ok_handler)
        .push(
            Router::new()
                .filter_fn(|req, _| req.method() == Method::from_bytes(b"PROPFIND").unwrap())
                .goal(propfind_handler),
        )
        .push(
            Router::new()
                .filter_fn(|req, _| req.method() == Method::from_bytes(b"MKCOL").unwrap())
                .goal(mkcol_handler),
        )
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
