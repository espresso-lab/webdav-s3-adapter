mod utils;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use dotenv::dotenv;
use salvo::http::{Method, StatusCode};
use salvo::prelude::*;
use tokio::sync::OnceCell;
use tracing::warn;
use utils::s3::{
    fetch_file_from_s3, init_client, is_folder, list_objects_in_s3, upload_file_to_s3,
};

use crate::utils::s3::get_object;
use crate::utils::webdav::{generate_webdav_propfind_response, S3ObjectOutput};

static CLIENT: OnceCell<Client> = OnceCell::const_new();

#[handler]
fn ok_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK).render(Text::Plain("OK"));
}

#[handler]
async fn get_handler(req: &mut Request, res: &mut Response) {
    warn!("get_handler");
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();
    match fetch_file_from_s3(CLIENT.get().unwrap(), &bucket_name, &key).await {
        Ok((file_contents, content_type)) => {
            res.headers_mut()
                .insert("Content-Type", content_type.parse().unwrap());
            res.headers_mut().insert(
                "Content-Length",
                file_contents.len().to_string().parse().unwrap(),
            );
            let _ = res.write_body(file_contents);
        }
        Err(_) => {
            res.status_code(StatusCode::NOT_FOUND);
            let _ = res.write_body("File not found");
        }
    }
}

#[handler]
async fn put_handler(req: &mut Request, res: &mut Response) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();
    let payload = req.payload().await.unwrap().clone();

    match upload_file_to_s3(
        CLIENT.get().unwrap(),
        &bucket_name,
        &key,
        ByteStream::new(payload.into()),
    )
    .await
    {
        Ok(upload_result) => {
            res.status_code(StatusCode::CREATED);
            res.headers_mut().insert(
                "ETag",
                upload_result.e_tag.unwrap_or_default().parse().unwrap(),
            );
            res.headers_mut().insert(
                "Location",
                format!("/{}{}", bucket_name, key).parse().unwrap(),
            );
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

#[handler]
fn copy_handler(_req: &mut Request, res: &mut Response) {
    warn!("copy_handler");
    res.status_code(StatusCode::OK).render(Text::Plain("COPY"));
}

#[handler]
fn move_handler(_req: &mut Request, res: &mut Response) {
    warn!("move_handler");
    res.status_code(StatusCode::OK).render(Text::Plain("MOVE"));
}

#[handler]
async fn propfind_handler(req: &mut Request, res: &mut Response) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();

    warn!("propfind_handler | {}", key);

    if is_folder(CLIENT.get().unwrap(), &bucket_name, &key)
        .await
        .unwrap_or_default()
    {
        match list_objects_in_s3(CLIENT.get().unwrap(), &bucket_name, &key, Some("/")).await {
            Ok(obj) => {
                let response = generate_webdav_propfind_response(
                    &bucket_name,
                    &key,
                    S3ObjectOutput::ListObjects(obj),
                );
                res.status_code(StatusCode::MULTI_STATUS)
                    .render(Text::Xml(response));
            }
            Err(_) => {
                res.status_code(StatusCode::NOT_FOUND);
            }
        }
    } else {
        match get_object(CLIENT.get().unwrap(), &bucket_name, &key).await {
            Ok(obj) => {
                let response = generate_webdav_propfind_response(
                    &bucket_name,
                    &key,
                    S3ObjectOutput::GetObject(obj),
                );
                res.status_code(StatusCode::MULTI_STATUS)
                    .render(Text::Xml(response));
            }
            Err(_) => {
                res.status_code(StatusCode::NOT_FOUND);
            }
        }
    }
}

#[handler]
async fn mkcol_handler(req: &mut Request, res: &mut Response) {
    warn!("mkcol handler");
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let path = req.params().get("**path").cloned().unwrap_or_default();

    let result_objects = CLIENT
        .get()
        .unwrap()
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(path)
        .send()
        .await;

    if result_objects.unwrap().key_count().unwrap_or(0) > 0 {
        res.status_code(StatusCode::CONFLICT);
    }

    res.status_code(StatusCode::NO_CONTENT);
}

struct Validator;
impl BasicAuthValidator for Validator {
    async fn validate(&self, username: &str, password: &str, depot: &mut Depot) -> bool {
        depot.insert("auth_user", username.to_owned());
        depot.insert("auth_pass", password.to_owned());

        !username.is_empty() && !password.is_empty()
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();
    CLIENT.get_or_init(init_client).await;

    let router = Router::new()
        .push(Router::with_path("/status").get(ok_handler))
        .push(
            Router::with_path("<bucket>/<**path>")
                .hoop(BasicAuth::new(Validator))
                .get(get_handler)
                .head(ok_handler)
                .put(put_handler)
                .delete(ok_handler)
                .push(
                    Router::new()
                        .filter_fn(|req, _| {
                            req.method() == Method::from_bytes(b"PROPFIND").unwrap()
                        })
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
                ),
        );

    let acceptor = TcpListener::new("0.0.0.0:3000").bind().await;
    Server::new(acceptor).serve(router).await;
}
