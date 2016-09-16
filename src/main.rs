extern crate futures;

use futures::{Future, Async};

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

trait Middleware {
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

    fn after(resp: Self::Response) -> Self::Response {
        resp
    }
}

impl<T: Middleware> Service for T {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = futures::Map<
        <T::WrappedService as Service>::Future,
        fn(Self::Response) -> Self::Response,
    >;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.wrapped_service().call(self.before(req))
            .map(T::after)
    }

    fn poll_ready(&self) -> Async<()> {
        self.wrapped_service().poll_ready()
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

struct HeadMiddleware<T>(T);

impl<T, U: AppendMiddleware<T>> AppendMiddleware<T> for HeadMiddleware<U> {
    type Middleware = HeadMiddleware<U::Middleware>;

    fn use_middleware(self, next: T) -> Self::Middleware {
        HeadMiddleware(self.0.use_middleware(next))
    }
}

impl<T: Service<Response=HttpResponse>> Middleware for HeadMiddleware<T> {
    type Request = T::Request;
    type Response = HttpResponse;
    type Error = T::Error;
    type WrappedService = T;

    fn wrapped_service(&self) -> &Self::WrappedService {
        &self.0
    }

    fn after(resp: Self::Response) -> Self::Response {
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

struct GzipMiddleware<T>(T);

impl<T, U: AppendMiddleware<T>> AppendMiddleware<T> for GzipMiddleware<U> {
    type Middleware = GzipMiddleware<U::Middleware>;

    fn use_middleware(self, next: T) -> Self::Middleware {
        GzipMiddleware(self.0.use_middleware(next))
    }
}

impl<T: Service<Response=HttpResponse>> Middleware for GzipMiddleware<T> {
    type Request = T::Request;
    type Response = HttpResponse;
    type Error = T::Error;
    type WrappedService = T;

    fn wrapped_service(&self) -> &Self::WrappedService {
        &self.0
    }

    fn after(resp: Self::Response) -> Self::Response {
        resp
    }
}
