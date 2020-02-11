extern crate scraper;
use scraper::{Selector};
// use futures_core::future::Future;
// use futures::future::FutureExt;
// use futures::future::Shared;
use std::future::Future;
use std::pin::Pin;

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
    Pred(Selector),
    ResponseLogic(ResponseLogic),
    Store(Box<dyn Fn(Vec<String>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>  + Send + Sync>)
}


// struct S<F>
// where
//     F: std::future::Future,
// {
//     foo: fn(u8) -> F,
// }


impl Scrape
{
    // unsafe_unpinned!(executables: C);

    pub fn new() -> Scrape {
        Scrape {
            executables: vec![]
        }
    }
    pub fn find<S: Into<String>>(mut self, predicate: S) -> Self {
        self.executables.push(Box::new(Ops::Pred(Selector::parse(&predicate.into()).unwrap())));
        self
    }
    pub fn response_logic(mut self, resp_logic: ResponseLogic) -> Self {
        self.executables.push(Box::new(Ops::ResponseLogic(resp_logic)));
        self
    }

    pub fn store(
        mut self, 
        c: Box<dyn Fn(Vec<String>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> + Send + Sync>
    ) -> Self
    {
        self.executables.push(Box::new(Ops::Store(c)));
        self
    }
}



