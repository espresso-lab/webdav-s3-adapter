mod utils;

use aws_sdk_s3::primitives::ByteStream;
use dotenv::dotenv;
use salvo::http::{Method, StatusCode};
use salvo::prelude::*;
use tracing::debug;
use utils::s3::{
    delete_file_from_s3, fetch_file_from_s3, init_client_for_auth, list_objects_in_s3,
    upload_file_to_s3,
};

use crate::utils::webdav::generate_webdav_propfind_response;

#[handler]
fn ok_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK).render(Text::Plain("OK"));
}

#[handler]
async fn get_handler(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();

    let aws_client = init_client_for_auth(
        depot.get::<String>("auth_user").unwrap().to_string(),
        depot.get::<String>("auth_pass").unwrap().to_string(),
    )
    .await;

    match fetch_file_from_s3(&aws_client, &bucket_name, &key).await {
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
            res.status_code(StatusCode::NOT_FOUND)
                .render(Text::Plain("File not found"));
        }
    }
}

#[handler]
async fn put_handler(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();

    let byte_stream = match req.payload_with_max_size(10 * 1024 * 1024 * 1024).await {
        Ok(bytes) => ByteStream::from(bytes.clone()),
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            return;
        }
    };

    let aws_client = init_client_for_auth(
        depot.get::<String>("auth_user").unwrap().to_string(),
        depot.get::<String>("auth_pass").unwrap().to_string(),
    )
    .await;

    match upload_file_to_s3(&aws_client, &bucket_name, &key, byte_stream).await {
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
async fn delete_handler(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();

    let aws_client = init_client_for_auth(
        depot.get::<String>("auth_user").unwrap().to_string(),
        depot.get::<String>("auth_pass").unwrap().to_string(),
    )
    .await;

    match delete_file_from_s3(&aws_client, &bucket_name, &key).await {
        Ok(_) => {
            res.status_code(StatusCode::NO_CONTENT);
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

#[handler]
fn copy_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::NOT_IMPLEMENTED);
}

#[handler]
fn move_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::NOT_IMPLEMENTED);
}

#[handler]
fn options_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::FOUND);
}

#[handler]
async fn propfind_handler(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let key = req.params().get("**path").cloned().unwrap_or_default();

    let aws_client = init_client_for_auth(
        depot.get::<String>("auth_user").unwrap().to_string(),
        depot.get::<String>("auth_pass").unwrap().to_string(),
    )
    .await;

    debug!(
        "user: {}, pw: {}",
        depot.get::<String>("auth_user").unwrap().to_string(),
        depot.get::<String>("auth_pass").unwrap().to_string()
    );

    match list_objects_in_s3(&aws_client, &bucket_name, &key, Some("/")).await {
        Ok(obj) => {
            let response = generate_webdav_propfind_response(&bucket_name, obj);
            res.status_code(StatusCode::MULTI_STATUS)
                .render(Text::Xml(response));
        }
        Err(_) => {
            res.status_code(StatusCode::NOT_FOUND);
        }
    }
}

#[handler]
async fn mkcol_handler(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let path = req.params().get("**path").cloned().unwrap_or_default();

    let aws_client = init_client_for_auth(
        depot.get::<String>("auth_user").unwrap().to_string(),
        depot.get::<String>("auth_pass").unwrap().to_string(),
    )
    .await;

    let result_objects = aws_client
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

trait WebDavRouter {
    fn webdav_propfind<H: Handler>(self, goal: H) -> Self;
    fn webdav_copy<H: Handler>(self, goal: H) -> Self;
    fn webdav_move<H: Handler>(self, goal: H) -> Self;
    fn webdav_mkcol<H: Handler>(self, goal: H) -> Self;
}

impl WebDavRouter for Router {
    #[inline]
    fn webdav_propfind<H: Handler>(self, goal: H) -> Self {
        self.push(
            Router::with_filter_fn(|req, _| {
                req.method() == Method::from_bytes(b"PROPFIND").unwrap()
            })
            .goal(goal),
        )
    }

    #[inline]
    fn webdav_copy<H: Handler>(self, goal: H) -> Self {
        self.push(
            Router::with_filter_fn(|req, _| req.method() == Method::from_bytes(b"COPY").unwrap())
                .goal(goal),
        )
    }

    #[inline]
    fn webdav_move<H: Handler>(self, goal: H) -> Self {
        self.push(
            Router::with_filter_fn(|req, _| req.method() == Method::from_bytes(b"MOVE").unwrap())
                .goal(goal),
        )
    }

    #[inline]
    fn webdav_mkcol<H: Handler>(self, goal: H) -> Self {
        self.push(
            Router::with_filter_fn(|req, _| req.method() == Method::from_bytes(b"MKCOL").unwrap())
                .goal(goal),
        )
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();

    let router = Router::new()
        .push(Router::with_path("/status").get(ok_handler))
        .push(
            Router::with_path("/<bucket>/<**path>")
                .hoop(BasicAuth::new(Validator))
                .head(ok_handler)
                .options(options_handler)
                .get(get_handler)
                .put(put_handler)
                .delete(delete_handler)
                .webdav_propfind(propfind_handler)
                .webdav_mkcol(mkcol_handler)
                .webdav_copy(copy_handler)
                .webdav_move(move_handler), // TODO: Maybe before auth
        );

    let service = Service::new(router).hoop(Logger::new());
    let acceptor = TcpListener::new("0.0.0.0:3000").bind().await;

    Server::new(acceptor).serve(service).await;
}
