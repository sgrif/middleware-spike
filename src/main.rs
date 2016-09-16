#[macro_use]
extern crate futures;

use std::sync::Arc;
use futures::{Future, Async, Poll};

fn main() {
    let app = MyApp::new()
        .use_middleware(Head)
        .use_middleware(LogRunTime(NullLogger));

    Server::new().run(app);
}

// Placeholders for things that would actually exist
struct HttpRequest;

impl HttpRequest {
    // This would of course be an enum but spiking... 'static because the real thing would be copy
    fn method(&self) -> &'static str {
        "GET"
    }

    fn path(&self) -> &str {
        "/"
    }
}

struct HttpResponse;
struct NullLogger;

/// I mostly just have this so the type of `app` gets validated, but I'll spike on the interface
/// this expects soon. It'll of course be a trait not a struct.
struct Server;

impl Server {
    fn new() -> Self {
        Server
    }

    fn run<T: Service<Request=HttpRequest, Response=HttpResponse>>(self, _: T) {
    }
}

/// This is a stand in for the user's application. The real type would be constructed by a web
/// framework or routing library.
struct MyApp;

impl MyApp {
    fn new() -> Self {
        MyApp
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

/// Inlined tokio_service::Service so I can mess with blanket impls. We'll add a wrapper type/trait
/// later. "around" middleware would implement this trait directly.
trait Service {
    type Request;
    type Response;
    type Error;
    type Future: Future<Item=Self::Response, Error=Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future;
    fn poll_ready(&self) -> Async<()>;
}

/// Traits for constructing middleware stacks. Simple "before" and "after" middleware will get
/// these for free. Full "around" middleware will need to handle wrapping the inner service
/// themselves and implement these traits.
trait AppendMiddleware<Next> {
    type Middleware;

    fn use_middleware(self, middleware: Next) -> Self::Middleware;
}

/// Since it's expected that appending to the middleware stack will be built by recursively calling
/// `use_middleware` down the stack, at the bottom we need something which can do the inverse. We
/// never want a middleware before the actual application.
///
/// There'll be an inverse form of this `stack.prepend_middleware(middleware)` which will just have
/// a default impl based on this trait, but it's exlcuded as it's unimportant for this spike.
trait PrependMiddleware<Next> {
    type Middleware;

    fn prepend_to(self, middleware: Next) -> Self::Middleware;
}

/// Helper for middleware which don't need the full "around" capabilities. This trait exists
/// because the path of least resistance for most around or after middleaware is to box the
/// resulting future. After middleware are significantly more common than around middleware, so we
/// can reduce the impact of that by providing this trait instead.
///
/// This needs to be one trait and not `BeforeMiddleware` and `AfterMiddleware` to avoid coherence
/// issues with the impl for `Service` later on. Plus, if someone has a middleware that needs to
/// run both before and after the app, but doesn't actually need a lexical scope, let's let them do
/// it.
trait Middleware {
    type Request;
    type Response;

    /// Default impl because most middleware are only before or after not both.
    fn before(&self, req: Self::Request) -> Self::Request {
        req
    }

    /// Default impl because most middleware are only before or after not both.
    fn after(&self, resp: Self::Response) -> Self::Response {
        resp
    }
}

/// If something is a simple before or after middleware they don't need to handle wrapping the next
/// item down the stack. We can do that for them.
impl<T, U> PrependMiddleware<U> for T where
    T: Middleware,
    U: Service<Request=T::Request, Response=T::Response>,
{
    type Middleware = MiddlewareService<T, U>;

    fn prepend_to(self, other: U) -> MiddlewareService<T, U> {
        MiddlewareService::new(self, other)
    }
}

/// This is a single "before"/"after" middleawre, and the rest of the stack. The middleware is
/// wrapped in an Arc so it's able to have configuration data without copying. We might be able to
/// use a stack reference eventually, but right now it's not possible with how futures are set up.
/// There was a long gitter discussion.
struct MiddlewareService<T, U> {
    middleware: Arc<T>,
    service: U,
}

impl<T, U> MiddlewareService<T, U> {
    fn new(middleware: T, service: U) -> Self {
        MiddlewareService {
            middleware: Arc::new(middleware),
            service: service,
        }
    }
}

impl<T, U, V> AppendMiddleware<V> for MiddlewareService<T, U> where
    U: AppendMiddleware<V>,
{
    type Middleware = MiddlewareService<T, U::Middleware>;

    fn use_middleware(self, middleware: V) -> Self::Middleware {
        MiddlewareService {
            middleware: self.middleware,
            service: self.service.use_middleware(middleware),
        }
    }
}

impl<T, U> Service for MiddlewareService<T, U> where
    T: Middleware,
    U: Service<Request=T::Request, Response=T::Response>,
{
    type Request = U::Request;
    type Response = U::Response;
    type Error = U::Error;
    type Future = RunAfterMiddleware<T, U::Future>;

    fn call(&self, req: Self::Request) -> Self::Future {
        RunAfterMiddleware::new(
            self.middleware.clone(),
            self.service.call(self.middleware.before(req)),
        )
    }

    fn poll_ready(&self) -> Async<()> {
        self.service.poll_ready()
    }
}

/// This struct exists because doing the ideomatic thing would require boxing the future.
struct RunAfterMiddleware<T, U> {
    middleware: Arc<T>,
    future_response: U,
}

impl<T, U> RunAfterMiddleware<T, U> {
    fn new(middleware: Arc<T>, future_response: U) -> Self {
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
            Ok(Async::Ready(response)) => {
                Ok(Async::Ready(self.middleware.after(response)))
            }
            err_or_not_ready => err_or_not_ready,
        }
    }
}

// =======
// Example after middleware
// =======

struct Head;

impl Middleware for Head {
    type Request = HttpRequest;
    type Response = HttpResponse;

