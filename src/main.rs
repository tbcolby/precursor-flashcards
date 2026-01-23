#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

use num_traits::{FromPrimitive, ToPrimitive};
use std::fmt::Write;

use blitstr2::GlyphStyle;
use gam::{TextBounds, TextView, UxRegistration};
use ux_api::minigfx::*;

const SERVER_NAME: &str = "_Flashcards_";
const APP_NAME: &str = "Flashcards"; // Must match context_name in manifest.json

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    Quit,
}

struct Card {
    front: &'static str,
    back: &'static str,
}

struct FlashcardApp {
    gam: gam::Gam,
    token: [u32; 4],
    content: gam::Gid,
    screensize: Point,
    cards: Vec<Card>,
    current_card: usize,
    showing_back: bool,
}

impl FlashcardApp {
    fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = gam::Gam::new(xns).expect("can't connect to GAM");

        let token = gam
            .register_ux(UxRegistration {
                app_name: String::from(APP_NAME),
                ux_type: gam::UxType::Chat,
                predictor: None,
                listener: sid.to_array(),
                redraw_id: AppOp::Redraw.to_u32().unwrap(),
                gotinput_id: None,
                audioframe_id: None,
                rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
                focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
            })
            .expect("couldn't register UX")
            .unwrap();

        let content = gam.request_content_canvas(token).expect("couldn't get canvas");
        let screensize = gam.get_canvas_bounds(content).expect("couldn't get dimensions");

        let cards = vec![
            Card {
                front: "What is Xous?",
                back: "A microkernel OS for the Precursor, using message-passing IPC between servers.",
            },
            Card {
                front: "What is the Precursor display?",
                back: "336x536 pixels, 1-bit (black and white only). No grayscale or color.",
            },
            Card {
                front: "What is the PDDB?",
                back: "Plausibly Deniable Database. Encrypted key-value storage organized as basis > dictionary > key.",
            },
            Card {
                front: "How do apps draw to screen?",
                back: "Through the GAM (Graphics Abstraction Manager) service, which manages canvases and trust levels.",
            },
            Card {
                front: "What CPU does Precursor use?",
                back: "100MHz VexRISC-V RV32IMAC. Single core, no FPU. Think carefully about computation.",
            },
            Card {
                front: "How does IPC work in Xous?",
                back: "Message passing. Scalar messages (4 usizes) or memory messages (buffer transfer). No shared memory.",
            },
            Card {
                front: "What is a Server ID (SID)?",
                back: "A unique address for a process's message queue. Obtained by registering a name with xous-names.",
            },
            Card {
                front: "How do apps handle input?",
                back: "Register rawkeys_id with GAM. Keys arrive as up to 4 chars packed in scalar message parameters.",
            },
        ];

        Self {
            gam,
            token,
            content,
            screensize,
            cards,
            current_card: 0,
            showing_back: false,
        }
    }

    fn redraw(&self) {
        let card = &self.cards[self.current_card];

        // Clear screen
        self.gam
            .draw_rectangle(
                self.content,
                Rectangle::new_with_style(
                    Point::new(0, 0),
                    self.screensize,
                    DrawStyle {
                        fill_color: Some(PixelColor::Light),
                        stroke_color: None,
                        stroke_width: 0,
                    },
                ),
            )
            .expect("can't clear");

        // Draw card border
        let margin = 12;
        let card_top = 40;
        let card_bottom = self.screensize.y - 80;
        self.gam
            .draw_rounded_rectangle(
                self.content,
                RoundedRectangle::new(
                    Rectangle::new_with_style(
                        Point::new(margin, card_top),
                        Point::new(self.screensize.x - margin, card_bottom),
                        DrawStyle {
                            fill_color: None,
                            stroke_color: Some(PixelColor::Dark),
                            stroke_width: 2,
                        },
                    ),
                    6,
                ),
            )
            .expect("can't draw card border");

        // Draw side indicator (front/back)
        let mut side_tv = TextView::new(
            self.content,
            TextBounds::BoundingBox(Rectangle::new_coords(
                margin + 8,
                card_top + 8,
                self.screensize.x - margin - 8,
                card_top + 30,
            )),
        );
        side_tv.style = GlyphStyle::Small;
        side_tv.clear_area = true;
        if self.showing_back {
            write!(side_tv.text, "ANSWER").unwrap();
        } else {
            write!(side_tv.text, "QUESTION").unwrap();
        }
        self.gam.post_textview(&mut side_tv).expect("can't post side");

        // Draw card content
        let text = if self.showing_back { card.back } else { card.front };
        let mut tv = TextView::new(
            self.content,
            TextBounds::BoundingBox(Rectangle::new_coords(
                margin + 16,
                card_top + 40,
                self.screensize.x - margin - 16,
                card_bottom - 16,
            )),
        );
        tv.style = GlyphStyle::Regular;
        tv.clear_area = true;
        write!(tv.text, "{}", text).unwrap();
        self.gam.post_textview(&mut tv).expect("can't post text");

        // Draw navigation footer
        let mut nav_tv = TextView::new(
            self.content,
            TextBounds::BoundingBox(Rectangle::new_coords(
                margin,
                card_bottom + 12,
                self.screensize.x - margin,
                self.screensize.y - 10,
            )),
        );
        nav_tv.style = GlyphStyle::Small;
        nav_tv.clear_area = true;
        write!(
            nav_tv.text,
            "Card {}/{}  |  <-/-> navigate  SPACE flip",
            self.current_card + 1,
            self.cards.len()
        )
        .unwrap();
        self.gam.post_textview(&mut nav_tv).expect("can't post nav");

        self.gam.redraw().expect("can't redraw");
    }

    fn handle_key(&mut self, key: char) {
        match key {
            // Right arrow or 'n' - next card
            '→' | 'n' => {
                if self.current_card < self.cards.len() - 1 {
                    self.current_card += 1;
                    self.showing_back = false;
                    self.redraw();
                }
            }
            // Left arrow or 'p' - previous card
            '←' | 'p' => {
                if self.current_card > 0 {
                    self.current_card -= 1;
                    self.showing_back = false;
                    self.redraw();
                }
            }
            // Space or Enter - flip card
            ' ' | '\r' | '\n' => {
                self.showing_back = !self.showing_back;
                self.redraw();
            }
            _ => {}
        }
    }
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("Flashcards PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    let mut app = FlashcardApp::new(&xns, sid);
    let mut allow_redraw = true;

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                if allow_redraw {
                    app.redraw();
                }
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                let keys = [
                    core::char::from_u32(k1 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k2 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k3 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k4 as u32).unwrap_or('\u{0000}'),
                ];
                for &key in keys.iter() {
                    if key != '\u{0000}' {
                        app.handle_key(key);
                    }
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, new_state_code, _, _, _, {
                let new_state = gam::FocusState::convert_focus_change(new_state_code);
                match new_state {
                    gam::FocusState::Background => {
                        allow_redraw = false;
                    }
                    gam::FocusState::Foreground => {
                        allow_redraw = true;
                        app.redraw();
                    }
                }
            }),
            Some(AppOp::Quit) => break,
            _ => log::error!("unknown opcode: {:?}", msg),
        }
    }

    xns.unregister_server(sid).unwrap();
    xous::destroy_server(sid).unwrap();
    xous::terminate_process(0)
}
