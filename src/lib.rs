//! # QuickCrawler
//! QuickCrawler is a Rust crate that provides a completely async, declarative web crawler 
//! with domain-specific request rate-limiting built-in.
//! 
//! # Examples
//! 
//! Let's say you are trying to crawl a subset of pages for a given domain:
//! 
//!   `https://bike-site.com/search?q=red-bikes`
//! 
//!   and a regular GET request will return:
//! 
//!   ```html
//!   <html>
//!       <body>
//!           <div>
//!               <a class="bike-item" href="https://bike-site.com/red-bike-1">
//!                   cool red bike 1
//!               </a><a class="bike-item" href="https://bike-site.com/red-bike-2">
//!                   cool red bike 2
//!               </a>
//!               <a class="bike-item" href="https://bike-site.com/red-bike-3">
//!                   cool red bike 3
//!               </a>
//!               <div>
//!                   <a class="bike-other-item" href="https://bike-site.com/other-red-bike-4">
//!                       other cool red bike 4
//!                   </a>
//!               </div>
//!           </div>
//!       </body>
//!   </html>
//! ```
//! 
//! 
//!   and when navigating to *links 1 through 3* on that page, EACH PAGE returns:
//! 
//!   ```html
//!   <html>
//!       <body>
//!           <div class='awesome-bike'>
//!               <div class='bike-info'>
//!                   The best bike ever.
//!               </div>
//!               <ul class='bike-specs'>
//!                   <li>
//!                       Super fast.
//!                   </li>
//!                   <li>
//!                       Jumps high.
//!                   </li>
//!               </ul>
//!           </div>
//!       </body>
//!   </html>
//!   ```
//! 
//! 
//!   and when navigating to *the last link* on that page, it returns:
//! 
//!    ```html
//!   <html>
//!       <body>
//!           <div class='other-bike'>
//!               <div class='other-bike-info'>
//!                   The best bike ever.
//!               </div>
//!               <ul class='other-bike-specs'>
//!                   <li>
//!                       Super slow.
//!                   </li>
//!                   <li>
//!                       Doesn't jump.
//!                   </li>
//!               </ul>
//!           </div>
//!       </body>
//!   </html>
//!   ```
//! 
//! 
//!   QuickCrawler declaratively helps you crawl, and scrape data from each of the given pages with ease:
//! 
//! 
//!   ```rust, no_run
//!   use quick_crawler::{
//!       QuickCrawler, 
//!       QuickCrawlerBuilder,
//!       limiter::Limiter, 
//!       RequestHandlerConfig,
//!       QuickCrawlerError,
//!       scrape::{
//!           ResponseLogic::Parallel, 
//!           StartUrl, 
//!           Scrape, 
//!           ElementUrlExtractor, 
//!           ElementDataExtractor
//!       }
//!   };
//! 
//! 
//!   fn main() {
//!       let mut builder = QuickCrawlerBuilder::new();
//! 
//! 
//!       let start_urls = vec![
//!           StartUrl::new()
//!               .url("https://bike-site.com/search?q=red-bikes")
//!               .method("GET")
//!               .response_logic(Parallel(vec![
//!                   // All Scrapers below will be provided the html page response body
//!                   Scrape::new()
//!                       .find_elements_with_urls(".bike-item")
//!                       .extract_urls_from_elements(ElementUrlExtractor::Attr("href".to_string()))
//!                       // now setup the logic to execute on each of the return html pages
//!                       .response_logic(Parallel(vec![
//!                           Scrape::new()
//!                               .find_elements_with_data(".awesome-bike .bike-info")
//!                               .extract_data_from_elements(ElementDataExtractor::Text)
//!                               .store(|vec: Vec<String>| async move {
//!                                   println!("store bike info in DB: {:?}", vec);
//!                               }),
//!                           Scrape::new()
//!                               .find_elements_with_data(".bike-specs li")
//!                               .extract_data_from_elements(ElementDataExtractor::Text)
//!                               .store(|vec: Vec<String>| async move {
//!                                   println!("store bike specs in DB: {:?}", vec);
//!                               }),
//!                       ])),
//!                   Scrape::new()
//!                       .find_elements_with_urls(".bike-other-item")
//!                       .extract_urls_from_elements(ElementUrlExtractor::Attr("href".to_string()))
//!                       .response_logic(Parallel(vec![
//!                           Scrape::new()
//!                               .find_elements_with_data(".other-bike .other-bike-info")
//!                               .extract_data_from_elements(ElementDataExtractor::Text)
//!                               .store(|vec: Vec<String>| async move {
//!                                   println!("store other bike info in DB: {:?}", vec);
//!                               }),
//!                           Scrape::new()
//!                               .find_elements_with_data(".other-bike-specs li")
//!                               .extract_data_from_elements(ElementDataExtractor::Text)
//!                               .store(|vec: Vec<String>| async move {
//!                                   println!("store other bike specs in DB: {:?}", vec);
//!                               }),
//!                       ]))  
//!               ])
//!           )
//!           // more StartUrl::new 's if you feel ambitious
//!       ] ;
//! 
//!       // It's smart to use a limiter - for now automatically set to 3 request per second per domain.
//!       // This will soon be configurable.
//! 
//!       let limiter = Limiter::new();
//! 
//!       builder
//!           .with_start_urls(
//!               start_urls
//!           )
//!           .with_limiter(
//!               limiter
//!           )
//!           // Optionally configure how to make a request and return an html string
//!           .with_request_handler(
//!               |config: RequestHandlerConfig| async move {
//!                   // ... use any request library, like reqwest
//!                   surf::get(config.url.clone()).recv_string().await.map_err(|_| QuickCrawlerError::RequestErr)
//!               }
//!           );
//!
//!       let crawler = builder.finish().map_err(|_| "Builder could not finish").expect("no error");
//!         
//!       // QuickCrawler is async, so choose your favorite executor.
//!       // (Tested and working for both async-std and tokio)
//!       let res = async_std::task::block_on(async {
//!           crawler.process().await
//!       });
//!    }
//!    ```



