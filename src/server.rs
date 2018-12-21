use regex;
use representations;
use search;
use std::env;
use storage;

use std::collections::HashMap;
use url::Url;

use serde_json;

use handlebars::Handlebars;
use std::io::Cursor;
use tiny_http::{Header, Request, Response, Server, StatusCode};

fn api_abc(
    groups: &regex::Captures,
    abc_cache: &mut storage::ABCCache,
) -> Response<Cursor<Vec<u8>>> {
    match groups.get(1) {
        Some(id) => match id.as_str().parse::<u32>() {
            Ok(id) => match abc_cache.get(id) {
                Some(content) => {
                    Response::from_string(content.as_str()).with_status_code(StatusCode(200))
                }
                _ => Response::from_string("Didn't recognise ABC tune id.")
                    .with_status_code(StatusCode(404)),
            },
            _ => Response::from_string("Didn't recognise ABC tune id.")
                .with_status_code(StatusCode(404)),
        },
        _ => {
            Response::from_string("Didn't recognise ABC tune id.").with_status_code(StatusCode(404))
        }
    }
}

fn api_svg(
    groups: &regex::Captures,
    abc_cache: &mut storage::ABCCache,
) -> Response<Cursor<Vec<u8>>> {
    match groups.get(1) {
        Some(id) => {
            match id.as_str().parse::<u32>() {
                Ok(id) => {
                    match abc_cache.get(id) {
                        Some(content) => {
                            // TODO AST already exists?
                            let ast = representations::abc_to_ast(&content);
                            let svg = representations::ast_to_svg(&ast);

                            Response::from_string(svg)
                                .with_header(
                                    Header::from_bytes(&b"Content-Type"[..], &b"image/svg+xml"[..])
                                        .unwrap(),
                                ).with_status_code(StatusCode(200))
                        }
                        _ => Response::from_string("Didn't recognise SVG tune id.")
                            .with_status_code(StatusCode(404)),
                    }
                }
                _ => Response::from_string("Didn't recognise SVG tune id.")
                    .with_status_code(StatusCode(404)),
            }
        }
        _ => {
            Response::from_string("Didn't recognise SVG tune id.").with_status_code(StatusCode(404))
        }
    }
}

// Search.
fn api_search(request: &Request, searcher: &search::SearchEngine) -> Response<Cursor<Vec<u8>>> {
    let base = Url::parse("http://0.0.0.0/").unwrap();

    match Url::join(&base, request.url()) {
        Err(error) => {
            eprint!("Error: {:?}", error);
            Response::from_string("Invalid URL...").with_status_code(StatusCode(400))
        }
        Ok(url) => {
            let mut params: Vec<(String, String)> = url.query_pairs().into_owned().collect();

            match searcher.parse_query(params) {
                Err(message) => Response::from_string(message).with_status_code(StatusCode(400)),
                Ok(query) => {
                    let (num_total_results, num_unique_results, facets, results) =
                        searcher.search(&query);

                    let result_body = serde_json::json!({
                                "query": query,
                                "total": num_total_results,
                                "unique": num_unique_results,
                                "results": results,
                                "facets": facets,
                            });

                    Response::from_string(result_body.to_string())
                        .with_status_code(StatusCode(200))
                        .with_header(
                            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                .unwrap(),
                        )
                }
            }
        }
    }
}

#[derive(Serialize)]
struct HtmlSearchContext {
    query: search::Query,
    num_total_results: usize,
    num_unique_results: usize,
    results: Vec<search::DecoratedResult>,
    facets: Option<HashMap<String, Vec<(String, u32)>>>,
}

