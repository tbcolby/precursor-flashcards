# Precursor Flashcards

A flashcard app for the [Precursor](https://www.crowdsupply.com/sutajio-kosagi/precursor) hardware platform running [Xous OS](https://github.com/betrusted-io/xous-core). Supports multiple decks stored in the PDDB, with network import for loading your own cards from a computer.

## Screenshots

### Deck List & Management

| Deck List | Deck Menu |
|-----------|-----------|
| ![Deck List](screenshots/deck_list.png) | ![Deck Menu](screenshots/deck_menu.png) |

### Card Review

| Question | Answer |
|----------|--------|
| ![Question](screenshots/question.png) | ![Answer](screenshots/answer.png) |

### Import

| Import Screen |
|---------------|
| ![Import](screenshots/import_wait.png) |

*Running on Renode emulator*

## Features

- Multiple decks with persistent storage via PDDB
- Built-in demo deck on first launch
- **Import** decks from a computer over the network (TCP port 7878)
- **Export** decks back to a computer (TCP port 7879)
- Flip between question and answer with Space/Enter
- Navigate between cards with arrow keys or n/p
- **Shuffle** deck order for randomized review
- Deck management with delete support
- Scrollable deck list for large collections
- Up to 500 cards per deck
- Duplicate deck name handling on import

## Controls

### Universal Keys

| Key | Action |
|-----|--------|
| F1 | Menu / Help |
| F4 | Exit / Back |
| F2 | Flip Card (in review) |
| F3 | Next Card (in review) |

### Deck List

| Key | Action |
|-----|--------|
| Up/Down arrows or `j`/`k` | Move cursor |
| Enter | Open selected deck |
| `i` | Import a new deck |
| `m` | Manage selected deck |
| `q` | Quit app |

### Card Review

| Key | Action |
|-----|--------|
| Space / Enter | Flip card (question/answer) |
| Right arrow / `n` | Next card |
| Left arrow / `p` | Previous card |
| `s` | Shuffle deck |
| `q` | Return to deck list |

### Deck Menu

| Key | Action |
|-----|--------|
| `e` | Export deck (TCP) |
| `d` | Delete deck |
| `y` / `n` | Confirm/cancel deletion |
| `q` | Return to deck list |

## Loading Your Own Cards

The app uses a TCP push mechanism so you don't have to type URLs on the tiny Precursor keyboard. You author a simple TSV (tab-separated) file on your computer and push it to the device over the network.

### 1. Create a deck file

Create a `.tsv` file with a `#name:` header and tab-separated front/back pairs:

```
#name:Spanish Vocab
hola	hello
gato	cat
perro	dog
casa	house
libro	book
```

- First column = front of card (question)
- Second column = back of card (answer)
- Lines starting with `#` (other than `#name:`) are comments and ignored
- Empty lines are skipped
- Maximum 500 cards per deck

### 2. Start the import listener

On the Precursor, press `i` from the deck list screen. The device will show "Listening on port 7878..." and wait for a connection.

### 3. Send the file from your computer

```bash
cat my_deck.tsv | nc <device-ip> 7878
```

Replace `<device-ip>` with your Precursor's IP address (visible in the network settings).

The deck will be parsed, saved to the PDDB, and appear in your deck list immediately.

### Tips

- If you omit the `#name:` header, the deck will be auto-named "Imported 1", "Imported 2", etc.
- If a deck with the same name already exists, a suffix like "(2)" is added
- Maximum import size is 64KB per transfer
- The listener accepts one connection then returns to the deck list

### Example deck files

**Programming trivia:**
```
#name:Programming
What year was C created?	1972
Who created Python?	Guido van Rossum
What does HTML stand for?	HyperText Markup Language
```

**Study flashcards:**
```
#name:Biology 101
What is the powerhouse of the cell?	Mitochondria
What is DNA's sugar?	Deoxyribose
How many chromosomes do humans have?	46
```

## Exporting Decks

You can export any deck back to your computer using TCP. This allows backup and transfer of decks between devices.

### 1. Start the export listener

On the Precursor, go to the deck menu (press `m` on the deck list) and press `e` for export. The device will listen on port 7879.

### 2. Receive the file on your computer

```bash
nc <device-ip> 7879 > my_deck.tsv
```

The exported file uses the same TSV format as import, with the `#name:` header preserved.

## Integration with xous-core

This app is designed to be placed in the `apps/` directory of the [xous-core](https://github.com/betrusted-io/xous-core) repository.

### Steps

1. Copy the app directory:
   ```bash
   cp -r precursor-flashcards/ xous-core/apps/flashcards/
   ```

2. Add to workspace `Cargo.toml` members list:
   ```toml
   members = [
       # ... existing members ...
       "apps/flashcards",
   ]
   ```

3. Add to `apps/manifest.json`:
   ```json
   "flashcards": {
       "context_name": "Flashcards",
       "menu_name": {
           "appmenu.flashcards": {
               "en": "Flashcards",
               "en-tts": "Flashcards"
           }
       }
   }
   ```

4. Build the Renode image:
   ```bash
   cargo xtask renode-image flashcards
   ```

## Architecture

The app follows standard Xous patterns:

- **State machine**: `DeckList` / `CardReview` / `DeckMenu` / `ImportWait` states with key dispatch
- **PDDB storage**: Dictionary `flashcards` with index key and per-deck binary-serialized card data
- **GAM registration**: Registers as `UxType::Chat` for canvas access
- **Raw keys**: Receives keyboard input via `rawkeys_id` scalar messages
- **TCP import**: Uses `std::net::TcpListener` on port 7878 (routed through Xous net service)
- **Focus handling**: Stops redrawing when backgrounded

### Source files

| File | Purpose |
|------|---------|
| `src/main.rs` | App state machine, key handling, main loop |
| `src/deck.rs` | Card/DeckMeta structs, binary serialization |
| `src/storage.rs` | PDDB operations (list, load, save, delete) |
| `src/import.rs` | TSV parser, TCP import/export |
| `src/ui.rs` | Screen drawing functions |

## Toolchain Requirements

- Rust stable (tested with 1.88.0)
- Custom Xous sysroot for `riscv32imac-unknown-xous-elf`
- See [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit) for complete setup instructions

## Development

This app was developed using the methodology described in [xous-dev-toolkit](https://github.com/tbcolby/xous-dev-toolkit) — an LLM-assisted approach to Precursor app development on macOS ARM64.

## Changelog

### v0.2.0

- **Shuffle** — Randomize card order with 's' key or menu option (Fisher-Yates algorithm)
- **Export** — Send decks back to computer via TCP (port 7879)
- **Fixed quit** — 'q' and F4 now properly exit the app from deck list
- **Fixed cursor** — Cursor position correctly adjusted after deleting a deck

### v0.1.0

- Initial release with import, multi-deck support, and PDDB storage

## Author

Made by Tyler Colby — [Colby's Data Movers, LLC](https://colbysdatamovers.com)

Contact: tyler@colbysdatamovers.com | [GitHub Issues](https://github.com/tbcolby/precursor-flashcards/issues)

## License

Licensed under the same terms as xous-core (Apache-2.0/MIT).