    fn after(&self, resp: Self::Response) -> Self::Response {
        // resp.body = stream::empty();
        resp
    }
}

// =======
// Example around middleware. Things get more verbose here.
// =======

struct LogRunTime<T>(T);

impl<T, U> PrependMiddleware<U> for LogRunTime<T> {
    type Middleware = LogRunTimeMiddleware<T, U>;

    fn prepend_to(self, other: U) -> Self::Middleware {
        LogRunTimeMiddleware::new(Arc::new(self.0), other)
    }
}

struct LogRunTimeMiddleware<T, U> {
    logger: Arc<T>,
    wrapped_service: U,
}

impl<T, U> LogRunTimeMiddleware<T, U> {
    fn new(logger: Arc<T>, wrapped_service: U) -> Self {
        LogRunTimeMiddleware {
            logger: logger,
            wrapped_service: wrapped_service,
        }
    }
}

impl<T, U, V> AppendMiddleware<V> for LogRunTimeMiddleware<T, U> where
    U: AppendMiddleware<V>,
{
    type Middleware = LogRunTimeMiddleware<T, U::Middleware>;

    fn use_middleware(self, other: V) -> Self::Middleware {
        LogRunTimeMiddleware::new(self.logger, self.wrapped_service.use_middleware(other))
    }
}

use std::time::Instant;

/// One thing that stood out to me here is that there's really no way to pull something off of the
/// request to use after the response is ready without copying.
impl<T, U> Service for LogRunTimeMiddleware<T, U> where
    U: Service<Request=HttpRequest>,
{
    type Request = HttpRequest;
    type Response = U::Response;
    type Error = U::Error;
    type Future = LogRunTimeFuture<T, U::Future>;

    fn call(&self, req: HttpRequest) -> Self::Future {
        let start_time = Instant::now();
        let method = req.method();
        let path = req.path().to_string();
        let logger = self.logger.clone();
        let future_response = self.wrapped_service.call(req);
        LogRunTimeFuture {
            start_time: start_time,
            method: method,
            path: path,
            logger: logger,
            future_response: future_response,
        }
    }

    fn poll_ready(&self) -> Async<()> {
        self.wrapped_service.poll_ready()
    }
}

struct LogRunTimeFuture<T, U> {
    start_time: Instant,
    method: &'static str,
    path: String,
    logger: Arc<T>,
    future_response: U,
}

impl<T, U: Future> Future for LogRunTimeFuture<T, U> {
    type Item = U::Item;
    type Error = U::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.future_response.poll());
        let elapsed = self.start_time.elapsed();
        let log_message = format!("Served {} {} in {}s {}ns",
            self.method, self.path, elapsed.as_secs(), elapsed.subsec_nanos());
        // self.logger.log("DEBUG", &log_message);
        Ok(Async::Ready(response))
    }
}
