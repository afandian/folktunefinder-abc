use svg;
use tune_ast_three;
use abc_lexer as l;
use music;
use std::iter::FromIterator;

/// Desired stave width.
/// TODO update this when there are real glyphs.
const STAVE_WIDTH: f32 = 800.0;

// Height of a single note head;
const HEAD_HEIGHT: f32 = 10.0;

// This crops up for aligning to the side of a note.
const HALF_HEAD_HEIGHT: f32 = HEAD_HEIGHT / 2.0;

const HEAD_WIDTH: f32 = HEAD_HEIGHT * 1.25;

const STEM_HEIGHT: f32 = 40.0;

// Vertical padding between each stave.
const STAVE_V_MARGIN: f32 = 20.0;

// Vertical padding between each System.
const SYSTEM_V_MARGIN: f32 = 20.0;

// How many lines (including spaces) in a stave.
const LINES_IN_STAVE: i32 = 9;

// If the scale is below this (i.e. we won't fill the line) then use the natural stave length.
// Prevents non-full-width staves from being forced to be full width.
const MINIMUM_STAVE_SCALE: f32 = 1.8;

pub struct Typesetting {}

impl Typesetting {
    pub fn new() -> Typesetting {
        Typesetting {}
    }
}

/// A Page is made up of a number of boxes which span the page.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Page {
    boxes: Vec<HorizontalBox>,
}

impl Page {
    fn new() -> Page {
        Page { boxes: vec![] }
    }

    fn render(&self, svg: &mut svg::Drawing) {
        let mut y: f32 = 0.0;
        for horizontal_box in self.boxes.iter() {

            horizontal_box.render(svg, y);

            y += horizontal_box.height();
        }
    }
}

/// A box that spans the page.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
enum HorizontalBox {
    // TODO we may have multi-stave systems in future.
    System(Stave),
}

impl HorizontalBox {
    fn height(&self) -> f32 {
        match self {
            &HorizontalBox::System(ref stave) => stave.height() + SYSTEM_V_MARGIN,
        }
    }

