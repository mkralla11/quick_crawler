// use futures::stream::{self, StreamExt};
use std::mem::replace;


use core::future::Future;

use std::pin::Pin;


use futures::future::{join};



use async_std::{task};
extern crate scraper;
use scraper::{Html};
use async_std::sync::{channel, Sender};


use crate::scrape::{StartUrl, ResponseLogic::{self, Parallel}, Ops::{self, Pred, Text, NavToEach, Store}};
use crate::{DataFromScraperValue, DataDistributor};




pub async fn execute_deep_scrape(start_url: &StartUrl, data_sender: Sender<DataFromScraperValue>)-> Result<(), String>{
    let url = match &start_url.url {
        Some(url)=>url,
        None=>{
            return Err("No Start Url".into())
        }
    };

    let req = match &start_url.method {
        Some(m) if m == "GET" =>"STUB URL REQ GET".to_string(),
        Some(m) if m == "POST" =>"STUB URL REQ POST".to_string(),
        // // FOR LIVE RESULTS
        // Some(m) if m == "GET" =>surf::get(url),
        // Some(m) if m == "POST" =>surf::post(url),
        Some(m)=>{
            return Err(format!("The method '{m}' is not a valid http method", m=m));
        }
        None=>{
            return Err("No http method".into())
        }
    };

    // // FOR LIVE RESULTS
    // let html_str = req.recv_string().await.map_err(|_| "Surf error".to_string())?;
    // println!("{:?}", html_str);



    let html_str = r#"
        <div>
            <a class="feed-item" href="https://tasty.co/compilation/another-meal-1">
                link to another meal 1
            </a>
            <a class="feed-item" href="https://tasty.co/compilation/another-meal-2">
                link to another meal 2
            </a>
            <a class="feed-item" href="https://tasty.co/compilation/another-meal-3">
                link to another meal 3
            </a>
        </div>
    "#.into();


    let response_logic = match &start_url.response_logic {
        Some(response_logic)=>response_logic.clone(),
        None=>{
            return Err("No response_logic".into())
        }
    };

    handle_response_logic(response_logic, html_str, data_sender).await;

    Ok(())
}




async fn handle_response_logic(response_logic: ResponseLogic, html_str: String, data_sender: Sender<DataFromScraperValue>) -> Result<(), String>{
    match response_logic {
        Parallel(par_items) => {
            use futures::stream::{self, StreamExt};
            // loop over each Item in array
            stream::iter(par_items).map(|item| (item, data_sender.clone(), html_str.clone())).for_each_concurrent(
                /* limit */ 4,
                |(scrape, sender, html_str)| async move {
                    handle_scrape(scrape.executables, html_str, sender).await;
                }
            ).await;
            Ok(())
        }
        _ => {
            return Err("Unknown logic".into())
        }
    }
}

enum DocOrNodes {
    Document,
    Nodes
}
struct HtmlContainer<'a> {
    doc: scraper::Html,
    html_str: String,
    nodes: Vec<scraper::ElementRef<'a>>,
    // next_htmls: Vec<String>,
    doc_or_nodes: DocOrNodes
}

impl<'a> HtmlContainer<'a>{
    fn new(html_str: String, doc: scraper::Html)-> HtmlContainer<'a> {
        HtmlContainer{
            html_str,
            doc,
            nodes: vec![],
            // next_: vec![],
            doc_or_nodes: DocOrNodes::Document
        }
    }
}

use futures::future::{BoxFuture, FutureExt};









