use std::io::{Read, Write, Seek, SeekFrom};

use crate::deck::{Card, DeckMeta, deserialize_cards, serialize_cards};

const DICT_NAME: &str = "flashcards";
const INDEX_KEY: &str = "_index";

pub struct DeckStorage {
    pddb: pddb::Pddb,
}

impl DeckStorage {
    pub fn new() -> Self {
        let pddb = pddb::Pddb::new();
        pddb.try_mount();
        Self { pddb }
    }

    /// List all deck names from the index.
    pub fn list_decks(&self) -> Vec<DeckMeta> {
        let names = self.read_index();
        let mut metas = Vec::new();
        for name in names {
            let card_count = self.get_card_count(&name);
            metas.push(DeckMeta { name, card_count });
        }
        metas
    }

    /// Load a deck's cards by name.
    pub fn load_deck(&self, name: &str) -> Option<Vec<Card>> {
        let key_name = format!("deck.{}", name);
        match self.pddb.get(DICT_NAME, &key_name, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut data = Vec::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_end(&mut data).is_ok() && !data.is_empty() {
                    deserialize_cards(&data)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Save a deck (cards + index entry).
    pub fn save_deck(&self, name: &str, cards: &[Card]) {
        let key_name = format!("deck.{}", name);
        let data = serialize_cards(cards);

        // Write card data
        match self.pddb.get(DICT_NAME, &key_name, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                if key.write_all(&data).is_ok() {
                    self.pddb.sync().ok();
                }
            }
            Err(e) => log::error!("Failed to save deck '{}': {:?}", name, e),
        }

        // Update index
        let mut names = self.read_index();
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
            self.write_index(&names);
        }
    }

    /// Delete a deck (cards + index entry).
    pub fn delete_deck(&self, name: &str) {
        let key_name = format!("deck.{}", name);
        self.pddb.delete_key(DICT_NAME, &key_name, None).ok();

        let mut names = self.read_index();
        names.retain(|n| n != name);
        self.write_index(&names);
        self.pddb.sync().ok();
    }

    /// Check if the index exists (for first-run detection).
    pub fn has_index(&self) -> bool {
        self.pddb.get(DICT_NAME, INDEX_KEY, None, false, false, None, None::<fn()>).is_ok()
    }

    /// Initialize with demo deck if no index exists.
    pub fn ensure_demo_deck(&self) {
        if self.has_index() {
            return;
        }
        let demo_cards = vec![
            Card {
                front: "What is Xous?".to_string(),
                back: "A microkernel OS for the Precursor, using message-passing IPC between servers.".to_string(),
            },
            Card {
                front: "What is the Precursor display?".to_string(),
                back: "336x536 pixels, 1-bit (black and white only). No grayscale or color.".to_string(),
            },
            Card {
                front: "What is the PDDB?".to_string(),
                back: "Plausibly Deniable Database. Encrypted key-value storage organized as basis > dictionary > key.".to_string(),
            },
            Card {
                front: "How do apps draw to screen?".to_string(),
                back: "Through the GAM (Graphics Abstraction Manager) service, which manages canvases and trust levels.".to_string(),
            },
            Card {
                front: "What CPU does Precursor use?".to_string(),
                back: "100MHz VexRISC-V RV32IMAC. Single core, no FPU.".to_string(),
            },
            Card {
                front: "How does IPC work in Xous?".to_string(),
                back: "Message passing. Scalar messages (4 usizes) or memory messages (buffer transfer). No shared memory.".to_string(),
            },
            Card {
                front: "What is a Server ID (SID)?".to_string(),
                back: "A unique address for a process's message queue. Obtained by registering a name with xous-names.".to_string(),
            },
            Card {
                front: "How do apps handle input?".to_string(),
                back: "Register rawkeys_id with GAM. Keys arrive as up to 4 chars packed in scalar message parameters.".to_string(),
            },
        ];
        self.save_deck("Xous Basics", &demo_cards);
    }

    fn read_index(&self) -> Vec<String> {
        match self.pddb.get(DICT_NAME, INDEX_KEY, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut data = String::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_string(&mut data).is_ok() {
                    data.lines()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_string())
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }

    fn write_index(&self, names: &[String]) {
        let data = names.join("\n");
        match self.pddb.get(DICT_NAME, INDEX_KEY, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(data.as_bytes()).ok();
                self.pddb.sync().ok();
            }
            Err(e) => log::error!("Failed to write index: {:?}", e),
        }
    }

    fn get_card_count(&self, name: &str) -> u32 {
        let key_name = format!("deck.{}", name);
        match self.pddb.get(DICT_NAME, &key_name, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut buf = [0u8; 4];
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_exact(&mut buf).is_ok() {
                    u32::from_le_bytes(buf)
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }
}