use std::future::Future;
use std::sync::{Arc, Mutex};
use std::pin::Pin;
mod execute;
use crate::execute::execute_deep_scrape;
pub mod limiter;
use limiter::Limiter;

#[macro_use]
extern crate debug_stub_derive;


pub mod scrape;
use crate::scrape::{ResponseLogic::Parallel, StartUrl, Scrape, ElementUrlExtractor, ElementDataExtractor};

use futures::stream::{self, StreamExt, TryStreamExt, Iter as StreamIter};
use futures::{ready, Stream};
// use futures::channel::mpsc::channel;
use async_std::sync::{channel, Receiver, Sender};
use async_std::task::{Context, Poll, sleep};

use futures::future::{join, BoxFuture};

use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum QuickCrawlerError {
    NoStartUrls,
    NoUrlInStartUrlErr,
    ParseDomainErr,
    SurfRequestErr,
    RequestErr,
    NoStartUrlMethodErr,
    InvalidStartUrlMethodErr(String),
    NoResponseLogicErr,
    UnknownResponseLogicErr
}

// #[derive(Debug)]
pub struct QuickCrawler<'a>
{
    start_urls: StreamIter<std::slice::Iter<'a, StartUrl>>,
    limiter: Option<Arc<Limiter>>,
    request_handler: Arc<DynRequestHandler>
}

// #[derive(Debug)]
// struct StartUrls
// {
//     data: Vec<StartUrl>
// }

pub struct QuickCrawlerBuilder
{
    start_urls: Option<Vec<StartUrl>>,
    limiter: Option<Arc<Limiter>>,
    request_handler: Option<Arc<DynRequestHandler>>
}



impl QuickCrawlerBuilder
{
    pub fn new() -> QuickCrawlerBuilder{
        QuickCrawlerBuilder {
            start_urls: None,
            limiter: None,
            request_handler: None
        }
    }

    pub fn with_start_urls<'a>(&'a mut self, start_urls: Vec<StartUrl>) -> &'a mut QuickCrawlerBuilder {
        self.start_urls = Some(start_urls);
        self
    }



    pub fn with_limiter<'a>(&'a mut self, limiter: Limiter) -> &'a mut QuickCrawlerBuilder {
        self.limiter = Some(Arc::new(limiter));
        self
    }

    pub fn with_request_handler<'a>(&'a mut self, request_handler: impl RequestHandler) -> &'a mut QuickCrawlerBuilder {
        self.request_handler = Some(Arc::new(request_handler));
        self
    }


    pub fn finish(&self) -> Result<QuickCrawler, QuickCrawlerError> {
        let data = self.start_urls.as_ref().ok_or(QuickCrawlerError::NoStartUrls)?;
        
        let request_handler = match self.request_handler.clone() {
            Some(r) => r,
            None => {
                Arc::new(|config: RequestHandlerConfig| async move {
                    // ... use any request library, like reqwest
                    surf::get(config.url.clone()).recv_string().await.map_err(|_| QuickCrawlerError::RequestErr)
                })
            }
        };


        Ok(
            QuickCrawler {
                start_urls: stream::iter(data),
                limiter: self.limiter.clone(),
                request_handler: request_handler
            }
        )
    }
}

