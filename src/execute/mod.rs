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
        Some(m) if m == "GET" =>surf::get(url),
        Some(m) if m == "POST" =>surf::post(url),
        Some(m)=>{
            return Err(format!("The method '{m}' is not a valid http method", m=m));
        }
        None=>{
            return Err("No http method".into())
        }
    };


    let html_str = req.recv_string().await.map_err(|_| "Surf error".to_string())?;
    // println!("body: {:?}", body);

    // let document = Html::parse_fragment(&body);



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
    // let mut doc = document;
        // let doc = Html::parse_fragment(&html_str);
        let mut container = HtmlContainer::new(html_str.clone(), Html::parse_fragment(&html_str));

        for (i, executable) in executables.iter().enumerate() {
            // let mut container = html_container;
            println!("executable {:?}", i);
            match &**executable {
                Pred(pred)=>{
                    println!("Pred!");
                    match container.doc_or_nodes {
                        DocOrNodes::Document=>{
                            // container.doc = Html::parse_fragment(&container.html_str);
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
                    // document.find(Class("blah")).collect();
                    
                    // handle_scrape(, document.find(pred), sender).await;
                }
                Text=>{
                    println!("no more text op");
                }
                NavToEach=>{
                    println!("no more nav to each op");
                }
                Ops::ResponseLogic(response_logic)=>{

                    println!("ResponseLogic!");
                    // let next_docs = replace(&mut container.next_docs, vec![]);
                    // html_container = None;
                    // std::mem::drop(container);
                    // handle_next_docs(next_docs, *response_logic, data_sender).await;
                    use futures::stream::{self, StreamExt};
                    // let next_htmls = vec!["<h1>Hello, <i>world!</i></h1>".into()];
                    
                    let hrefs = container.nodes.iter().map(|node| node.value().attr("href")).filter(|href| href.is_some()).map(|href| href.unwrap().to_owned());
                    


                    task::block_on(async {
                        let (sender, receiver) = channel::<DataFromScraperValue>(5);
                        let stream_senders_fut: Pin<Box<dyn Future<Output=()>>> = Box::pin(stream::iter(hrefs).enumerate().map(|(i,href)| (i, href, sender.clone())).for_each_concurrent(
                            /* limit */ 5,
                            |(i, href, snd)| async move {
                                println!("{:?}", href);
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