fn handle_scrape(executables: Vec<Box<Ops>>, html_str: String, data_sender: Sender<DataFromScraperValue>)-> BoxFuture<'static, Result<(), String>>{
    async move {

        let mut container = HtmlContainer::new(html_str.clone(), Html::parse_fragment(&html_str));

        for (i, executable) in executables.iter().enumerate() {
            println!("executable {:?}", i);
            match &**executable {
                Pred(pred)=>{
                    println!("Pred!");
                    match container.doc_or_nodes {
                        DocOrNodes::Document=>{
                            let nodes = container.doc.select(&pred).collect();
                            replace(&mut container.nodes, nodes);

                            println!("found doc!");

                            container.doc_or_nodes = DocOrNodes::Nodes;
                        } 
                        DocOrNodes::Nodes=>{
                            container.nodes = container.nodes.iter().flat_map(|item| item.select(&pred)).collect::<Vec<_>>();

                            
                            container.doc_or_nodes = DocOrNodes::Nodes;
                        }

                    }
                }
                Text=>{
                    println!("no more text op");
                }
                NavToEach=>{
                    println!("no more nav to each op");
                }
                Ops::ResponseLogic(response_logic)=>{

                    println!("ResponseLogic!");

                    use futures::stream::{self, StreamExt};
                    
                    let hrefs = container.nodes.iter().map(|node| node.value().attr("href")).filter(|href| href.is_some()).map(|href| href.unwrap().to_owned());
                    

                    // Can't figure out how to remove this block on because 
                    // of Scraper crate dependency that uses Cells :(
                    task::block_on(async {
                        let (sender, receiver) = channel::<DataFromScraperValue>(5);
                        let stream_senders_fut: Pin<Box<dyn Future<Output=()>>> = Box::pin(stream::iter(hrefs).enumerate().map(|(i,href)| (i, href, sender.clone())).for_each_concurrent(
                            /* limit */ 5,
                            |(i, href, snd)| async move {
                                println!("{:?}", href);
                                // // FOR LIVE RESULTS
                                // let html_str = surf::get(href).recv_string().await.map_err(|_| "Surf error".to_string()).expect("should work");
                                let html_str = format!("<div class='ingredients-prep'><div class='ingredient'>{} test ingredent</div><div class='prep-steps'><li>step: {}</li></div></div>", i, i);
                                let _res = snd.send(
                                    DataFromScraperValue::DataFromScraper{
                                        text: html_str
                                    }
                                ).await;

                                async_std::task::yield_now().await;
                            }
                        ));


                        let data_distributor_stream = DataDistributor::new(receiver);
                        let data_distributor_stream_fut: Pin<Box<dyn Future<Output=Vec<DataFromScraperValue>>>>= Box::pin(data_distributor_stream.collect());
                        let data_to_manager_sender2 = sender.clone();
                        let stream_complete_fut = async move {
                            let res = stream_senders_fut.await;
                            let _res = data_to_manager_sender2.send(
                                DataFromScraperValue::Complete
                            ).await;
                            // println!("finished sender stream {:?}", res);
                            res
                        };



                        let (res, _) = join(data_distributor_stream_fut, stream_complete_fut).await;

                        let html_strs: Vec<String> = res.iter().map(|data_scraper_value| {
                            match data_scraper_value {
                                DataFromScraperValue::DataFromScraper{text}=>{
                                    text.clone()
                                }
                                _ => "".to_string()
                            }
                        }).collect();
                        

                        stream::iter(html_strs).map(|html_str: String| (html_str, data_sender.clone(), response_logic.clone())).for_each_concurrent(
                            /* limit */ 4,
                            |(html_str, data_sender, response_logic)| async {
                                // idea().await;
                                handle_response_logic(response_logic, html_str, data_sender).await;
                            }
                        ).await;   



                    });
                }
                Store=>{
                    println!("Store!");
                    for node in container.nodes.iter(){
                        // Can't figure out how to remove this block on because 
                        // of Scraper crate dependency that uses Cells :(
                        task::block_on(async {
                            let res = node.text().collect::<Vec<_>>().concat();
                            println!("storing! {:?}", res);
                            data_sender.send(
                                DataFromScraperValue::DataFromScraper{
                                    text: res
                                }
                            ).await;
                        });
                        
                    }
                }
            }

        }
        Ok(())
    }.boxed()
}