pub struct RequestHandlerConfig {
    pub url: String
}


pub trait RequestHandler: Send + Sync + 'static {
    /// Invoke the endpoint within the given context
    fn call<'a>(&'a self, config: RequestHandlerConfig) -> BoxFuture<'a, Result<String, QuickCrawlerError>>;
}

pub type DynRequestHandler = dyn RequestHandler;

impl<F: Send + Sync + 'static, Fut> RequestHandler for F
where
    F: Fn(RequestHandlerConfig) -> Fut,
    Fut: Future<Output=Result<String, QuickCrawlerError>> + Send + 'static,
{
    fn call<'a>(&'a self, config: RequestHandlerConfig) -> BoxFuture<'a, Result<String, QuickCrawlerError>> {
        let fut = (self)(config);
        Box::pin(async move { fut.await })
        
    }
}


// impl BuilderWithStartUrls for QuickCrawlerBuilder
// {
//     fn with(&mut self, start_urls: StartUrls) -> &QuickCrawlerBuilder {
//         self.start_urls = Some(start_urls);
//         self
//     }
// }

// impl BuilderWithLimiter for QuickCrawlerBuilder
// {
//     fn with(&mut self, limiter: Limiter) -> &QuickCrawlerBuilder {
//         self.limiter = Some(limiter);
//         self
//     }
// }



#[derive(Debug)]
pub enum DataFromScraperValue{
    Complete,
    DataFromScraper {
        text: String
    }
}

trait DataDistributorComplete{
    fn is_complete_sentinal(&self) -> bool;
}

impl DataDistributorComplete for DataFromScraperValue {
    fn is_complete_sentinal(&self) -> bool {
        match self {
            Self::Complete => true,
            _ => false,
        }
    }
}

// #[derive(Debug, Clone, PartialEq)]
// struct DataFromScraper {
//     url: String,
//     count: usize
// }


pub struct DataDistributor {
    receiver: Receiver<DataFromScraperValue>
}

impl DataDistributor {
    fn new(receiver: Receiver<DataFromScraperValue>) -> DataDistributor {
        DataDistributor {
            receiver
        }
    }
}


impl Stream for DataDistributor {
    /// The type of the value yielded by the stream.
    type Item = DataFromScraperValue;

    /// Attempt to resolve the next item in the stream.
    /// Retuns `Poll::Pending` if not ready, `Poll::Ready(Some(x))` if a value
    /// is ready, and `Poll::Ready(None)` if the stream has completed.
    fn poll_next<'a>(mut self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<Self::Item>> {
            let Self {
                receiver
            } = &mut *self;

            let empty = receiver.is_empty();
            // println!("here! {:?}", empty);
            if empty {
                // cx.waker().clone().wake();
                return Poll::Pending;
            }
            else {
                let mut unwrapped_fut = Box::pin(async move {
                    receiver.recv().await.unwrap()
                });
                

                let res = ready!(unwrapped_fut.as_mut().poll(cx));

                match res.is_complete_sentinal() {
                    true => {
                        // println!("poll NONE match (done): {:?}", res);
                        return Poll::Ready(None); 
                    }
                    _ => {
                        // println!("poll some match: {:?}", res);
                        return Poll::Ready(Some(res));
                    }
                }

                // return Poll::Ready(Some(
                //     DataFromScraper{
                //         url: "test".into(),
                //         count: 1
                //     }
                // ));
            }
        }
}






async fn dispatch<'a>(
    data_to_manager_sender: Sender<DataFromScraperValue>, 
    start_url: &'a StartUrl, 
    limiter: Option<Arc<Limiter>>, 
    request_handler: Arc<DynRequestHandler>
) -> Result<(), QuickCrawlerError>
{
    // let mut count = count.lock().unwrap();
    // *count += 1;
    // let val = *count;
    // std::mem::drop(count);
    // println!("about to send");
    
    // if should_delay {
    //     println!("delaying...");
    //     sleep(Duration::from_secs(1)).await;
    // }
    
    // println!("sending");
    // let _res = data_to_manager_sender.send(
    //     DataFromScraperValue::DataFromScraper{
    //         url: start_url.url.clone().unwrap(),
    //         count: val
    //     }
    // ).await;
    execute_deep_scrape(&start_url, data_to_manager_sender, limiter, request_handler).await?;
    // async_std::task::yield_now().await;
    // println!("in loop: {:?} {:?} {:?}", start_url.url, val, res);

    // let res: Result<(), ()> = Ok(());
    // res
    Ok(())
}