// Search.
// TODO add links for navigation:
// - next page link
// - prev page link
// - all vs rollup
// - add filters
// - remove filters
fn html_search(
    request: &Request,
    searcher: &search::SearchEngine,
    handlebars: &Handlebars,
) -> Response<Cursor<Vec<u8>>> {
    let base = Url::parse("http://0.0.0.0/").unwrap();

    match Url::join(&base, request.url()) {
        Err(error) => {
            eprint!("Error: {:?}", error);
            Response::from_string("Invalid URL...").with_status_code(StatusCode(400))
        }
        Ok(url) => {
            let mut params: Vec<(String, String)> = url.query_pairs().into_owned().collect();

            match searcher.parse_query(params) {
                // TODO bit nicer message.
                Err(message) => Response::from_string(message).with_status_code(StatusCode(400)),
                Ok(query) => {
                    let (num_total_results, num_unique_results, facets, results) =
                        searcher.search(&query);

                    let context = HtmlSearchContext {
                        query,
                        num_total_results,
                        num_unique_results,
                        results,
                        facets,
                    };
                    Response::from_string(
                        handlebars
                            .render("search", &context)
                            .unwrap_or("Template error!".to_string())
                            .to_string(),
                    ).with_status_code(StatusCode(200))
                    .with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                    )
                }
            }
        }
    }
}

fn features(_request: &Request, searcher: &search::SearchEngine) -> Response<Cursor<Vec<u8>>> {
    let result = searcher.get_features();

    let body = serde_json::json!(result);

    Response::from_string(body.to_string())
        .with_status_code(StatusCode(200))
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
}

// Return a Handlebars object for templating HTML. This is optional, and by default only the API
// is available.
// If there is a template directory specified in the HTML_TEMPLATES environment variable, load that
// directory.
fn build_template_registry() -> Handlebars {
    let mut handlebars = Handlebars::new();

    let key = "HTML_TEMPLATES";
    if let Ok(path) = env::var(key) {
        match handlebars.register_templates_directory(".html", &path) {
            Err(err) => {
                eprintln!("Error loading template dir {} : {:?}", &path, err);
            }
            _ => (),
        }
    }

    handlebars
}

pub fn main(mut searcher: search::SearchEngine) {
    // API endpoints.
    // There have been folktunefinders before.
    let re_api_abc = regex::Regex::new(r"/api/v3/tunes/(\d+).abc").unwrap();
    let re_api_svg = regex::Regex::new(r"/api/v3/tunes/(\d+).svg").unwrap();
    let re_api_tunes = regex::Regex::new(r"/api/v3/tunes").unwrap();
    let re_api_features = regex::Regex::new(r"/api/v3/features").unwrap();

    // HTML endpoints.
    let re_html_home = regex::Regex::new(r"/").unwrap();
    let re_html_tunes = regex::Regex::new(r"/tunes").unwrap();
    let re_html_tune = regex::Regex::new(r"/tunes/(\d+)").unwrap();

    let key = "HTTP_BIND";
    let bind = match env::var(key) {
        Ok(address) => address,
        Err(_) => {
            eprintln!("Using bind default HTTP_BIND address of : 0.0.0.0:8000");
            "0.0.0.0:8000".to_string()
        }
    };

    // This can optionally run a HTML UI.
    let templates = build_template_registry();

    let server = Server::http(bind).unwrap();

    // Create a local mutable copy.
    // TODO this is less than ideal, as it depends on the cache being constructed in ReadOnly mode.
    // If not, this would double memory usage.
    let mut abc_cache = (*searcher.abcs).clone();

    for request in server.incoming_requests() {
        let response: Response<_> =
        // API
            if let Some(groups) = re_api_abc.captures(request.url()) {
            api_abc(&groups, &mut abc_cache)
        } else if let Some(groups) = re_api_svg.captures(request.url()) {
            api_svg(&groups, &mut abc_cache)
        } else if let Some(_groups) = re_api_tunes.captures(request.url()) {
            api_search(&request, &mut searcher)
        } else if let Some(_groups) = re_api_features.captures(request.url()) {
                features(&request, &mut searcher)
            }
                // HTML routes.

                // TODO HOME
                else if let Some(_) = re_html_tunes.captures(request.url()) {
                    html_search(&request, &mut searcher, &templates)
                }

                // Else.
                else {
            Response::from_string("Didn't recognise that.").with_status_code(StatusCode(404))
        };

        request.respond(response).expect("Can't write response!");
    }
}
