extern crate scraper;
use scraper::{Selector};


#[derive(Clone)]
pub enum ResponseLogic {
    Parallel(Vec<Scrape>),
    Serial(Vec<Scrape>)
}

// #[derive(Debug)]
pub struct StartUrl {
    pub url: Option<String>,
    pub method: Option<String>,
    pub response_logic: Option<ResponseLogic>
}


impl<'a> StartUrl {
    pub fn new() -> StartUrl {
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

#[derive(Clone)]
pub struct Scrape {
    pub executables: Vec<Box<Ops>>,
    // text: Text
}

// use std::fmt;
// impl fmt::Debug for dyn Predicate {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Predicate")
//     }
// }


#[derive(Clone)]
pub enum Ops{
    Pred(Selector),
    Text,
    NavToEach,
    ResponseLogic(ResponseLogic),
    Store
}



impl Scrape {
    pub fn new() -> Scrape {
        Scrape {
            executables: vec![]
        }
    }
    pub fn find<S: Into<String>>(mut self, predicate: S) -> Self {
        self.executables.push(Box::new(Ops::Pred(Selector::parse(&predicate.into()).unwrap())));
        self
    }
    pub fn text(mut self) -> Self {
        self.executables.push(Box::new(Ops::Text));
        self
    }
    pub fn nav_to_each(mut self) -> Self {
        self.executables.push(Box::new(Ops::NavToEach));
        self
    }
    pub fn response_logic(mut self, resp_logic: ResponseLogic) -> Self {
        self.executables.push(Box::new(Ops::ResponseLogic(resp_logic)));
        self
    }
    pub fn store(mut self) -> Self {
        self.executables.push(Box::new(Ops::Store));
        self
    }
    // pub fn to_box(self) -> Box<Self> {
    //     Box::new(self)
    // }
}