    fn render(&self, svg: &mut svg::Drawing, y: f32) {
        match self {
            &HorizontalBox::System(ref stave) => stave.render(svg, y),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum Glyph {
    SingleBar,
    DoubleBar,
    EndBar,
    OpenRepeat,
    CloseRepeat,
    /// Note head of (position-on-stave)
    /// If we're unable to determine the glyph, can be none.
    NoteHead(i32, Option<music::DurationGlyph>),
    Clef(music::Clef),
    BeamBreak,
}

impl Glyph {}

fn draw_tail(svg: &mut svg::Drawing, x: f32, y: f32) {
    svg.line_path(x, y, "M0 0 l2 1 l5 3 l2 14 l-2 5".to_string());
}

/// Entity
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
struct Entity {
    glyph: Glyph,
    x: f32,
}

impl Entity {
    fn new(glyph: Glyph) -> Entity {
        Entity {
            glyph: glyph,
            x: 0.0,
        }
    }

    /// Does this constitute the type of glyph that should be included in front matter?
    fn is_front_matter(&self) -> bool {
        match self.glyph {
            // Normal front matter things.
            // TODO time signature, key signature.
            Glyph::Clef(_) => true,

            // Any kind of barline should be part of front matter.
            // Even weird things that shouldn't be there like close repeat.
            Glyph::SingleBar | Glyph::DoubleBar | Glyph::EndBar | Glyph::OpenRepeat |
            Glyph::CloseRepeat => true,

            // Notehead and friends are definitely out.
            // TODO no catch-all until all glyph types initially settled.
            Glyph::NoteHead(_, _) => false,

            Glyph::BeamBreak => false,
        }
    }

    /// Does this constitute the type of glyph that should be included in end matter?
    fn is_end_matter(&self) -> bool {
        match self.glyph {
            Glyph::SingleBar | Glyph::DoubleBar | Glyph::EndBar | Glyph::OpenRepeat |
            Glyph::CloseRepeat => true,
            _ => false,
        }
    }

    fn width(&self) -> f32 {
        match self.glyph {
            // TODO number of dots will make a difference.
            Glyph::NoteHead(_, glyph) => {
                // Space for the head.
                HEAD_WIDTH * 2.0 +
                    // Space for the dots.
                    match glyph {
                        Some(music::DurationGlyph { shape, dots }) => HEAD_WIDTH * dots as f32,
                        _ => 0.0,
                    }
            }

            // TODO add padding, but in a way that is flush with the end of the line.
            Glyph::SingleBar => 1.0,
            Glyph::DoubleBar => 3.0,
            Glyph::EndBar => 8.0,
            Glyph::OpenRepeat => 20.0,
            Glyph::CloseRepeat => 10.0,

            Glyph::Clef(_) => 50.0,

            // Beam breaks are invisible.
            Glyph::BeamBreak => 0.0,
        }
    }

    /// The absolute coordinate of the end of this Entity's glyph's tail.
    /// Only applies to NoteHeads, and only those that have tails.
    /// TODO currently assumes only up.
    fn tail_anchor(&self) -> Option<(f32, f32)> {
        match self.glyph {
            Glyph::NoteHead(position, duration) => {
                match duration {

                    Some(music::DurationGlyph { shape, dots }) => {
                        let y = (LINES_IN_STAVE - position) as f32 * HEAD_HEIGHT;

                        // TODO switch on direction.
                        Some((self.x + HEAD_WIDTH, y - STEM_HEIGHT))
                    }

                    None => None,
                }
            }

            _ => None,

        }
    }

    fn render(&self, svg: &mut svg::Drawing, x: f32, y: f32) {
        // x in argument is the general offset, i.e. left margin.
        // self.x is the offset within the stave.
        let x = x + self.x;

        match self.glyph {
            Glyph::Clef(clef) => {
                let yy = y + (LINES_IN_STAVE - clef.centre) as f32 * HEAD_HEIGHT;

                svg.rect(x, yy - HEAD_HEIGHT / 2.0, 10.0, HEAD_HEIGHT);
                svg.text(x, yy - HEAD_HEIGHT / 2.0, "clef".to_string());
            }
            Glyph::SingleBar => {
                svg.rect(
                    x,
                    y + HEAD_HEIGHT,
                    1.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );
            }
            Glyph::DoubleBar => {
                svg.rect(
                    x,
                    y + HEAD_HEIGHT,
                    1.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );
                svg.rect(
                    x + 5.0,
                    y + HEAD_HEIGHT,
                    1.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );
            }
            Glyph::OpenRepeat => {
                svg.rect(
                    x,
                    y + HEAD_HEIGHT,
                    1.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );

                let dot_size = 6.0;

                svg.rect_fill(
                    x + 5.0,
                    y + (LINES_IN_STAVE - 3) as f32 * HEAD_HEIGHT,
                    dot_size,
                    dot_size,
                );
                svg.rect_fill(
                    x + 5.0,
                    y + (LINES_IN_STAVE - 5) as f32 * HEAD_HEIGHT,
                    dot_size,
                    dot_size,
                );

            }
            Glyph::CloseRepeat => {
                let dot_size = 6.0;


                svg.rect_fill(
                    x,
                    y + (LINES_IN_STAVE - 3) as f32 * HEAD_HEIGHT,
                    dot_size,
                    dot_size,
                );
                svg.rect_fill(
                    x,
                    y + (LINES_IN_STAVE - 5) as f32 * HEAD_HEIGHT,
                    dot_size,
                    dot_size,
                );

                svg.rect(
                    x + 10.0,
                    y + HEAD_HEIGHT,
                    1.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );
            }
            Glyph::EndBar => {

                svg.rect(
                    x,
                    y + HEAD_HEIGHT,
                    1.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );
                svg.rect_fill(
                    x + 5.0,
                    y + HEAD_HEIGHT,
                    3.0,
                    (LINES_IN_STAVE - 1) as f32 * HEAD_HEIGHT,
                );
            }

            Glyph::NoteHead(position, glyph) => {
                let yy = (y + (LINES_IN_STAVE - position) as f32 * HEAD_HEIGHT);

                match glyph {
                    None => {
                        svg.text(x, yy, "?".to_string());
                    }
                    Some(music::DurationGlyph { shape, dots }) => {

                        // Note head
                        match shape {
                            music::DurationClass::Semibreve |
                            music::DurationClass::Minim => {
                                svg.circle(
                                    x + HEAD_WIDTH / 2.0,
                                    yy + HEAD_WIDTH / 2.0,
                                    HEAD_WIDTH / 2.0,
                                    false,
                                );
                            }

                            music::DurationClass::Crotchet |
                            music::DurationClass::Quaver |
                            music::DurationClass::Semiquaver |
                            music::DurationClass::Demisemiquaver => {
                                svg.circle(
                                    x + HEAD_WIDTH / 2.0,
                                    yy + HEAD_WIDTH / 2.0,
                                    HEAD_WIDTH / 2.0,
                                    true,
                                );
                            }
                        }

                        if let Some((stem_x, stem_y)) = self.tail_anchor() {

                            // Stem
                            match shape {
                                music::DurationClass::Minim |
                                music::DurationClass::Crotchet |
                                music::DurationClass::Quaver |
                                music::DurationClass::Semiquaver |
                                music::DurationClass::Demisemiquaver => {
                                    svg.line(stem_x, stem_y + y, stem_x, yy);
                                }

                                _ => (),
                            }





                            // Tail 1
                            match shape {
                                music::DurationClass::Quaver |
                                music::DurationClass::Semiquaver |
                                music::DurationClass::Demisemiquaver => {
                                    draw_tail(svg, x + HEAD_WIDTH, stem_y + y + HALF_HEAD_HEIGHT);
                                }

                                _ => (),
                            }

                            // Tail 2
                            match shape {
                                music::DurationClass::Semiquaver |
                                music::DurationClass::Demisemiquaver => {
                                    draw_tail(
                                        svg,
                                        x + HEAD_WIDTH,
                                        stem_y + y + HALF_HEAD_HEIGHT + 8.0,
                                    );
                                }

                                _ => (),
                            }

                            // Tail 3
                            match shape {
                                music::DurationClass::Demisemiquaver => {
                                    draw_tail(
                                        svg,
                                        x + HEAD_WIDTH,
                                        stem_y + y + HALF_HEAD_HEIGHT + 16.0,
                                    );
                                }

                                _ => (),
                            }
                        }

                        for dot in 0..dots {
                            svg.circle(
                                x + HEAD_WIDTH + (dot + 2) as f32 * HEAD_HEIGHT * 0.5,
                                yy - HEAD_HEIGHT / 2.0,
                                2.0,
                                true,
                            );

                        }
                    }

                    // svg.rect_fill(x, yy - HEAD_HEIGHT / 2.0, HEAD_HEIGHT * 1.5, HEAD_HEIGHT);
                }
            }

            // As a glyph this doesn't render.
            BeamBreak => (),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
struct Stave {
    entities: Vec<Entity>,
}




impl Stave {
    fn new() -> Stave {
        Stave { entities: vec![] }
    }

    fn height(&self) -> f32 {
        // TODO Include size of stave, ledger lines, etc.
        // Currently this is 5 lines and spaces + one space either side.
        (HEAD_HEIGHT * LINES_IN_STAVE as f32) + STAVE_V_MARGIN
    }

    fn render(&self, svg: &mut svg::Drawing, y: f32) {

        // Split the line in to three regions:
        // 1 - Front matter, including clef, time signature, key signature. This should be typeset
        //     to the same scale on every line.
        // 2 - Justifiable. The rest of the line that should be typeset proportionally.
        // 3 - End matter. The final barline(s), should be right-aligned and typeset at the same
        //     scale.

        // As we have mutable copies around, using offsets is a lot neater than slices!
        let mut justifiable_start_i = 0;
        for i in 0..self.entities.len() {
            justifiable_start_i = i;
            if !self.entities[i].is_front_matter() {
                break;
            }
        }

        let mut justifiable_end_i = self.entities.len();
        for i in (0..self.entities.len()).rev() {
            if !self.entities[i].is_end_matter() {
                break;
            }
            justifiable_end_i = i;
        }

        // Take a mutable copy of the entities. The x values will be shuffled around within the
        // scope of this method but we don't want self.render() to be mutable in the broader scope.
        // We're throwing away the mutated x values after the stave has been typeset.
        let mut entities: Vec<Entity> = Vec::from_iter(self.entities.iter().cloned());

        // Get the natural width of each section so we can work out the scale.
        // The scale for the front and end matter is always 1.
        // The scale for the justifiable section is whatever's left in the middle.
        let front_matter_width: f32 = (&entities[..justifiable_start_i])
            .iter()
            .map(|x| x.width())
            .sum();
        let end_matter_width: f32 = (&entities[justifiable_end_i..])
            .iter()
            .map(|x| x.width())
            .sum();
        let justifiable_width: f32 = (&entities[justifiable_start_i..justifiable_end_i])
            .iter()
            .map(|x| x.width())
            .sum();

        // TODO prevent divide by zero
        let justifiable_scale = (STAVE_WIDTH - (front_matter_width + end_matter_width)) /
            justifiable_width;

        let justifiable_scale = f32::min(STAVE_WIDTH / justifiable_width, MINIMUM_STAVE_SCALE);

        // Stave width doesn't always add up to the ideal STAVE_WIDTH, i.e. a short stave for a
        // short line.
        let stave_width: f32 = (justifiable_width * justifiable_scale) + front_matter_width +
            end_matter_width;

        // Lay out all the entities' x values.
        let mut x = 0.0;
        for i in 0..justifiable_start_i {
            entities[i].x = x;
            x += entities[i].width() * 1.0;
        }

        for i in justifiable_start_i..justifiable_end_i {
            entities[i].x = x;
            x += entities[i].width() * justifiable_scale;
        }

        // Need to wind back from the end so the right-hand edge aligns perfectly.
        x = stave_width - end_matter_width;
        for i in justifiable_end_i..entities.len() {
            entities[i].x = x;
            x += entities[i].width() * 1.0;
        }

        // Now typeset.
        for entity in entities.iter() {
            // The entity has its own offset within the stave. The 0.0 here is page margin.
            // TODO add page margin?
            entity.render(svg, 0.0, y);
        }

        for bar_i in 0..LINES_IN_STAVE {
            let yy = y + (LINES_IN_STAVE - bar_i) as f32 * HEAD_HEIGHT;
            // Alternating lines and spaces.
            if bar_i % 2 == 0 {
                svg.rect(0.0, yy, stave_width, 1.0);
            }
        }

        // Now draw beams.

        // Start (most recent qualifying glyph entity) of this beam group.
        let mut beam_start_i = None;
        // End (most recent qualifying glyph entity) of beam group.
        let mut beam_end_i = None;

        for i in 0..entities.len() {
            let entity = &entities[i];

            match entity.glyph {
                Glyph::NoteHead(_, duration) => {
                    match duration {
                        Some(duration) => {
                            if duration.shape.beams() > 0 {
                                if beam_start_i == None {
                                    beam_start_i = Some(i);
                                } else {
                                    beam_end_i = Some(i);
                                }
                            }
                        }

                        None => (),
                    }
                }

                Glyph::BeamBreak => {
                    if let Some(start_i) = beam_start_i {
                        if let Some(end_i) = beam_end_i {



                            // draw beam
                            // TODO next step is get notehead to show line up or down, then a
                            // method called end_of_stalk to return absolute position.
                            svg.rect_debug(
                                entities[start_i].x + HEAD_WIDTH,
                                y,
                                entities[end_i].x - entities[start_i].x,
                                20.0,
                            );

                        }
                    }

                    beam_start_i = None;
                    beam_end_i = None;
                }

                // Not interested in anything else for drawing beams.
                _ => (),
            }
        }


    }
}

pub fn typeset_from_ast(ast: tune_ast_three::Tune) -> Page {
    let mut page = Page::new();

    let mut current_stave = Stave::new();

    // Always have a key and time signature on the go.
    let mut key_signature = l::T::KeySignature(
        music::PitchClass {
            diatonic_pitch_class: music::DiatonicPitchClass::C,
            accidental: None,
        },
        music::Mode::Major,
    );
    let mut metre = music::Metre(4, 4);

    // TODO We only ever use treble clef at the moment.
    let mut current_clef = music::Clef::treble();

    for token in ast.prelude {
        match token {
            l::T::KeySignature(pitch_class, mode) => {
                key_signature = l::T::KeySignature(pitch_class, mode)
            }
            l::T::Metre(new_metre) => metre = new_metre,
            _ => (),
        }
    }

    current_stave.entities.push(
        Entity::new(Glyph::Clef(current_clef)),
    );
    // TODO add key signature with params
    // TODO add time signature with params.

    for voice in ast.voices {
        for token in voice {
            match token {
                l::T::Newline => {
                    page.boxes.push(HorizontalBox::System(current_stave));
                    current_stave = Stave::new();


                    current_stave.entities.push(
                        Entity::new(Glyph::Clef(current_clef)),
                    );
                    // TODO add key signature with params
                    // TODO add time signature with params.
                }

                // TODO can collapse some sequential things down into single glyphs.
                l::T::SingleBar => current_stave.entities.push(Entity::new(Glyph::SingleBar)),

                l::T::DoubleBar => current_stave.entities.push(Entity::new(Glyph::DoubleBar)),

                l::T::OpenRepeat => current_stave.entities.push(Entity::new(Glyph::OpenRepeat)),

                l::T::CloseRepeat => current_stave.entities.push(Entity::new(Glyph::CloseRepeat)),

                l::T::EndBar => current_stave.entities.push(Entity::new(Glyph::EndBar)),

                l::T::Note(note) => {
                    // TODO extras like accidentals etc.
                    let music::Note(pitch, duration) = note;
                    let clef_interval = current_clef.pitch.interval_to(pitch);

                    let position = (clef_interval.pitch_classes + current_clef.centre) as i32;
                    let glyph = duration.to_glyph();

                    current_stave.entities.push(Entity::new(
                        Glyph::NoteHead(position, glyph),
                    ));
                }

                // Beam break manifests as a zero-width entity. Just like in ABC.
                l::T::BeamBreak => current_stave.entities.push(Entity::new(Glyph::BeamBreak)),

                _ => {
                    // Ignore
                    // TODO don't ignore!
                }
            }
        }
    }

    page.boxes.push(HorizontalBox::System(current_stave));

    page
}

pub fn render_page(page: Page) -> String {
    let mut svg = svg::Drawing::new();

    page.render(&mut svg);

    svg.render()
}
