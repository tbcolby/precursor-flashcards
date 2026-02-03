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

// F-key character codes from Xous keyboard service
const KEY_F1: char = '\u{0011}';
const KEY_F2: char = '\u{0012}';
const KEY_F3: char = '\u{0013}';
const KEY_F4: char = '\u{0014}';

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
    // Menu overlay state
    menu_visible: bool,
    menu_cursor: usize,
    help_visible: bool,
    should_quit: bool,
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
            menu_visible: false,
            menu_cursor: 0,
            help_visible: false,
            should_quit: false,
        }
    }

    fn redraw(&self) {
        if self.help_visible {
            ui::draw_help(&self.gam, self.content, self.screensize, self.help_text());
            return;
        }
        if self.menu_visible {
            ui::draw_menu(
                &self.gam,
                self.content,
                self.screensize,
                self.menu_items(),
                self.menu_cursor,
            );
            return;
        }

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
        // F-keys always processed first
        match key {
            KEY_F1 => { self.toggle_menu(); return; }
            KEY_F4 => { self.handle_f4(); return; }
            KEY_F2 => { self.handle_f2(); return; }
            KEY_F3 => { self.handle_f3(); return; }
            _ => {}
        }

        // If help screen is showing, any key dismisses it
        if self.help_visible {
            self.help_visible = false;
            self.redraw();
            return;
        }

        // If menu is open, handle menu navigation only
        if self.menu_visible {
            match key {
                '↑' | 'k' => {
                    if self.menu_cursor > 0 {
                        self.menu_cursor -= 1;
                        self.redraw();
                    }
                }
                '↓' | 'j' => {
                    let items = self.menu_items();
                    if self.menu_cursor + 1 < items.len() {
                        self.menu_cursor += 1;
                        self.redraw();
                    }
                }
                '\r' | '\n' => {
                    self.menu_select_item();
                }
                _ => {}
            }
            return;
        }

        // Normal mode-specific key handling
        match self.state.clone() {
            AppState::DeckList => self.handle_key_deck_list(key),
            AppState::CardReview => self.handle_key_card_review(key),
            AppState::DeckMenu { confirm_delete } => self.handle_key_deck_menu(key, confirm_delete),
            AppState::ImportWait => {
                if key == 'q' {
                    self.state = AppState::DeckList;
                    self.refresh_deck_list();
                    self.redraw();
                }
            }
        }
    }

    fn menu_items(&self) -> &'static [&'static str] {
        match &self.state {
            AppState::DeckList => &["Help", "Import Deck (TCP)", "Manage Deck"],
            AppState::CardReview => &["Help", "Flip Card", "Next Card", "Shuffle", "Back to List"],
            AppState::DeckMenu { .. } => &["Help", "Export (TCP)", "Delete Deck", "Back to List"],
            AppState::ImportWait => &["Help"],
        }
    }

    fn toggle_menu(&mut self) {
        if self.help_visible {
            self.help_visible = false;
            self.redraw();
            return;
        }
        self.menu_visible = !self.menu_visible;
        self.menu_cursor = 0;
        self.redraw();
    }

    fn menu_select_item(&mut self) {
        let state = self.state.clone();
        self.menu_visible = false;

        match &state {
            AppState::DeckList => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        self.state = AppState::ImportWait;
                        self.redraw();
                        self.do_import();
                        return;
                    }
                    2 => {
                        if let Some(deck_meta) = self.decks.get(self.cursor) {
                            self.current_deck_name = deck_meta.name.clone();
                            self.state = AppState::DeckMenu { confirm_delete: false };
                        }
                    }
                    _ => {}
                }
            }
            AppState::CardReview => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => { self.showing_back = !self.showing_back; }
                    2 => {
                        if self.current_card + 1 < self.cards.len() {
                            self.current_card += 1;
                            self.showing_back = false;
                        }
                    }
                    3 => {
                        // Shuffle cards
                        self.shuffle_cards();
                        self.current_card = 0;
                        self.showing_back = false;
                    }
                    4 => {
                        self.state = AppState::DeckList;
                        self.refresh_deck_list();
                    }
                    _ => {}
                }
            }
            AppState::DeckMenu { .. } => {
                match self.menu_cursor {
                    0 => { self.help_visible = true; }
                    1 => {
                        // Export via TCP
                        if let Some(cards) = self.storage.load_deck(&self.current_deck_name) {
                            match import::export_via_tcp(&self.current_deck_name, &cards) {
                                Ok(bytes) => log::info!("Exported {} bytes", bytes),
                                Err(e) => log::error!("Export failed: {}", e),
                            }
                        }
                    }
                    2 => {
                        // Trigger delete confirmation
                        self.state = AppState::DeckMenu { confirm_delete: true };
                    }
                    3 => {
                        self.state = AppState::DeckList;
                        self.refresh_deck_list();
                    }
                    _ => {}
                }
            }
            AppState::ImportWait => {
                if self.menu_cursor == 0 {
                    self.help_visible = true;
                }
            }
        }
        self.redraw();
    }

    fn handle_f2(&mut self) {
        if self.help_visible { self.help_visible = false; self.redraw(); return; }
        if self.menu_visible { self.menu_visible = false; }
        // F2 = Flip card (during review)
        if let AppState::CardReview = &self.state {
            self.showing_back = !self.showing_back;
        }
        self.redraw();
    }

    fn handle_f3(&mut self) {
        if self.help_visible { self.help_visible = false; self.redraw(); return; }
        if self.menu_visible { self.menu_visible = false; }
        // F3 = Next card (during review)
        if let AppState::CardReview = &self.state {
            if self.current_card + 1 < self.cards.len() {
                self.current_card += 1;
                self.showing_back = false;
            }
        }
        self.redraw();
    }

    fn handle_f4(&mut self) {
        // F4 closes help/menu first
        if self.help_visible {
            self.help_visible = false;
            self.redraw();
            return;
        }
        if self.menu_visible {
            self.menu_visible = false;
            self.redraw();
            return;
        }
        // F4 = Back: card review→deck list→quit
        match &self.state {
            AppState::CardReview => {
                self.state = AppState::DeckList;
                self.refresh_deck_list();
                self.redraw();
            }
            AppState::DeckMenu { .. } => {
                self.state = AppState::DeckList;
                self.redraw();
            }
            AppState::DeckList => {
                // At top level - quit the app
                self.should_quit = true;
            }
            AppState::ImportWait => {
                // Can't interrupt blocking import
            }
        }
    }

    fn help_text(&self) -> &'static str {
        match &self.state {
            AppState::DeckList => {
                "FLASHCARDS HELP\n\n\
                 F1     Menu\n\
                 F4     Quit\n\n\
                 Up/Dn  Move cursor\n\
                 Enter  Open deck\n\
                 i      Import deck\n\
                 m      Manage deck\n\
                 q      Quit"
            }
            AppState::CardReview => {
                "CARD REVIEW HELP\n\n\
                 F1     Menu\n\
                 F2     Flip card\n\
                 F3     Next card\n\
                 F4     Back to list\n\n\
                 Space  Flip card\n\
                 <-/->  Prev/Next\n\
                 n/p    Next/Prev\n\
                 s      Shuffle deck\n\
                 q      Back to list"
            }
            AppState::DeckMenu { .. } => {
                "DECK MENU HELP\n\n\
                 F1     Menu\n\
                 F4     Back to list\n\n\
                 e      Export (TCP 7879)\n\
                 d      Delete deck\n\
                 y/n    Confirm/cancel\n\
                 q      Back to list"
            }
            AppState::ImportWait => {
                "IMPORT HELP\n\n\
                 Waiting for TCP\n\
                 connection on\n\
                 port 7878.\n\n\
                 Send TSV file\n\
                 from computer."
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
                // Signal quit - this will be processed by returning true from handle_key
                // The main loop will then terminate
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_key_card_review(&mut self, key: char) {
        match key {
            '→' | 'n' => {
                if self.current_card + 1 < self.cards.len() {
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
            's' => {
                self.shuffle_cards();
                self.current_card = 0;
                self.showing_back = false;
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
                    // Adjust cursor if it's now beyond the list
                    if self.decks.is_empty() {
                        self.cursor = 0;
                        self.scroll_offset = 0;
                    } else if self.cursor >= self.decks.len() {
                        self.cursor = self.decks.len() - 1;
                    }
                    self.update_scroll();
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
                'e' => {
                    // Export via TCP
                    if let Some(cards) = self.storage.load_deck(&self.current_deck_name) {
                        match import::export_via_tcp(&self.current_deck_name, &cards) {
                            Ok(bytes) => log::info!("Exported {} bytes", bytes),
                            Err(e) => log::error!("Export failed: {}", e),
                        }
                    }
                    self.redraw();
                }
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

    fn shuffle_cards(&mut self) {
        // Simple Fisher-Yates shuffle using system time as seed
        let seed = xous::create_server_id().unwrap().0[0] as usize;
        let len = self.cards.len();
        if len <= 1 {
            return;
        }
        let mut rng = seed;
        for i in (1..len).rev() {
            // Simple LCG-based random
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let j = rng % (i + 1);
            self.cards.swap(i, j);
        }
        log::info!("Shuffled {} cards", len);
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
                // Check if quit was requested
                if app.should_quit {
                    break;
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
