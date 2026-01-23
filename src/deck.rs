#[derive(Clone)]
pub struct Card {
    pub front: String,
    pub back: String,
}

pub struct DeckMeta {
    pub name: String,
    pub card_count: u32,
}

/// Serialize a deck's cards to binary format:
/// [u32: card_count] { [u16: front_len][front_utf8] [u16: back_len][back_utf8] } ...
pub fn serialize_cards(cards: &[Card]) -> Vec<u8> {
    let mut buf = Vec::new();
    let count = cards.len() as u32;
    buf.extend_from_slice(&count.to_le_bytes());
    for card in cards {
        let front_bytes = card.front.as_bytes();
        let back_bytes = card.back.as_bytes();
        buf.extend_from_slice(&(front_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(front_bytes);
        buf.extend_from_slice(&(back_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(back_bytes);
    }
    buf
}

/// Deserialize cards from binary format.
pub fn deserialize_cards(data: &[u8]) -> Option<Vec<Card>> {
    if data.len() < 4 {
        return None;
    }
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let mut cards = Vec::with_capacity(count);
    let mut pos = 4;
    for _ in 0..count {
        if pos + 2 > data.len() {
            return None;
        }
        let front_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        if pos + front_len > data.len() {
            return None;
        }
        let front = String::from_utf8(data[pos..pos + front_len].to_vec()).ok()?;
        pos += front_len;

        if pos + 2 > data.len() {
            return None;
        }
        let back_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        if pos + back_len > data.len() {
            return None;
        }
        let back = String::from_utf8(data[pos..pos + back_len].to_vec()).ok()?;
        pos += back_len;

        cards.push(Card { front, back });
    }
    Some(cards)
}