impl<'a> QuickCrawler<'a>
{
    pub async fn process(self) -> Result<Vec<DataFromScraperValue>, QuickCrawlerError> {
        
        // let stream = &self.start_urls;
        // let count = Arc::new(Mutex::new(0usize));
        let (data_to_manager_sender, data_to_manager_receiver) = channel::<DataFromScraperValue>(100);

        let limiter = self.limiter;
        let request_handler = self.request_handler;

        let stream_senders_fut: Pin<Box<dyn Future<Output=Result<(), QuickCrawlerError>>>> = Box::pin(self.start_urls.map(|url| (data_to_manager_sender.clone(), url, limiter.clone(), request_handler.clone())).map(Ok).try_for_each_concurrent(
            3,
            |(data_to_manager_sender, start_url, limiter, request_handler)| async move {
                // let i = i + 1;
                let res = dispatch(data_to_manager_sender, start_url, limiter, request_handler).await;
                async_std::task::yield_now().await;
                res
            }
        ));





        // let collect_fut = collect_results_for_receiver(data_to_manager_receiver);
        let data_distributor_stream = DataDistributor::new(data_to_manager_receiver);
        let data_distributor_stream_fut: Pin<Box<dyn Future<Output=Vec<DataFromScraperValue>>>>= Box::pin(data_distributor_stream.collect());
        // let res = data_to_manager_receiver.recv().await.ok_or("Error 3")?;
        let data_to_manager_sender2 = data_to_manager_sender.clone();
        let stream_complete_fut = async move {
            let res = stream_senders_fut.await;
            let _res = data_to_manager_sender2.send(
                DataFromScraperValue::Complete
            ).await;
            // println!("finished sender stream {:?}", res);
            res
        };

        let (data, _) = join(data_distributor_stream_fut, stream_complete_fut).await;
        // println!("outside loop: {:?}", data);
        Ok(data)
    }
    // fn add_url(&mut self, url: String) -> &Self {
    //     let new_stream = stream::iter(vec![url]);
    //     self.start_urls = self.start_urls.chain(url);
    //     self
    // }
}





#[cfg(test)]
mod tests {
    use super::*;
    use async_std::{task};

    use mockito::{mock, server_address, Matcher};
    // #[test]
    // fn with_start_urls() {
    //     let mut builder = QuickCrawlerBuilder::new();



    //     builder.with(
    //         StartUrls{
    //             data: vec!["https://www.google.com".into()] 
    //         }
    //     );
    //     // assert_eq!(builder.start_urls.as_ref().unwrap(), &start_urls_1);
    // }



