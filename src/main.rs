#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod deck;
mod import;
mod storage;
mod ui;

use num_traits::{FromPrimitive, ToPrimitive};

use gam::UxRegistration;
use gam::menu::*;

use crate::deck::{Card, DeckMeta};
use crate::storage::DeckStorage;

const SERVER_NAME: &str = "_Flashcards_";
const APP_NAME: &str = "Flashcards";

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    Quit,
}

#[derive(Clone)]
enum AppState {
    DeckList,
    CardReview,
    DeckMenu { confirm_delete: bool },
    ImportWait,
}

struct FlashcardApp {
    gam: gam::Gam,
    #[allow(dead_code)]
    token: [u32; 4],
    content: gam::Gid,
    screensize: Point,
    storage: DeckStorage,
    state: AppState,
    // Deck list state
    decks: Vec<DeckMeta>,
    cursor: usize,
    scroll_offset: usize,
    // Card review state
    current_deck_name: String,
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

        let storage = DeckStorage::new();
        storage.ensure_demo_deck();
        let decks = storage.list_decks();

        Self {
            gam,
            token,
            content,
            screensize,
            storage,
            state: AppState::DeckList,
            decks,
            cursor: 0,
            scroll_offset: 0,
            current_deck_name: String::new(),
            cards: Vec::new(),
            current_card: 0,
            showing_back: false,
        }
    }

    fn redraw(&self) {
        match &self.state {
            AppState::DeckList => {
                ui::draw_deck_list(
                    &self.gam,
                    self.content,
                    self.screensize,
                    &self.decks,
                    self.cursor,
                    self.scroll_offset,
                );
            }
            AppState::CardReview => {
                if let Some(card) = self.cards.get(self.current_card) {
                    ui::draw_card_review(
                        &self.gam,
                        self.content,
                        self.screensize,
                        &self.current_deck_name,
                        card,
                        self.current_card,
                        self.cards.len(),
                        self.showing_back,
                    );
                }
            }
            AppState::DeckMenu { confirm_delete } => {
                let card_count = self.decks.get(self.cursor).map(|d| d.card_count).unwrap_or(0);
                ui::draw_deck_menu(
                    &self.gam,
                    self.content,
                    self.screensize,
                    &self.current_deck_name,
                    card_count,
                    *confirm_delete,
                );
            }
            AppState::ImportWait => {
                ui::draw_import_wait(
                    &self.gam,
                    self.content,
                    self.screensize,
                    import::listen_port(),
                );
            }
        }
    }

    fn handle_key(&mut self, key: char) {
        match self.state.clone() {
            AppState::DeckList => self.handle_key_deck_list(key),
            AppState::CardReview => self.handle_key_card_review(key),
            AppState::DeckMenu { confirm_delete } => self.handle_key_deck_menu(key, confirm_delete),
            AppState::ImportWait => {
                // Import state handles 'q' for cancel but the listener blocks,
                // so in practice this is only reached after import completes
                if key == 'q' {
                    self.state = AppState::DeckList;
                    self.refresh_deck_list();
                    self.redraw();
                }
            }
        }
    }

    fn handle_key_deck_list(&mut self, key: char) {
        match key {
            '↓' | 'j' => {
                if !self.decks.is_empty() && self.cursor < self.decks.len() - 1 {
                    self.cursor += 1;
                    self.update_scroll();
                    self.redraw();
                }
            }
            '↑' | 'k' => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.update_scroll();
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                if let Some(deck_meta) = self.decks.get(self.cursor) {
                    let name = deck_meta.name.clone();
                    if let Some(cards) = self.storage.load_deck(&name) {
                        self.current_deck_name = name;
                        self.cards = cards;
                        self.current_card = 0;
                        self.showing_back = false;
                        self.state = AppState::CardReview;
                        self.redraw();
                    }
                }
            }
            'i' => {
                self.state = AppState::ImportWait;
                self.redraw();
                self.do_import();
            }
            'm' => {
                if let Some(deck_meta) = self.decks.get(self.cursor) {
                    self.current_deck_name = deck_meta.name.clone();
                    self.state = AppState::DeckMenu { confirm_delete: false };
                    self.redraw();
                }
            }
            'q' => {
                // Quit is handled by the main loop sending AppOp::Quit
            }
            _ => {}
        }
    }

    fn handle_key_card_review(&mut self, key: char) {
        match key {
            '→' | 'n' => {
                if self.current_card < self.cards.len() - 1 {
                    self.current_card += 1;
                    self.showing_back = false;
                    self.redraw();
                }
            }
            '←' | 'p' => {
                if self.current_card > 0 {
                    self.current_card -= 1;
                    self.showing_back = false;
                    self.redraw();
                }
            }
            ' ' | '\r' | '\n' => {
                self.showing_back = !self.showing_back;
                self.redraw();
            }
            'q' => {
                self.state = AppState::DeckList;
                self.refresh_deck_list();
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_deck_menu(&mut self, key: char, confirm_delete: bool) {
        if confirm_delete {
            match key {
                'y' => {
                    self.storage.delete_deck(&self.current_deck_name);
                    self.state = AppState::DeckList;
                    self.refresh_deck_list();
                    if self.cursor >= self.decks.len() && self.cursor > 0 {
                        self.cursor = self.decks.len() - 1;
                    }
                    self.redraw();
                }
                'n' | 'q' => {
                    self.state = AppState::DeckMenu { confirm_delete: false };
                    self.redraw();
                }
                _ => {}
            }
        } else {
            match key {
                'd' => {
                    self.state = AppState::DeckMenu { confirm_delete: true };
                    self.redraw();
                }
                'q' => {
                    self.state = AppState::DeckList;
                    self.redraw();
                }
                _ => {}
            }
        }
    }

    fn do_import(&mut self) {
        match import::listen_for_import() {
            Some(result) => {
                let name = result.name.unwrap_or_else(|| {
                    format!("Imported {}", self.decks.len() + 1)
                });
                let name = self.unique_deck_name(&name);
                self.storage.save_deck(&name, &result.cards);
                log::info!("Imported deck '{}' with {} cards", name, result.cards.len());
            }
            None => {
                log::info!("Import cancelled or failed");
            }
        }
        self.state = AppState::DeckList;
        self.refresh_deck_list();
        self.redraw();
    }

    fn refresh_deck_list(&mut self) {
        self.decks = self.storage.list_decks();
    }

    fn update_scroll(&mut self) {
        let line_height = 28isize;
        let list_top = 44isize;
        let list_bottom = self.screensize.y - 60;
        let max_visible = ((list_bottom - list_top) / line_height) as usize;

        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + max_visible {
            self.scroll_offset = self.cursor - max_visible + 1;
        }
    }

    fn unique_deck_name(&self, base: &str) -> String {
        let existing: Vec<&str> = self.decks.iter().map(|d| d.name.as_str()).collect();
        if !existing.contains(&base) {
            return base.to_string();
        }
        let mut i = 2;
        loop {
            let candidate = format!("{} ({})", base, i);
            if !existing.contains(&candidate.as_str()) {
                return candidate;
            }
            i += 1;
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
