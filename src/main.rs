use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use dotenv::dotenv;
use s3::primitives::ByteStream;
use s3::Client;
use salvo::http::{Method, StatusCode};
use salvo::prelude::*;
use std::borrow::Borrow;
use std::env;
use tokio::io::BufReader;
use tokio::sync::OnceCell;
use tracing::info;

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
    warn!("OK");
    res.status_code(StatusCode::OK);
}

#[handler]
async fn get_handler(req: &mut Request, res: &mut Response) {
    warn!("get_handler");

    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let path = req.params().get("**path").cloned().unwrap_or_default();
    let client = CLIENT.get().unwrap();

    let result = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(path)
        .send()
        .await
        .unwrap();

    for object in result.contents() {
        info!(" - {}", object.key().unwrap_or("Unknown"));
    }

    res.status_code(StatusCode::OK);
}

// TODO:
// let cache_control = req.headers().get(header::CACHE_CONTROL).and_then(header_string);
//     let content_disposition = req.headers().get(header::CONTENT_DISPOSITION).and_then(header_string);
//     let content_encoding = req.headers().get(header::CONTENT_ENCODING).and_then(header_string);
//     let content_language = req.headers().get(header::CONTENT_LANGUAGE).and_then(header_string);
//     let content_type = req.headers().get(header::CONTENT_TYPE).and_then(header_string);
//     let expires = req.headers().get(header::EXPIRES).and_then(header_string);
#[handler]
async fn put_handler(req: &mut Request, res: &mut Response) {
    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let path = req.params().get("**path").cloned().unwrap_or_default();
    let payload = req.payload().await.unwrap().clone();

    //     let body_stream: Box<Stream<Item=Bytes, Error=Error>> = Box::new(
    //     req.payload()
    //         .map_err(|_e| ErrorInternalServerError("Something went wrong while reading request stream"))
    // );

    // req.payload();

    // file.clone().path().file_name().unwrap().to_str();
    // warn!(
    //     "file path {}",
    //     file.clone().path().file_name().unwrap().to_str().unwrap()
    // );

    // req.body()
    // let reader = BufReader::new(req.body());
    // stream::iter(reader.bytes());
    // ByteStream::from_static(req.body());

    let upload_result = CLIENT
        .get()
        .unwrap()
        .put_object()
        .bucket(&bucket_name)
        .key(path)
        .body(ByteStream::new(payload.into()))
        .send()
        .await;

    if !upload_result.unwrap().checksum_sha256.unwrap().is_empty() {
        res.status_code(StatusCode::NO_CONTENT);
    }

    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
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
    // see: https://learn.microsoft.com/en-us/previous-versions/office/developer/exchange-server-2003/aa142960(v=exchg.65)

    let bucket_name = req.params().get("bucket").cloned().unwrap_or_default();
    let path = req.params().get("**path").cloned().unwrap_or_default();

    warn!("propfind_handler | {}", path);

    let get_result = CLIENT
        .get()
        .unwrap()
        .get_object()
        .bucket(&bucket_name)
        .key(path.clone())
        .send()
        .await;

    match get_result {
        Ok(obj) => {
            let xml = r#"
            <?xml version="1.0"?>
            <a:multistatus
            xmlns:b="urn:uuid:c2f41010-65b3-11d1-a29f-00aa00c14882/"
            xmlns:a="DAV:">
            <a:response>
            <a:href>https://server/public/test2/item1.txt</a:href>
            <a:propstat>
                <a:status>HTTP/1.1 200 OK</a:status>
                <a:prop>
                    <a:getcontenttype>text/plain</a:getcontenttype>
                    <a:getcontentlength b:dt="int">33</a:getcontentlength>
                </a:prop>
            </a:propstat>
            </a:response>
            </a:multistatus>
            "#;

            return res.status_code(StatusCode::OK).render(Text::Xml(xml));
        }
        Err(err) => {}
    }

    // If it's not a file, then check if it's a folder

    let list_result = CLIENT
        .get()
        .unwrap()
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(path)
        .send()
        .await;

    match list_result {
        Ok(obj) => {
            let xml = r#"
            <?xml version="1.0" ?>
            <D:multistatus xmlns:D="DAV:">
            <D:response>
                <D:href>https://www.contoso.com/public/container/</D:href>
                <D:propstat>
                        <D:prop xmlns:R="https://www.contoso.com/schema/">
                            <R:author>Rob Caron</R:author>
                            <R:editor>Jessup Meng</R:editor>
                            <D:creationdate>
                                1999-11-01T17:42:21-06:30
                            </D:creationdate>
                            <D:displayname>
                                Example Collection
                            </D:displayname>
                            <D:resourcetype><D:collection></D:resourcetype>
                            <D:supportedlock>
                                <D:lockentry>
                                <D:lockscope><D:shared/></D:lockscope>
                                <D:locktype><D:write/></D:locktype>
                                </D:lockentry>
                            </D:supportedlock>
                        </D:prop>
                        <D:status>HTTP/1.1 200 OK</D:status>
                    </D:propstat>
                    </D:response>
            </D:multistatus>
            "#;

            return res.status_code(StatusCode::OK).render(Text::Xml(xml));
        }
        Err(err) => {}
    }

    res.status_code(StatusCode::NOT_FOUND);
}

/*
MKCOL creates a new collection resource at the location specified by the Request-URI.
If the Request-URI is already mapped to a resource, then the MKCOL MUST fail.
During MKCOL processing, a server MUST make the Request-URI an internal member of its parent collection,
unless the Request-URI is “/”. If no such ancestor exists, the method MUST fail.

When the MKCOL operation creates a new collection resource, all ancestors MUST already exist,
or the method MUST fail with a 409 (Conflict) status code.
(RFC 4918: HTTP Extensions for Web Distributed Authoring and Versioning (WebDAV))
 */
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

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();
    CLIENT.get_or_init(init_client).await;

    // PUT http://localhost:3000/Enpass/vault.enpassdbsync
    // PROPFIND http://localhost:3000/Enpass/vault.enpassdbsync
    // PROPFIND http://localhost:3000/
    // MKCOL http://localhost:3000/Enpass/ --> OK

    let router = Router::with_path("<bucket>/<**path>")
        .get(get_handler)
        .head(ok_handler)
        .put(put_handler)
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