    #[test]
    fn with_start_urls_finished() -> () {
        let base_url = &mockito::server_url();
        let start_path = "/search?q=dinner";
        let path1 = "/compilation/another-meal-1";
        let path2 = "/compilation/another-meal-2";
        let path3 = "/compilation/another-meal-3";
        // relative path
        let path4 = "../compilation/other-meal-1";

        let start_url = format!("{}{}", base_url, start_path);
        let url1 = format!("{}{}", base_url, path1);
        let url2 = format!("{}{}", base_url, path2);
        let url3 = format!("{}{}", base_url, path3);
        let url4 = format!("{}{}", base_url, path4);

        // Make sure to support RELATIVE PATH
        // (as shown below using path4 variable)
        let _m1 = mock("GET", start_path)
            .with_body(
                format!(r#"
                    <html>
                        <div>
                            <a class="feed-item" href="{}">
                                link to another meal 1
                            </a><a class="feed-item" href="{}">
                                link to another meal 2
                            </a>
                            <a class="feed-item" href="{}">
                                link to another meal 3
                            </a>
                            <div>
                                <a class="other-feed-item" href="{}">
                                    other link to another meal 1
                                </a>
                            </div>
                        </div>
                    </html>
                "#, url1, url2, url3, path4)
            )
            .create();
        

        let _m2 = mock("GET", Matcher::Regex(r"^/compilation/another-meal-1$".to_string()))
            .with_body(
                format!(r#"
                    <div class='ingredients-prep'>
                        <div class='ingredient'>
                            set 1: test ingredient 1
                        </div>
                        <div class='ingredient'>
                            set 1: test ingredient 2
                        </div>
                        <div class='prep-steps'>
                            <li>
                                set 1: step 1
                            </li>
                        </div>
                    </div>
                "#)
            )
            .create();

        let _m2 = mock("GET", Matcher::Regex(r"^/compilation/another-meal-(2|3)$".to_string()))
            .with_body(
                format!(r#"
                    <div class='ingredients-prep'>
                        <div class='ingredient'>
                            set 2: test ingredient 1
                        </div>
                        <div class='ingredient'>
                            set 2: test ingredient 2
                        </div>
                        <div class='prep-steps'>
                            <li>
                                set 2: step 1
                            </li>
                            <li>
                                set 2: step 2
                            </li>
                        </div>
                    </div>
                "#)
            )
            .create();

        let _m3 = mock("GET", Matcher::Regex(r"^/compilation/other-meal-1$".to_string()))
            .with_body(
                format!(r#"
                    <div class='ingredients-prep'>
                        <div class='ingredient'>
                            other ingredient 1
                        </div>
                        <div class='ingredient'>
                            other ingredient 2
                        </div>
                        <div class='prep-steps'>
                            <li>
                                other step 1
                            </li>
                            <li>
                                other step 2
                            </li>
                            <li>
                                other step 3
                            </li>
                        </div>
                    </div>
                "#)
            )
            .create();
        // format!("<div class='ingredients-prep'><div class='ingredient'>{} test ingredent</div><div class='ingredient'>{} test ingredent</div><div class='prep-steps'><li>step: {}</li></div></div>", i, i, i);




        let mut builder = QuickCrawlerBuilder::new();

        // println!("the start_url {}", start_url);

        let start_urls = vec![
            StartUrl::new()
                .url(start_url)
                .method("GET")
                .response_logic(Parallel(vec![
                    // will be provided an html page
                    Scrape::new()
                        .find_elements_with_urls(".feed-item")
                        .extract_urls_from_elements(ElementUrlExtractor::Attr("href".to_string()))
                        .response_logic(Parallel(vec![
                            Scrape::new()
                                .find_elements_with_data(".ingredients-prep .ingredient")
                                .extract_data_from_elements(ElementDataExtractor::Text)
                                .store(|vec: Vec<String>| async move {
                                    println!("store ingredients: {:?}", vec);
                                }),
                            Scrape::new()
                                .find_elements_with_data(".ingredients-prep .prep-steps li")
                                .extract_data_from_elements(ElementDataExtractor::Text)
                                .store(|vec: Vec<String>| async move {
                                    println!("store prep-steps: {:?}", vec);
                                }),
                        ])),
                    Scrape::new()
                        .find_elements_with_urls(".other-feed-item")
                        .extract_urls_from_elements(ElementUrlExtractor::Attr("href".to_string()))
                        .response_logic(Parallel(vec![
                            Scrape::new()
                                .find_elements_with_data(".ingredients-prep .ingredient")
                                .extract_data_from_elements(ElementDataExtractor::Text)
                                .store(|vec: Vec<String>| async move {
                                    println!("store ingredients: {:?}", vec);
                                }),
                            Scrape::new()
                                .find_elements_with_data(".ingredients-prep .prep-steps li")
                                .extract_data_from_elements(ElementDataExtractor::Text)
                                .store(|vec: Vec<String>| async move {
                                    println!("store prep-steps: {:?}", vec);
                                }),
                        ]))  
                ])
            )
            // more StartUrl::new 's 
        ] ;


        let limiter = Limiter::new();

        builder
            .with_start_urls(
                start_urls
            )
            .with_limiter(
                limiter
            )
            .with_request_handler(
                |config: RequestHandlerConfig| async move {
                    // ... use any request library, like reqwest
                    surf::get(config.url.clone()).recv_string().await.map_err(|_| QuickCrawlerError::RequestErr)
                }
            );


        let crawler = builder.finish().map_err(|_| "Builder could not finish").expect("no error");
        let res = task::block_on(async {
            crawler.process().await
        });

        println!("{:?}", res);
        assert_eq!(res.is_ok(), true);

    }

    // #[test]
    // // #[should_panic]
    // fn no_start_urls_finished() {
    //     let builder = QuickCrawlerBuilder::new();


    //     let scraper_result = builder.finish();
    //      assert!(scraper_result.is_err());
    //      // assert_eq!(scraper_result, Err(QuickCrawlerError::NoStartUrls));
    // }
}
