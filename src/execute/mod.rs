// use futures::stream::{self, StreamExt};
use std::mem::replace;
use std::sync::{Arc};

use crate::limiter::Limiter;
use url::Url;
use std::future::Future;

use std::pin::Pin;


use futures::future::{join};








use async_std::{task};
extern crate scraper;
use scraper::{Html};
use async_std::sync::{Sender};


use crate::scrape::{StartUrl, ResponseLogic::{self, Parallel}, Ops::{self, Pred, Store}};
use crate::{DataFromScraperValue, DataDistributor};


async fn limit_url_via<S: Into<String>>(limiter: &Option<Arc<Limiter>>, url: S) -> () {

    if limiter.is_some() {
        let base = Url::parse(&url.into()).expect("could not parse url");
        let domain = base.domain().unwrap();
        limiter.as_ref().unwrap().limit(domain).await;
    }
}



pub async fn execute_deep_scrape<'a>(start_url: &StartUrl, data_sender: Sender<DataFromScraperValue>, limiter: Option<Arc<Limiter>>)-> Result<(), String>
{
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

    limit_url_via(&limiter, url).await;
    
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
            <div>
                <a class="other-feed-item" href="https://tasty.co/compilation/other-another-meal-1">
                    other link to another meal 1
                </a>
            </div>
        </div>
    "#.into();


    let response_logic = match &start_url.response_logic {
        Some(response_logic)=>response_logic.clone(),
        None=>{
            return Err("No response_logic".into())
        }
    };

    handle_response_logic(&response_logic, html_str, data_sender, limiter).await;

    Ok(())
}




async fn handle_response_logic<'a>(response_logic: &'a ResponseLogic, html_str: String, data_sender: Sender<DataFromScraperValue>, limiter: Option<Arc<Limiter>>) -> Result<(), String>

{
    match response_logic {
        Parallel(par_items) => {
            use futures::stream::{self, StreamExt};
            // loop over each Item in array
            stream::iter(par_items).map(|item| (item, data_sender.clone(), html_str.clone(), limiter.clone())).for_each_concurrent(
                /* limit */ 4,
                |(scrape, sender, html_str, limiter)| async move {
                    handle_scrape(&scrape.executables, html_str, sender, limiter).await;
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
struct HtmlContainer {
    html_str: String,
    node_strs: Vec<String>,
    node_hrefs: Vec<String>,
    doc_or_nodes: DocOrNodes
}

impl HtmlContainer{
    fn new(html_str: String)-> HtmlContainer {
        HtmlContainer{
            html_str,
            node_strs: vec![],
            node_hrefs: vec![],
            // next_: vec![],
            doc_or_nodes: DocOrNodes::Document
        }
    }
}

use futures::future::{BoxFuture, FutureExt};









fn handle_scrape<'a>(executables: &'a Vec<Box<Ops>>, html_str: String, data_sender: Sender<DataFromScraperValue>, limiter: Option<Arc<Limiter>>)-> BoxFuture<'a, Result<(), String>>
{
    Box::pin(async move {

        let mut container = HtmlContainer::new(html_str.clone());

        for (i, executable) in executables.iter().enumerate() {
            println!("executable {:?}", i);
            match &**executable {
                Pred(pred)=>{
                    println!("Pred!");
                    match container.doc_or_nodes {
                        DocOrNodes::Document=>{
                            let node_strs = replace(&mut container.node_strs, vec![]);
                            container.node_hrefs = vec![];
                            Html::parse_fragment(&container.html_str).select(&pred).for_each(|node| {
                                
                                container.node_strs.push(node.html().to_owned());
                                let href = node.value().attr("href");
                                if href.is_some() {
                                    container.node_hrefs.push(href.unwrap().to_owned());
                                }
                            });


                            container.doc_or_nodes = DocOrNodes::Nodes;
                        } 
                        DocOrNodes::Nodes=>{
                            let node_strs = replace(&mut container.node_strs, vec![]);
                            container.node_hrefs = vec![];
                            node_strs.into_iter().for_each(|item| {
                                let doc = Html::parse_fragment(&item);
                                doc.select(&pred).for_each(|node| {
                                    
                                    container.node_strs.push(node.html().to_owned());
                                    let href = node.value().attr("href");
                                    if href.is_some() {
                                        container.node_hrefs.push(href.unwrap().to_owned());
                                    }
                                });

                            });
                            container.doc_or_nodes = DocOrNodes::Nodes;
                        }

                    }
                }
                Ops::ResponseLogic(response_logic)=>{

                    println!("ResponseLogic!");

                    use futures::stream::{self, StreamExt};
                    
                    // let hrefs = container.node_hrefs;
                    

                    // Can't figure out how to remove this block on because 
                    // of Scraper crate dependency that uses Cells :(
                    println!("{:?}", container.node_hrefs);
                    // let (sender, receiver) = channel::<DataFromScraperValue>(5);
                    Box::pin(stream::iter(&container.node_hrefs).enumerate().map(|(i,href)| (i, href, data_sender.clone(), response_logic.clone(), limiter.clone())).for_each_concurrent(
                        /* limit */ 5,
                        |(i, href, data_sender, response_logic, limiter)| async move {
                            println!("here {:?}", href);
                            limit_url_via(&limiter, href).await;
                            // // FOR LIVE RESULTS
                            // let html_str = surf::get(href).recv_string().await.map_err(|_| "Surf error".to_string()).expect("should work");
                            let html_str = format!("<div class='ingredients-prep'><div class='ingredient'>{} test ingredent</div><div class='ingredient'>{} test ingredent</div><div class='prep-steps'><li>step: {}</li></div></div>", i, i, i);
                            
                            handle_response_logic(&response_logic, html_str, data_sender, limiter).await;

                            // async_std::task::yield_now().await;
                        }
                    )).await;



                }
                Store(f)=>{
                    println!("Store!");
                    let res = container.node_strs.iter().flat_map(|item| Html::parse_fragment(&item).root_element().text().map(|item| item.to_string()).collect::<Vec<String>>()).collect::<Vec<String>>();
                    f.call(res).await;
                }
            }

        }
        Ok(())
    })
}







