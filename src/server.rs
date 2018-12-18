use abc_lexer;
use regex;
use relations;
use representations;
use search;
use std::collections::HashMap;
use std::env;
use storage;
use tune_ast_three;
use typeset;

use url::Url;

use serde_json;

use search::Query;

use std::io::Cursor;
use tiny_http::{Header, Request, Response, Server, StatusCode};

fn abc(groups: &regex::Captures, abc_cache: &mut storage::ABCCache) -> Response<Cursor<Vec<u8>>> {
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

fn svg(groups: &regex::Captures, abc_cache: &mut storage::ABCCache) -> Response<Cursor<Vec<u8>>> {
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

fn search(request: &Request, searcher: &search::SearchEngine) -> Response<Cursor<Vec<u8>>> {
    let base = Url::parse("http://0.0.0.0/").unwrap();

    match Url::join(&base, request.url()) {
        Err(error) => {
            eprint!("Error: {:?}", error);
            Response::from_string("Invalid URL...").with_status_code(StatusCode(400))
        }
        Ok(url) => {
            let mut params: HashMap<_, _> = url.query_pairs().into_owned().collect();

            match search::parse_query(&params) {
                Err(message) => Response::from_string(message).with_status_code(StatusCode(400)),
                Ok(query) => {
                    let (num_total_results, num_unique_results, results) = searcher.search(&query);

                    let result_body = serde_json::json!({
                        "query": query,
                        "total": num_total_results,
                        "unique": num_unique_results,
                        "results": results,
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

pub fn main(mut searcher: search::SearchEngine) {
    // There have been folktunefinders before.
    let re_abc = regex::Regex::new(r"/v3/tunes/(\d+).abc").unwrap();
    let re_svg = regex::Regex::new(r"/v3/tunes/(\d+).svg").unwrap();
    let re_search = regex::Regex::new(r"/v3/tunes/search").unwrap();

    let key = "HTTP_BIND";
    let bind = match env::var(key) {
        Ok(address) => address,
        Err(_) => {
            eprintln!("Using bind default HTTP_BIND address of : 0.0.0.0:8000");
            "0.0.0.0:8000".to_string()
        }
    };

    let server = Server::http(bind).unwrap();

    // Create a local mutable copy.
    let mut abc_cache = (*searcher.abcs).clone();

    for request in server.incoming_requests() {
        let response: Response<_> = if let Some(groups) = re_abc.captures(request.url()) {
            abc(&groups, &mut abc_cache)
        } else if let Some(groups) = re_svg.captures(request.url()) {
            svg(&groups, &mut abc_cache)
        } else if let Some(groups) = re_search.captures(request.url()) {
            search(&request, &mut searcher)
        } else {
            Response::from_string("Didn't recognise that.").with_status_code(StatusCode(404))
        };

        request.respond(response).expect("Can't write response!");
    }
}
