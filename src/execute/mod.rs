// use futures::stream::{self, StreamExt};
use std::mem::replace;
use scraper::{Selector};

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


use crate::scrape::{StartUrl, ResponseLogic::{self, Parallel}, ElementUrlExtractor, ElementDataExtractor,  Ops::{self, *}};
use crate::{DataFromScraperValue, QuickCrawlerError::{self, *}};


async fn limit_url_via<S: Copy +  Into<String>>(limiter: &Option<Arc<Limiter>>, url: S) -> Result<(), QuickCrawlerError> {

    if limiter.is_some() {
        let base = Url::parse(&url.into()).map_err(|_| QuickCrawlerError::ParseDomainErr)?;
        
        // println!("unwrapping {:?}", url.into().clone());
        let domain = match base.host_str() {
            Some(d) =>d,
            None => return Err(QuickCrawlerError::ParseDomainErr)
        };
        // println!("unwrapped {:?}", domain.clone());
        limiter.as_ref().unwrap().limit(domain).await;
    };
    Ok(())
}



pub async fn execute_deep_scrape<'a>(start_url: &StartUrl, data_sender: Sender<DataFromScraperValue>, limiter: Option<Arc<Limiter>>)-> Result<(), QuickCrawlerError>
{
    let url = match &start_url.url {
        Some(url)=>url,
        None=>{
            return Err(NoUrlInStartUrlErr)
        }
    };

    let req = match &start_url.method {
        // Some(m) if m == "GET" =>"STUB URL REQ GET".to_string(),
        // Some(m) if m == "POST" =>"STUB URL REQ POST".to_string(),
        // // FOR LIVE RESULTS
        Some(m) if m == "GET" =>surf::get(url),
        Some(m) if m == "POST" =>surf::post(url),
        Some(m)=>{
            return Err(InvalidStartUrlMethodErr(m.to_string()))
        }
        None=>{
            return Err(NoStartUrlMethodErr)
        }
    };

    limit_url_via(&limiter, url).await?;
    
    // // FOR LIVE RESULTS
    let html_str = req.recv_string().await.map_err(|_| SurfRequestErr)?;


    let response_logic = match &start_url.response_logic {
        Some(response_logic)=>response_logic.clone(),
        None=>{
            return Err(NoResponseLogicErr)
        }
    };

    handle_response_logic(&response_logic, html_str, data_sender, limiter).await?;

    Ok(())
}




async fn handle_response_logic<'a>(response_logic: &'a ResponseLogic, html_str: String, data_sender: Sender<DataFromScraperValue>, limiter: Option<Arc<Limiter>>) -> Result<(), QuickCrawlerError>

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
            return Err(UnknownResponseLogicErr)
        }
    }
}


struct HtmlContainer {
    html_str: String,
    url_node_strs: Vec<String>,
    data_node_strs: Vec<String>,
    node_urls: Vec<String>,
    data_items: Vec<String>,
}

impl HtmlContainer{
    fn new(html_str: String)-> HtmlContainer {
        HtmlContainer{
            html_str,
            url_node_strs: vec![],
            data_node_strs: vec![],
            node_urls: vec![],
            data_items: vec![],
            // next_: vec![],
        }
    }
}

use futures::future::{BoxFuture, FutureExt};




fn find_node_strs(pred: &Selector, html_str: &str) -> Vec<String> {
    let mut node_strs = Vec::new();
    // let node_strs = replace(&mut container.node_strs, vec![]);
    Html::parse_fragment(html_str).select(pred).for_each(|node| {
        node_strs.push(node.html().replace('\n', "").trim().to_owned());
    });
    return node_strs
}

fn find_urls(ex: &ElementUrlExtractor, node_strs: &Vec<String>) -> Vec<String> {
    let mut urls = Vec::new();
    // let node_strs = replace(&mut container.node_strs, vec![]);
    node_strs.iter().for_each(|node| {
        let node_el = Html::parse_fragment(&node);

        match ex {
            ElementUrlExtractor::Attr(target_attr) => {
                node_el.root_element().children().for_each(|child| {
                    child
                        .value()
                        .as_element()
                        .and_then(|el| el.attr(target_attr))
                        .map(|url| {
                            // println!("url {:?}", url);
                            urls.push(url.to_owned());
                        });
                })
            }
        };
    });

    return urls
}

fn find_data(ex: &ElementDataExtractor, node_strs: &Vec<String>) -> Vec<String> {
    let mut urls = Vec::new();
    // let node_strs = replace(&mut container.node_strs, vec![]);
    node_strs.iter().for_each(|node| {
        let node_el = Html::parse_fragment(&node);

        match ex {
            ElementDataExtractor::Text => {
                // let element_value = Html::parse_fragment(&node).root_element().value();
                urls.extend(node_el.root_element().text().map(|item| item.trim().to_string()).collect::<Vec<String>>());
            }
        };
    });

    return urls
}



fn handle_scrape<'a>(executables: &'a Vec<Box<Ops>>, html_str: String, data_sender: Sender<DataFromScraperValue>, limiter: Option<Arc<Limiter>>)-> BoxFuture<'a, Result<(), String>>
{
    Box::pin(async move {

        let mut container = HtmlContainer::new(html_str.clone());

        for executable in executables.iter() {
            // println!("executable {:?}", i);
            match &**executable {
                UrlSelector(selector_str)=>{
                    let node_strs = find_node_strs(&selector_str, &container.html_str);
                    replace(&mut container.url_node_strs, node_strs);
                }
                DataSelector(selector_str)=>{
                    // println!("Pred!");
                    let node_strs = find_node_strs(&selector_str, &container.html_str);
                    replace(&mut container.data_node_strs, node_strs);
                }
                UrlExtractor(ex)=>{
                    let urls = find_urls(ex, &container.url_node_strs);
                    replace(&mut container.node_urls, urls);
                }
                DataExtractor(ex)=>{
                    let data_items = find_data(ex, &container.data_node_strs);
                    replace(&mut container.data_items, data_items);
                }
                Ops::ResponseLogic(response_logic)=>{

                    // println!("ResponseLogic!");

                    use futures::stream::{self, StreamExt};
                    
                    // let hrefs = container.node_urls;
                    

                    // Can't figure out how to remove this block on because 
                    // of Scraper crate dependency that uses Cells :(
                    // println!("{:?}", container.node_urls);
                    // let (sender, receiver) = channel::<DataFromScraperValue>(5);
                    Box::pin(stream::iter(&container.node_urls).map(|href| (href, data_sender.clone(), response_logic.clone(), limiter.clone())).for_each_concurrent(
                        /* limit */ 5,
                        |(href, data_sender, response_logic, limiter)| async move {
                            // println!("here {:?}", href);
                            limit_url_via(&limiter, href).await;
                            // // FOR LIVE RESULTS
                            let html_str = surf::get(href).recv_string().await.map_err(|_| "Surf error".to_string()).expect("should work");
                            // let html_str = format!("<div class='ingredients-prep'><div class='ingredient'>{} test ingredent</div><div class='ingredient'>{} test ingredent</div><div class='prep-steps'><li>step: {}</li></div></div>", i, i, i);
                            
                            handle_response_logic(&response_logic, html_str, data_sender, limiter).await;

                            // async_std::task::yield_now().await;
                        }
                    )).await;



                }
                Store(f)=>{
                    let res = container.data_items.iter().map(|x| x.to_string()).collect::<Vec<String>>();
                    f.call(res).await;
                }
            }

        }
        Ok(())
    })
}







