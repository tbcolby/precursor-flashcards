use std::fmt::Write;

use gam::{Gam, GlyphStyle, Gid};
use gam::menu::*;

use crate::deck::{Card, DeckMeta};

pub fn clear_screen(gam: &Gam, content: Gid, screensize: Point) {
    gam.draw_rectangle(
        content,
        Rectangle::new_with_style(
            Point::new(0, 0),
            screensize,
            DrawStyle {
                fill_color: Some(PixelColor::Light),
                stroke_color: None,
                stroke_width: 0,
            },
        ),
    )
    .expect("can't clear");
}

pub fn draw_deck_list(
    gam: &Gam,
    content: Gid,
    screensize: Point,
    decks: &[DeckMeta],
    cursor: usize,
    scroll_offset: usize,
) {
    clear_screen(gam, content, screensize);

    // Title
    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 8, screensize.x - 12, 36)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "Flashcards").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    // Deck list
    let line_height = 28;
    let list_top = 44;
    let list_bottom = screensize.y - 60;
    let max_visible = ((list_bottom - list_top) / line_height) as usize;

    if decks.is_empty() {
        let mut tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(20, list_top, screensize.x - 20, list_top + 30)),
        );
        tv.style = GlyphStyle::Regular;
        tv.clear_area = true;
        write!(tv.text, "No decks. Press 'i' to import.").unwrap();
        gam.post_textview(&mut tv).expect("can't post empty msg");
    } else {
        let visible_end = (scroll_offset + max_visible).min(decks.len());
        for (i, deck) in decks[scroll_offset..visible_end].iter().enumerate() {
            let y = list_top + (i as isize) * line_height;
            let marker = if scroll_offset + i == cursor { "> " } else { "  " };

            let mut tv = TextView::new(
                content,
                TextBounds::BoundingBox(Rectangle::new_coords(12, y, screensize.x - 12, y + line_height - 2)),
            );
            tv.style = GlyphStyle::Regular;
            tv.clear_area = true;
            write!(tv.text, "{}{} ({} cards)", marker, deck.name, deck.card_count).unwrap();
            gam.post_textview(&mut tv).expect("can't post deck item");
        }
    }

    // Footer
    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(
            12,
            screensize.y - 50,
            screensize.x - 12,
            screensize.y - 10,
        )),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "arrows=select ENTER=open i=import m=manage q=quit").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_card_review(
    gam: &Gam,
    content: Gid,
    screensize: Point,
    deck_name: &str,
    card: &Card,
    card_index: usize,
    total_cards: usize,
    showing_back: bool,
) {
    clear_screen(gam, content, screensize);

    // Card border
    let margin = 12;
    let card_top = 40;
    let card_bottom = screensize.y - 80;
    gam.draw_rounded_rectangle(
        content,
        RoundedRectangle::new(
            Rectangle::new_with_style(
                Point::new(margin, card_top),
                Point::new(screensize.x - margin, card_bottom),
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

    // Side indicator
    let mut side_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(
            margin + 8,
            card_top + 8,
            screensize.x - margin - 8,
            card_top + 30,
        )),
    );
    side_tv.style = GlyphStyle::Small;
    side_tv.clear_area = true;
    if showing_back {
        write!(side_tv.text, "ANSWER").unwrap();
    } else {
        write!(side_tv.text, "QUESTION").unwrap();
    }
    gam.post_textview(&mut side_tv).expect("can't post side");

    // Card content
    let text = if showing_back { &card.back } else { &card.front };
    let mut tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(
            margin + 16,
            card_top + 40,
            screensize.x - margin - 16,
            card_bottom - 16,
        )),
    );
    tv.style = GlyphStyle::Regular;
    tv.clear_area = true;
    write!(tv.text, "{}", text).unwrap();
    gam.post_textview(&mut tv).expect("can't post text");

    // Footer
    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(
            margin,
            card_bottom + 12,
            screensize.x - margin,
            screensize.y - 10,
        )),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(
        nav_tv.text,
        "{} {}/{}  <-/-> nav  SPACE flip  q=back",
        deck_name,
        card_index + 1,
        total_cards
    )
    .unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post nav");

    gam.redraw().expect("can't redraw");
}

pub fn draw_import_wait(gam: &Gam, content: Gid, screensize: Point, port: u16) {
    clear_screen(gam, content, screensize);

    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 20, screensize.x - 12, 48)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "Import Deck").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    let mut tv1 = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 60, screensize.x - 12, 90)),
    );
    tv1.style = GlyphStyle::Regular;
    tv1.clear_area = true;
    write!(tv1.text, "Listening on port {}...", port).unwrap();
    gam.post_textview(&mut tv1).expect("can't post status");

    let mut tv2 = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 100, screensize.x - 12, 180)),
    );
    tv2.style = GlyphStyle::Small;
    tv2.clear_area = true;
    write!(tv2.text, "From your computer run:\n  cat deck.tsv | nc <device-ip> {}", port).unwrap();
    gam.post_textview(&mut tv2).expect("can't post instructions");

    let mut tv3 = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 200, screensize.x - 12, 280)),
    );
    tv3.style = GlyphStyle::Small;
    tv3.clear_area = true;
    write!(tv3.text, "TSV format:\n  #name:Deck Name\n  front<TAB>back\n  front2<TAB>back2").unwrap();
    gam.post_textview(&mut tv3).expect("can't post format");

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 40, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    write!(nav_tv.text, "Waiting for connection... (q=cancel)").unwrap();
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}

pub fn draw_deck_menu(
    gam: &Gam,
    content: Gid,
    screensize: Point,
    deck_name: &str,
    card_count: u32,
    confirm_delete: bool,
) {
    clear_screen(gam, content, screensize);

    let mut title_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 20, screensize.x - 12, 48)),
    );
    title_tv.style = GlyphStyle::Bold;
    title_tv.clear_area = true;
    write!(title_tv.text, "Manage Deck").unwrap();
    gam.post_textview(&mut title_tv).expect("can't post title");

    let mut info_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, 60, screensize.x - 12, 120)),
    );
    info_tv.style = GlyphStyle::Regular;
    info_tv.clear_area = true;
    write!(info_tv.text, "Name: {}\nCards: {}", deck_name, card_count).unwrap();
    gam.post_textview(&mut info_tv).expect("can't post info");

    if confirm_delete {
        let mut confirm_tv = TextView::new(
            content,
            TextBounds::BoundingBox(Rectangle::new_coords(12, 140, screensize.x - 12, 220)),
        );
        confirm_tv.style = GlyphStyle::Regular;
        confirm_tv.clear_area = true;
        write!(confirm_tv.text, "Delete '{}'?\n\n  y = confirm\n  n = cancel", deck_name).unwrap();
        gam.post_textview(&mut confirm_tv).expect("can't post confirm");
    }

    let mut nav_tv = TextView::new(
        content,
        TextBounds::BoundingBox(Rectangle::new_coords(12, screensize.y - 40, screensize.x - 12, screensize.y - 10)),
    );
    nav_tv.style = GlyphStyle::Small;
    nav_tv.clear_area = true;
    if confirm_delete {
        write!(nav_tv.text, "y=delete  n=cancel").unwrap();
    } else {
        write!(nav_tv.text, "d=delete  q=back").unwrap();
    }
    gam.post_textview(&mut nav_tv).expect("can't post footer");

    gam.redraw().expect("can't redraw");
}
