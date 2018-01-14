//! SVG building tools

use std::fmt::Write;



pub struct Drawing {
    width: i32,
    height: i32,
}

impl Drawing {
    pub fn new() -> Drawing {
        // Starts empty, resize to accommodate.
        Drawing {
            width: 0,
            height: 0,
        }
    }

    pub fn render(&self) -> String {
        let mut buf = String::new();

        write!(
            &mut buf,
            "<svg version='1.1' baseProfile='full' width='{}' height='{}' \
                          xmlns='http://www.w3.org/2000/svg'>",
            self.width,
            self.height
        ).unwrap();


        write!(&mut buf, "</svg>");

        buf
    }
}
