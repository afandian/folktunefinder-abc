use abc_lexer;
use regex;
use std;
use std::collections::HashMap;
use std::env;
use tune_ast_three;
use typeset;

use tiny_http::{Header, Response, Server, StatusCode};

pub fn main(tunes: &HashMap<u32, String>) {
    let re_abc = regex::Regex::new(r"/abc/(\d+)").unwrap();
    let re_svg = regex::Regex::new(r"/svg/(\d+)").unwrap();

    let key = "HTTP_BIND";
    let bind = match env::var(key) {
        Ok(address) => address,
        Err(e) => {
            eprintln!("Using bind default HTTP_BIND address of : 0.0.0.0:8000");
            "0.0.0.0:8000".to_string()
        }
    };

    let server = Server::http(bind).unwrap();

    for request in server.incoming_requests() {
        let response: Response<_> = if let Some(groups) = re_abc.captures(request.url()) {
            if let Some(id) = groups.get(1) {
                if let Ok(id) = id.as_str().parse::<u32>() {
                    if let Some(content) = tunes.get(&id) {
                        Response::from_string(content.as_str()).with_status_code(StatusCode(200))
                    } else {
                        Response::from_string("Didn't recognise ABC tune id.")
                            .with_status_code(StatusCode(404))
                    }
                } else {
                    Response::from_string("Didn't recognise ABC tune id.")
                        .with_status_code(StatusCode(404))
                }
            } else {
                Response::from_string("Didn't recognise ABC tune id.")
                    .with_status_code(StatusCode(404))
            }
        } else if let Some(groups) = re_svg.captures(request.url()) {
            if let Some(id) = groups.get(1) {
                if let Ok(id) = id.as_str().parse::<u32>() {
                    if let Some(content) = tunes.get(&id) {
                        let chars = content.chars().collect::<Vec<char>>();
                        let ast = tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars));
                        let typeset_page = typeset::typeset_from_ast(ast);
                        let svg_content = typeset::render_page(typeset_page);

                        Response::from_string(svg_content)
                            .with_header(
                                Header::from_bytes(&b"Content-Type"[..], &b"image/svg+xml"[..])
                                    .unwrap(),
                            )
                            .with_status_code(StatusCode(200))
                    } else {
                        Response::from_string("Didn't recognise SVG tune id.")
                            .with_status_code(StatusCode(404))
                    }
                } else {
                    Response::from_string("Didn't recognise SVG tune id.")
                        .with_status_code(StatusCode(404))
                }
            } else {
                Response::from_string("Didn't recognise SVG tune id.")
                    .with_status_code(StatusCode(404))
            }
        } else {
            Response::from_string("Didn't recognise that.").with_status_code(StatusCode(404))
        };

        request.respond(response);
    }
}
