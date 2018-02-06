use application;
use regex;
use std::env;
use std;

use tiny_http::{Server, Response, StatusCode, Header};

pub fn main(application: &application::Application) {
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
            if let Some(abc_id) = groups.get(1) {
                if let Ok(abc_id) = abc_id.as_str().parse::<u32>() {
                    if let Some(content) = application.get_abc(abc_id) {
                        Response::from_string(content).with_status_code(StatusCode(200))
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
            if let Some(svg_id) = groups.get(1) {
                if let Ok(svg_id) = svg_id.as_str().parse::<u32>() {
                    if let Some(content) = application.get_svg(svg_id) {
                        Response::from_string(content)
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
