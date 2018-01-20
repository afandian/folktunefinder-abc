//! Visualisation
//! Create graphical representations of tunes.

use svg;
// use tune_ast_two;

pub struct Visualisation {}

impl Visualisation {
    pub fn new() -> Visualisation {
        Visualisation {}
    }
}

// pub fn viz_from_ast(ast: tune_ast_two::Tune) -> String {
//     let mut svg = svg::Drawing::new();

//     const SECTION_HEIGHT: u32 = 20;
//     const SECTION_X: u32 = 20;
//     const BAR_WIDTH: u32 = 100;
//     const SECTION_PAD: u32 = 5;
//     let mut y = 0;
//     for section in ast.sections {
//         let num_bars: usize = section.main.len() +
//             section.n_time_bars.iter().map(|x| x.len()).sum::<usize>();

//         svg.rect(SECTION_X, y, BAR_WIDTH * num_bars as u32, SECTION_HEIGHT);

//         let mut x = 0;
//         for main_bar in section.main.iter() {
//             svg.rect(SECTION_X + x, y, BAR_WIDTH, SECTION_HEIGHT);
//             x += BAR_WIDTH;
//         }

//         let mut n_time = 1;
//         for n_time_bar in section.n_time_bars.iter() {
//             for bar in n_time_bar.iter() {
//                 svg.rect(SECTION_X + x, y, BAR_WIDTH, SECTION_HEIGHT);

//                 svg.text(SECTION_X + x, y + SECTION_HEIGHT, format!("{}", n_time));

//                 n_time += 1;
//                 x += BAR_WIDTH;
//             }
//         }

//         y += SECTION_HEIGHT + SECTION_PAD;
//     }

//     svg.render()
// }
