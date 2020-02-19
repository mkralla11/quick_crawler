extern crate scraper;
use scraper::{Selector};
// use futures_core::future::Future;
// use futures::future::FutureExt;
// use futures::future::Shared;
use std::future::Future;
use std::pin::Pin;
use futures::future::{BoxFuture};

// use pin_utils::{unsafe_pinned, unsafe_unpinned};

// #[derive(Clone)]
pub enum ResponseLogic
{
    Parallel(Vec<Scrape>),
    Serial(Vec<Scrape>)
}

// #[derive(Debug)]
pub struct StartUrl
{
    pub url: Option<String>,
    pub method: Option<String>,
    pub response_logic: Option<ResponseLogic>
}



pub trait Store: Send + Sync + 'static {
    /// Invoke the endpoint within the given context
    fn call<'a>(&'a self, data: Vec<String>) -> BoxFuture<'a, ()>;
}

pub type DynStore = dyn Store;

impl<F: Send + Sync + 'static, Fut> Store for F
where
    F: Fn(Vec<String>) -> Fut,
    Fut: Future + Send + 'static,
{
    fn call<'a>(&'a self, data: Vec<String>) -> BoxFuture<'a, ()> {
        let fut = (self)(data);
        Box::pin(async move { fut.await; })
    }
}


impl StartUrl
{
    pub fn new() -> StartUrl{
        StartUrl {
            url: None,
            method: None,
            response_logic: None
        }
    }
    pub fn url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }
    pub fn method<S: Into<String>>(mut self, method: S) -> Self {
        self.method = Some(method.into());
        self
    }
    pub fn response_logic(mut self, response_logic: ResponseLogic) -> Self {
        self.response_logic = Some(response_logic);
        self
    }
    // pub fn to_owned(&self) -> Self {
    //     *self
    // }
}

// #[derive(Clone)]
pub struct Scrape
{
    pub executables: Vec<Box<Ops>>,
    // text: Text
}

// use std::fmt;
// impl fmt::Debug for dyn Predicate {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Predicate")
//     }
// }


// #[derive(Clone)]
pub enum Ops
{
    // Pred(Selector),
    UrlSelector(Selector),
    UrlExtractor(ElementUrlExtractor),
    DataSelector(Selector),
    DataExtractor(ElementDataExtractor),
    ResponseLogic(ResponseLogic),
    Store(Box<DynStore>)
}


// struct S<F>
// where
//     F: std::future::Future,
// {
//     foo: fn(u8) -> F,
// }

pub enum ElementUrlExtractor {
    Attr(String)
}

pub enum ElementDataExtractor {
    Text
}

impl Scrape
{
    // unsafe_unpinned!(executables: C);

    pub fn new() -> Scrape {
        Scrape {
            executables: vec![]
        }
    }
    // pub fn find<S: Into<String>>(mut self, predicate: S) -> Self {
    //     self.executables.push(Box::new(Ops::Pred(Selector::parse(&predicate.into()).unwrap())));
    //     self
    // }
    pub fn find_elements_with_data<S: Into<String>>(mut self, predicate: S) -> Self {
        self.executables.push(Box::new(Ops::DataSelector(Selector::parse(&predicate.into()).unwrap())));
        self
    }
    pub fn extract_data_from_elements(mut self, extractor: ElementDataExtractor) -> Self {
        self.executables.push(Box::new(Ops::DataExtractor(extractor)));
        self
    }

    pub fn find_elements_with_urls<S: Into<String>>(mut self, predicate: S) -> Self {
        self.executables.push(Box::new(Ops::UrlSelector(Selector::parse(&predicate.into()).unwrap())));
        self
    }
    pub fn extract_urls_from_elements(mut self, extractor: ElementUrlExtractor) -> Self {
        self.executables.push(Box::new(Ops::UrlExtractor(extractor)));
        self
    }
    pub fn response_logic(mut self, resp_logic: ResponseLogic) -> Self {
        self.executables.push(Box::new(Ops::ResponseLogic(resp_logic)));
        self
    }

    pub fn store(
        mut self, 
        c: impl Store
    ) -> Self
    {
        self.executables.push(Box::new(Ops::Store(Box::new(c))));
        self
    }
}



