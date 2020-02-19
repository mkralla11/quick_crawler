# QuickCrawler


  QuickCrawler is a Rust crate that provides a completely async, declarative web crawler 
  with domain-specific request rate-limiting built-in.

# Examples

  Let's say you are trying to crawl a subset of pages for a given domain:

  `https://bike-site.com/search?q=red-bikes`

  and a regular GET request will return:

  ```html
    <html>
        <body>
            <div>
                <a class="bike-item" href="https://bike-site.com/red-bike-1">
                    cool red bike 1
                </a><a class="bike-item" href="https://bike-site.com/red-bike-2">
                    cool red bike 2
                </a>
                <a class="bike-item" href="https://bike-site.com/red-bike-3">
                    cool red bike 3
                </a>
                <div>
                    <a class="bike-other-item" href="https://bike-site.com/other-red-bike-4">
                        other cool red bike 4
                    </a>
                </div>
            </div>
        </body>
    </html>
  ```


  and when navigating to *links 1 through 3* on that page, EACH PAGE returns:

  ```html
  <html>
      <body>
          <div class='awesome-bike'>
              <div class='bike-info'>
                  The best bike ever.
              </div>
              <ul class='bike-specs'>
                  <li>
                      Super fast.
                  </li>
                  <li>
                      Jumps high.
                  </li>
              </ul>
          </div>
      </body>
  </html>
  ```


  and when navigating to *the last link* on that page, it returns:

  ```html
  <html>
      <body>
          <div class='other-bike'>
              <div class='other-bike-info'>
                  The best bike ever.
              </div>
              <ul class='other-bike-specs'>
                  <li>
                      Super slow.
                  </li>
                  <li>
                      Doesn't jump.
                  </li>
              </ul>
          </div>
      </body>
  </html>
  ```


  QuickCrawler declaratively helps you crawl, and scrape data from each of the given pages with ease:


  ```rust, no_run
  use quick_crawler::{
      QuickCrawler, 
      QuickCrawlerBuilder,
      limiter::Limiter, 
      scrape::{
          ResponseLogic::Parallel, 
          StartUrl, 
          Scrape, 
          ElementUrlExtractor, 
          ElementDataExtractor
      }
  };


  fn main() {
      let mut builder = QuickCrawlerBuilder::new();


      let start_urls = vec![
          StartUrl::new()
              .url("https://bike-site.com/search?q=red-bikes")
              .method("GET")
              .response_logic(Parallel(vec![
                  // All Scrapers below will be provided the html page response body
                  Scrape::new()
                      .find_elements_with_urls(".bike-item")
                      .extract_urls_from_elements(ElementUrlExtractor::Attr("href".to_string()))
                      // now setup the logic to execute on each of the return html pages
                      .response_logic(Parallel(vec![
                          Scrape::new()
                              .find_elements_with_data(".awesome-bike .bike-info")
                              .extract_data_from_elements(ElementDataExtractor::Text)
                              .store(|vec: Vec<String>| async move {
                                  println!("store bike info in DB: {:?}", vec);
                              }),
                          Scrape::new()
                              .find_elements_with_data(".bike-specs li")
                              .extract_data_from_elements(ElementDataExtractor::Text)
                              .store(|vec: Vec<String>| async move {
                                  println!("store bike specs in DB: {:?}", vec);
                              }),
                      ])),
                  Scrape::new()
                      .find_elements_with_urls(".bike-other-item")
                      .extract_urls_from_elements(ElementUrlExtractor::Attr("href".to_string()))
                      .response_logic(Parallel(vec![
                          Scrape::new()
                              .find_elements_with_data(".other-bike .other-bike-info")
                              .extract_data_from_elements(ElementDataExtractor::Text)
                              .store(|vec: Vec<String>| async move {
                                  println!("store other bike info in DB: {:?}", vec);
                              }),
                          Scrape::new()
                              .find_elements_with_data(".other-bike-specs li")
                              .extract_data_from_elements(ElementDataExtractor::Text)
                              .store(|vec: Vec<String>| async move {
                                  println!("store other bike specs in DB: {:?}", vec);
                              }),
                      ]))  
              ])
          )
          // more StartUrl::new 's if you feel ambitious
      ] ;

      // It's smart to use a limiter - for now automatically set to 3 request per second per domain.
      // This will soon be configurable.

      let limiter = Limiter::new();

      builder
          .with_start_urls(
              start_urls
          )
          .with_limiter(
              limiter
          );
      let crawler = builder.finish().map_err(|_| "Builder could not finish").expect("no error");
      
      // QuickCrawler is async, so choose your favorite executor.
      // (Tested and working for both async-std and tokio)
      let res = async_std::task::block_on(async {
          crawler.process().await
      });

  }

  ```



# Contribute

clone the repo.

To run tests:

  cargo watch -x check -x 'test  -- --nocapture'

See the `src/lib.rs` test for example usage.

# Thank You For Using!

If you use this crate and you found it helpful to your project, please star it!


# License

MIT
