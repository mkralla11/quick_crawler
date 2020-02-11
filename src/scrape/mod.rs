extern crate scraper;
use scraper::{Selector};
// use futures_core::future::Future;
// use futures::future::FutureExt;
// use futures::future::Shared;
use std::future::Future;
// use pin_utils::{unsafe_pinned, unsafe_unpinned};

#[derive(Clone)]
pub enum ResponseLogic<C, F>
where
    C: Fn(Vec<String>) -> F,
    F: Future
{
    Parallel(Vec<Scrape<C, F>>),
    Serial(Vec<Scrape<C, F>>)
}

// #[derive(Debug)]
pub struct StartUrl<C, F>
where
    C: Fn(Vec<String>) -> F,
    F: Future
{
    pub url: Option<String>,
    pub method: Option<String>,
    pub response_logic: Option<ResponseLogic<C, F>>
}


impl<C, F> StartUrl<C, F>
where
    C: Fn(Vec<String>) -> F,
    F: Future
{
    pub fn new() -> StartUrl<C, F> {
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
    pub fn response_logic(mut self, response_logic: ResponseLogic<C, F>) -> Self {
        self.response_logic = Some(response_logic);
        self
    }
    // pub fn to_owned(&self) -> Self {
    //     *self
    // }
}

#[derive(Clone)]
pub struct Scrape<C, F>
where
    C: Fn(Vec<String>) -> F,
    F: Future 
{
    pub executables: Vec<Box<Ops<C, F>>>,
    // text: Text
}

// use std::fmt;
// impl fmt::Debug for dyn Predicate {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Predicate")
//     }
// }


#[derive(Clone)]
pub enum Ops<C, F>
where
    C: Fn(Vec<String>) -> F,
    F: Future
{
    Pred(Selector),
    ResponseLogic(ResponseLogic<C, F>),
    Store(C)
}


// struct S<F>
// where
//     F: std::future::Future,
// {
//     foo: fn(u8) -> F,
// }


impl<C, F> Scrape<C, F>
where
    C: Fn(Vec<String>) -> F,
    F: Future<Output = ()>
{
    // unsafe_unpinned!(executables: C);

    pub fn new() -> Scrape<C, F> {
        Scrape {
            executables: vec![]
        }
    }
    pub fn find<S: Into<String>>(mut self, predicate: S) -> Self {
        self.executables.push(Box::new(Ops::Pred(Selector::parse(&predicate.into()).unwrap())));
        self
    }
    pub fn response_logic(mut self, resp_logic: ResponseLogic<C, F>) -> Self {
        self.executables.push(Box::new(Ops::ResponseLogic(resp_logic)));
        self
    }

    pub fn store(
        mut self, 
        c: C
    ) -> Self
    {
        self.executables.push(Box::new(Ops::Store(c)));
        self
    }
    // pub fn to_box(self) -> Box<Self> {
    //     Box::new(self)
    // }
}



