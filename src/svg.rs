//! SVG building tools

use std::fmt::Write;

enum Entity {
    Rect(u32, u32, u32, u32),
    Text(u32, u32, String),
}

pub struct Drawing {
    width: u32,
    height: u32,
    entities: Vec<Entity>,
}

impl Drawing {
    pub fn new() -> Drawing {
        // Starts empty, resize to accommodate.
        Drawing {
            width: 0,
            height: 0,
            entities: vec![],
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

        for entity in self.entities.iter() {
            match entity {
                &Entity::Rect(x, y, w, h) => {
                    write!(
                        &mut buf,
                        "<rect x='{}' y='{}' width='{}' height='{}' \
                                      style='fill:none;stroke:black;stroke-width:2' />",
                        x,
                        y,
                        w,
                        h
                    ).unwrap();
                }

                &Entity::Text(x, y, ref text) => {
                    // TOOD ESCAPE
                    write!(&mut buf, "<text x='{}' y='{}' >{}</text>", x, y, text).unwrap();
                }
            }
        }

        write!(&mut buf, "</svg>").unwrap();

        buf
    }

    fn ensure(&mut self, x: u32, y: u32) {
        self.width = u32::max(x, self.width);
        self.height = u32::max(y, self.height);
    }

    pub fn rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.ensure(x, y);
        self.ensure(x + w, y + h);

        self.entities.push(Entity::Rect(x, y, w, h));
    }

    pub fn text(&mut self, x: u32, y: u32, text: String) {
        self.ensure(x, y);

        self.entities.push(Entity::Text(x, y, text));
    }
}
