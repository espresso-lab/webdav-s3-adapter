use salvo::{http::Method, prelude::*};

#[handler]
fn ok_handler(_req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK);
}

#[handler]
fn get_handler(req: &mut Request, res: &mut Response) {
    res.status_code(StatusCode::OK)
        .render(Text::Plain(req.uri().path().to_string()));
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
