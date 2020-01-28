// #![type_length_limit="6954178"]
use core::future::Future;
use std::sync::{Arc, Mutex};
use std::pin::Pin;


use futures::stream::{self, StreamExt, Iter as StreamIter};
use futures::{ready, Stream};
// use futures::channel::mpsc::channel;
use async_std::sync::{channel, Receiver, Sender};
use async_std::task::{Context, Poll, sleep};

use futures::future::{join};

use std::time::Duration;

#[derive(Debug, PartialEq)]
enum QuickScraperError {
    NoStartUrls
}

#[derive(Debug)]
struct QuickScraper {
    start_urls: StreamIter<std::vec::IntoIter<String>>,
}

#[derive(Debug, Clone, PartialEq)]
struct StartUrls {
    data: Vec<String>
}

struct QuickScraperBuilder {
    start_urls: Option<StartUrls>
}

trait BuilderWithStartUrls {
    fn with(self: &mut Self, start_urls: StartUrls) -> &QuickScraperBuilder;
}


impl QuickScraperBuilder {
    fn new() -> QuickScraperBuilder {
        QuickScraperBuilder {
            start_urls: None
        }
    }
    fn finish(&self) -> Result<QuickScraper, QuickScraperError> {
        let data = self.start_urls.clone().ok_or(QuickScraperError::NoStartUrls)?.data;
        Ok(
            QuickScraper {
                start_urls: stream::iter(data)
            }
        )
    }
}


impl BuilderWithStartUrls for QuickScraperBuilder {
    fn with(&mut self, start_urls: StartUrls) -> &QuickScraperBuilder {
        self.start_urls = Some(start_urls);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
enum DataFromScraperValue{
    Complete,
    DataFromScraper {
        url: String,
        count: usize
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


struct DataDistributor {
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
                        println!("poll NONE match (done): {:?}", res);
                        return Poll::Ready(None); 
                    }
                    _ => {
                        println!("poll some match: {:?}", res);
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






async fn dispatch(count: Arc<Mutex<usize>>, data_to_manager_sender: Sender<DataFromScraperValue>, url: String) -> (){
    let mut count = count.lock().unwrap();
    *count += 1;
    let val = *count;
    std::mem::drop(count);
    println!("about to send");
    
    // if should_delay {
    //     println!("delaying...");
    //     sleep(Duration::from_secs(1)).await;
    // }
    
    println!("sending");
    let _res = data_to_manager_sender.send(
        DataFromScraperValue::DataFromScraper{
            url: url.clone(),
            count: val
        }
    ).await;
    // async_std::task::yield_now().await;
    println!("in loop: {url} {count}", url=url, count=val);

    // let res: Result<(), ()> = Ok(());
    // res
}

impl QuickScraper {
    async fn process(self) -> Result<(), String> {
        
        // let stream = &self.start_urls;
        let count = Arc::new(Mutex::new(0usize));
        let (data_to_manager_sender, data_to_manager_receiver) = channel::<DataFromScraperValue>(5);


        let stream_senders_fut: Pin<Box<dyn Future<Output=()>>> = Box::pin(self.start_urls.enumerate().map(|(i, url)| (i, count.clone(), data_to_manager_sender.clone(), url.clone())).for_each_concurrent(
            3,
            |(_i, count, data_to_manager_sender, url)| async move {
                // let i = i + 1;
                dispatch(count, data_to_manager_sender, url).await;
                async_std::task::yield_now().await;
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
            println!("finished sender stream {:?}", res);
            res
        };

        let res = join(data_distributor_stream_fut, stream_complete_fut).await;
        println!("outside loop: {:?}", res);
        Ok(())
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


    #[test]
    fn with_start_urls() {
        let mut builder = QuickScraperBuilder::new();



        builder.with(
            StartUrls{
                data: vec!["https://www.google.com".into()] 
            }
        );
        // assert_eq!(builder.start_urls.as_ref().unwrap(), &start_urls_1);
    }

    #[test]
    fn with_start_urls_finished() -> () {
        let mut builder = QuickScraperBuilder::new();
        let start_urls = StartUrls{
            data: vec![
                "https://www.test-1.com".into(),
                "https://www.test-2.com".into(),
                "https://www.test-3.com".into(),
                "https://www.test-4.com".into(),
                "https://www.test-5.com".into(),
                "https://www.test-6.com".into(),
                "https://www.test-7.com".into(),
                "https://www.test-8.com".into(),
                "https://www.test-9.com".into(),
                "https://www.test-10.com".into(),
                "https://www.test-11.com".into(),
                "https://www.test-12.com".into(),
                "https://www.test-13.com".into(),
                "https://www.test-14.com".into(),
            ] 
        };


        builder.with(
            start_urls
        );
        let scraper = builder.finish().expect("Builder could not finish");
        let res = task::block_on(async {
            println!("\n");
            let fut = scraper.process();
            // scraper.add_url("https://www.test-4.com".into());
            let res = fut.await;
            println!("\n");
            res
        });

        println!("{:?}", res);
        assert_eq!(res.is_ok(), true);

    }

    #[test]
    // #[should_panic]
    fn no_start_urls_finished() {
        let builder = QuickScraperBuilder::new();


        let scraper_result = builder.finish();
         assert!(scraper_result.is_err());
         // assert_eq!(scraper_result, Err(QuickScraperError::NoStartUrls));
    }
}