extern crate futures;

use futures::{Future, Async, Poll};

fn main() {
    let app = MyApp::new()
        .use_middleware(Head)
        .use_middleware(Gzip);

    Server::new().run(app);
}

trait AppendMiddleware<Next> {
    type Middleware;

    fn use_middleware(self, middleware: Next) -> Self::Middleware;
}

trait PrependMiddleware<Next> {
    type Middleware;

    fn prepend_to(self, middleware: Next) -> Self::Middleware;
}

// Placeholders for things that would actually exist
struct HttpRequest;

impl HttpRequest {
    fn method(&self) -> &'static str {
        unimplemented!();
    }

    fn path(&self) -> &str {
        unimplemented!();
    }
}

struct HttpResponse;
struct Server;

impl Server {
    fn new() -> Self {
        Server
    }

    fn run<T: Service<Request=HttpRequest, Response=HttpResponse>>(self, _: T) {
    }
}

// This is a stand in for the user's application. The real type would be constructed by a web
// framework or routing library.
#[derive(Clone)]
struct MyApp;

impl MyApp {
    fn new() -> Self {
        MyApp
    }
}

trait Service {
    type Request;
    type Response;
    type Error;
    type Future: Future<Item=Self::Response, Error=Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future;
    fn poll_ready(&self) -> Async<()>;
}

trait Middleware: Clone {
    type Request;
    type Response;
    type Error;
    type WrappedService: Service<
        Request=Self::Request,
        Response=Self::Response,
        Error=Self::Error,
    >;

    fn wrapped_service(&self) -> &Self::WrappedService;

    fn before(&self, req: Self::Request) -> Self::Request {
        req
    }

    fn after(&self, resp: Self::Response) -> Self::Response {
        resp
    }
}

impl<T: Middleware> Service for T {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = RunAfterMiddleware<
        Self,
        <T::WrappedService as Service>::Future,
    >;

    fn call(&self, req: Self::Request) -> Self::Future {
        RunAfterMiddleware::new(
            self.clone(),
            self.wrapped_service().call(self.before(req)),
        )
    }

    fn poll_ready(&self) -> Async<()> {
        self.wrapped_service().poll_ready()
    }
}

struct RunAfterMiddleware<T, U> {
    middleware: T,
    future_response: U,
}

impl<T, U> RunAfterMiddleware<T, U> {
    fn new(middleware: T, future_response: U) -> Self {
        RunAfterMiddleware {
            middleware: middleware,
            future_response: future_response,
        }
    }
}

impl<T, U> Future for RunAfterMiddleware<T, U> where
    T: Middleware,
    U: Future<Item=T::Response>,
{
    type Item = U::Item;
    type Error = U::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.future_response.poll() {
            Ok(Async::Ready(response)) => self.middleware.after(response),
            err_or_not_ready => err_or_not_ready,
        }
    }
}

impl Service for MyApp {
    type Request = HttpRequest;
    type Response = HttpResponse;
    type Error = ();
    type Future = futures::Finished<Self::Response, Self::Error>;

    fn call(&self, _: Self::Request) -> Self::Future {
        futures::finished(HttpResponse)
    }

    fn poll_ready(&self) -> Async<()> {
        Async::Ready(())
    }
}

impl<T: PrependMiddleware<MyApp>> AppendMiddleware<T> for MyApp {
    type Middleware = T::Middleware;

    fn use_middleware(self, middleware: T) -> Self::Middleware {
        middleware.prepend_to(self)
     }
}

struct Head;

impl<T> PrependMiddleware<T> for Head {
    type Middleware = HeadMiddleware<T>;

    fn prepend_to(self, next: T) -> Self::Middleware {
        HeadMiddleware(next)
    }
}

#[derive(Clone)]
struct HeadMiddleware<T>(T);

impl<T, U: AppendMiddleware<T>> AppendMiddleware<T> for HeadMiddleware<U> {
    type Middleware = HeadMiddleware<U::Middleware>;

    fn use_middleware(self, next: T) -> Self::Middleware {
        HeadMiddleware(self.0.use_middleware(next))
    }
}

impl<T: Service<Response=HttpResponse> + Clone> Middleware for HeadMiddleware<T> {
    type Request = T::Request;
    type Response = HttpResponse;
    type Error = T::Error;
    type WrappedService = T;

    fn wrapped_service(&self) -> &Self::WrappedService {
        &self.0
    }

    fn after(&self, resp: Self::Response) -> Self::Response {
        // resp.body = stream::empty();
        resp
    }
}

struct Gzip;

impl<T> PrependMiddleware<T> for Gzip {
    type Middleware = GzipMiddleware<T>;

    fn prepend_to(self, next: T) -> Self::Middleware {
        GzipMiddleware(next)
    }
}

#[derive(Clone)]
struct GzipMiddleware<T>(T);

impl<T, U: AppendMiddleware<T>> AppendMiddleware<T> for GzipMiddleware<U> {
    type Middleware = GzipMiddleware<U::Middleware>;

    fn use_middleware(self, next: T) -> Self::Middleware {
        GzipMiddleware(self.0.use_middleware(next))
    }
}

impl<T: Service<Response=HttpResponse> + Clone> Middleware for GzipMiddleware<T> {
    type Request = T::Request;
    type Response = HttpResponse;
    type Error = T::Error;
    type WrappedService = T;

    fn wrapped_service(&self) -> &Self::WrappedService {
        &self.0
    }

    fn after(&self, resp: Self::Response) -> Self::Response {
        resp
    }
}
