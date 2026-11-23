//! Unit tests for the co-op overlays (roadmap T10-014).

use proptest::prelude::*;

use crate::coop_overlays::*;
use crate::draw::{Color, DrawList, Quad, TextAlign};
use crate::geometry::{CANVAS_H, CANVAS_W, Rect, Vec2};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn texts(list: &DrawList) -> Vec<String> {
    list.texts.iter().map(|t| t.text.to_string()).collect()
}

fn has_text(list: &DrawList, text: &str) -> bool {
    list.texts.iter().any(|t| t.text == text)
}

fn quads_of(list: &DrawList, color: Color) -> Vec<Rect> {
    list.quads
        .iter()
        .filter(|q| q.color == color)
        .map(|q| q.rect)
        .collect()
}

fn inside_canvas(r: Rect) -> bool {
    r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= CANVAS_W && r.y + r.h <= CANVAS_H
}

fn inside(r: Rect, safe: Rect) -> bool {
    r.x >= safe.x - 1e-3
        && r.y >= safe.y - 1e-3
        && r.x + r.w <= safe.x + safe.w + 1e-3
        && r.y + r.h <= safe.y + safe.h + 1e-3
}

// ---- clip_text ------------------------------------------------------------------------------------------------------------

#[test]
fn clip_text_keeps_a_short_text_and_cuts_a_long_one_with_an_ellipsis() {
    assert_eq!(clip_text("abc", 3), "abc");
    assert_eq!(clip_text("abcd", 3), "ab\u{2026}");
    assert_eq!(clip_text("abcd", 4), "abcd");
    assert_eq!(clip_text("abcd", 1), "\u{2026}");
    assert_eq!(clip_text("abcd", 0), "");
    assert_eq!(clip_text("", 0), "");
    assert_eq!(clip_text("", 5), "");
}

#[test]
fn clip_text_counts_characters_not_bytes_and_drops_control_characters() {
    let word = "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442}";
    assert_eq!(clip_text(word, 6), word);
    assert_eq!(clip_text(word, 4), "\u{43f}\u{440}\u{438}\u{2026}");
    assert_eq!(clip_text("a\nb\tc\u{7}d", 10), "abcd");
    assert_eq!(clip_text("   padded  ", 10), "padded");
}

#[test]
fn estimate_text_width_grows_with_length_and_size() {
    assert!(close(estimate_text_width(10, 20.0), 110.0));
    assert!(close(estimate_text_width(0, 20.0), 0.0));
}

// ---- scoreboard -----------------------------------------------------------------------------------------------------------

fn entry(id: u32, name: &str, kills: u32, deaths: u32, score: i64) -> ScoreEntry {
    ScoreEntry {
        id,
        name: name.to_owned(),
        kills,
        deaths,
        score,
        ping_ms: Some(30),
        local: false,
        talking: false,
    }
}

fn ids(sb: &Scoreboard) -> Vec<u32> {
    sb.rows().iter().map(|e| e.id).collect()
}

#[test]
fn the_scoreboard_is_hidden_until_the_key_is_held_and_hidden_again_on_release() {
    let mut sb = Scoreboard::new();
    sb.set_entries(&[entry(1, "Mia", 3, 1, 300)]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
    assert!(!sb.is_visible());
    sb.set_held(true);
    assert!(sb.is_visible());
    sb.draw_into(&mut list);
    assert!(has_text(&list, "Mia"));
    sb.set_held(false);
    list.clear();
    sb.draw_into(&mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
}

#[test]
fn drawing_appends_to_what_is_already_in_the_list() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let mut list = DrawList::default();
    list.quads.push(Quad {
        rect: Rect::default(),
        color: Color([1.0, 0.0, 1.0, 1.0]),
    });
    sb.draw_into(&mut list);
    assert_eq!(list.quads[0].color, Color([1.0, 0.0, 1.0, 1.0]));
    assert!(list.quads.len() > 1);
}

#[test]
fn an_empty_scoreboard_draws_a_panel_and_the_headings_without_rows() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    sb.set_entries(&[]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert_eq!(list.quads.len(), 1);
    assert!(has_text(&list, "Scoreboard (0)"));
    for heading in ["Player", "Kills", "Deaths", "Score", "Ping"] {
        assert!(has_text(&list, heading), "{heading}");
    }
    assert_eq!(sb.hidden(), 0);
}

#[test]
fn one_player_is_one_row() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    sb.set_entries(&[entry(4, "Solo", 7, 2, 700)]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert!(has_text(&list, "Scoreboard (1)"));
    for t in ["Solo", "7", "2", "700", "30"] {
        assert!(has_text(&list, t), "{t}");
    }
    assert_eq!(list.quads.len(), 2, "the panel and one row");
}

#[test]
fn players_are_sorted_by_score_then_kills_then_fewer_deaths_then_id() {
    let mut sb = Scoreboard::new();
    sb.set_entries(&[
        entry(5, "e", 1, 0, 100),
        entry(1, "a", 1, 0, 300),
        entry(2, "b", 5, 9, 200),
        entry(3, "c", 5, 1, 200),
        entry(4, "d", 5, 1, 200),
        entry(6, "f", 0, 0, -50),
    ]);
    assert_eq!(ids(&sb), vec![1, 3, 4, 2, 5, 6]);
}

#[test]
fn kills_break_a_tie_in_score_and_the_sort_ignores_the_arrival_order() {
    let players = [
        entry(1, "a", 2, 2, 10),
        entry(2, "b", 2, 2, 10),
        entry(3, "c", 3, 0, 10),
        entry(4, "d", 0, 0, 20),
    ];
    let mut first = Scoreboard::new();
    first.set_entries(&players);
    let mut reversed = players.clone();
    reversed.reverse();
    let mut second = Scoreboard::new();
    second.set_entries(&reversed);
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(ids(&first), vec![4, 3, 1, 2]);
}

#[test]
fn a_negative_score_sorts_below_zero_and_prints_with_its_sign() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    sb.set_entries(&[entry(1, "a", 0, 3, -20), entry(2, "b", 0, 0, 0)]);
    assert_eq!(ids(&sb), vec![2, 1]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert!(has_text(&list, "-20"));
}

#[test]
fn the_local_player_row_is_highlighted_and_only_that_one() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let mut me = entry(2, "Me", 1, 1, 100);
    me.local = true;
    sb.set_entries(&[entry(1, "Other", 9, 0, 900), me, entry(3, "Third", 0, 0, 0)]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    let rows = quads_of(&list, LOCAL_ROW);
    assert_eq!(rows.len(), 1);
    // Me is second by score: the highlight is the second stripe.
    let stripes: Vec<Rect> = list.quads.iter().skip(1).map(|q| q.rect).collect();
    assert!(close(rows[0].y, stripes[1].y));
    assert!(rows[0].y > stripes[0].y);
}

#[test]
fn sixteen_players_fit_without_a_more_line() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let players: Vec<ScoreEntry> = (0..16)
        .map(|i| entry(i, &format!("p{i}"), i, 0, i as i64))
        .collect();
    sb.set_entries(&players);
    assert_eq!(sb.rows().len(), 16);
    assert_eq!(sb.hidden(), 0);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert!(!texts(&list).iter().any(|t| t.contains("more")));
    assert!(
        list.quads.iter().all(|q| inside_canvas(q.rect)),
        "the panel fits the canvas"
    );
    assert_eq!(sb.rows()[0].id, 15, "the best player first");
}

#[test]
fn more_than_sixteen_players_are_cut_with_a_count_and_the_local_player_is_kept() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let mut players: Vec<ScoreEntry> = (0..20)
        .map(|i| entry(i, &format!("p{i}"), 0, 0, 100 - i as i64))
        .collect();
    players[18].local = true;
    sb.set_entries(&players);
    assert_eq!(sb.rows().len(), MAX_SCOREBOARD_ROWS);
    assert_eq!(sb.hidden(), 4);
    assert_eq!(sb.rows()[15].id, 18, "the local player takes the last row");
    assert_eq!(ids(&sb)[..15], (0..15).collect::<Vec<u32>>()[..]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert!(has_text(&list, "+4 more"));
    assert!(has_text(&list, "Scoreboard (20)"));
    assert!(list.quads.iter().all(|q| inside_canvas(q.rect)));
}

#[test]
fn a_local_player_who_already_fits_changes_nothing_when_there_are_too_many() {
    let mut sb = Scoreboard::new();
    let mut players: Vec<ScoreEntry> = (0..20)
        .map(|i| entry(i, "x", 0, 0, 100 - i as i64))
        .collect();
    players[3].local = true;
    sb.set_entries(&players);
    assert_eq!(ids(&sb), (0..16).collect::<Vec<u32>>());
    assert_eq!(sb.hidden(), 4);
}

#[test]
fn the_local_player_in_the_seventeenth_place_takes_the_last_row() {
    let mut sb = Scoreboard::new();
    let mut players: Vec<ScoreEntry> = (0..17)
        .map(|i| entry(i, "x", 0, 0, 100 - i as i64))
        .collect();
    players[16].local = true;
    sb.set_entries(&players);
    assert_eq!((sb.rows().len(), sb.hidden()), (16, 1));
    assert_eq!(sb.rows()[15].id, 16);
    assert_eq!(sb.rows()[14].id, 14);
}

#[test]
fn exactly_the_seventeenth_player_is_cut_and_a_new_list_replaces_the_old() {
    let mut sb = Scoreboard::new();
    let players: Vec<ScoreEntry> = (0..17)
        .map(|i| entry(i, "x", 0, 0, 100 - i as i64))
        .collect();
    sb.set_entries(&players);
    assert_eq!((sb.rows().len(), sb.hidden()), (16, 1));
    sb.set_entries(&players[..3]);
    assert_eq!((sb.rows().len(), sb.hidden()), (3, 0));
}

#[test]
fn names_are_cleaned_cut_and_never_empty() {
    let mut sb = Scoreboard::new();
    sb.set_entries(&[
        entry(1, &"W".repeat(60), 0, 0, 3),
        entry(2, "  \u{7} ", 0, 0, 2),
        entry(3, "Ti\nm", 0, 0, 1),
    ]);
    let names: Vec<&str> = sb.rows().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names[0].chars().count(), MAX_NAME_CHARS);
    assert!(names[0].ends_with('\u{2026}'));
    assert_eq!(names[1], "Player 2");
    assert_eq!(names[2], "Tim");
}

#[test]
fn the_ping_figure_is_green_yellow_or_red_and_a_dash_when_unknown() {
    assert_eq!(ping_quality(0), PingQuality::Good);
    assert_eq!(ping_quality(79), PingQuality::Good);
    assert_eq!(ping_quality(80), PingQuality::Fair);
    assert_eq!(ping_quality(149), PingQuality::Fair);
    assert_eq!(ping_quality(150), PingQuality::Poor);
    assert_eq!(ping_quality(u32::MAX), PingQuality::Poor);

    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let mut unknown = entry(1, "bot", 0, 0, 3);
    unknown.ping_ms = None;
    let mut slow = entry(2, "slow", 0, 0, 2);
    slow.ping_ms = Some(4000);
    let mut fair = entry(3, "fair", 0, 0, 1);
    fair.ping_ms = Some(100);
    sb.set_entries(&[unknown, slow, fair, entry(4, "good", 0, 0, 0)]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    let cell = |text: &str| {
        list.texts
            .iter()
            .find(|t| t.text == text && t.align == TextAlign::Center && t.pos.x > 1300.0)
            .unwrap_or_else(|| panic!("no ping cell {text}"))
            .color
    };
    let (dash, slow_c, fair_c, good_c) = (cell("-"), cell("999"), cell("100"), cell("30"));
    assert!(slow_c.0[0] > 0.9 && slow_c.0[1] < 0.5, "red");
    assert!(
        fair_c.0[0] > 0.9 && fair_c.0[1] > 0.8 && fair_c.0[2] < 0.5,
        "yellow"
    );
    assert!(good_c.0[1] > 0.9 && good_c.0[0] < 0.5, "green");
    assert!(dash.0[0] < 0.7 && dash.0[1] < 0.7, "muted");
}

#[test]
fn a_talking_player_gets_a_green_dot() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let mut talker = entry(1, "Talker", 0, 0, 5);
    talker.talking = true;
    sb.set_entries(&[talker, entry(2, "Quiet", 0, 0, 1)]);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert_eq!(quads_of(&list, VOICE_ON).len(), 1);
}

#[test]
fn rows_are_evenly_spaced_and_columns_are_apart() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let players: Vec<ScoreEntry> = (0..6).map(|i| entry(i, "n", 0, 0, i as i64)).collect();
    sb.set_entries(&players);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    let stripes: Vec<Rect> = list.quads.iter().skip(1).map(|q| q.rect).collect();
    assert_eq!(stripes.len(), 6);
    for pair in stripes.windows(2) {
        assert!(pair[1].y >= pair[0].y + pair[0].h, "stripes do not overlap");
        assert!(close(pair[1].y - pair[0].y, 44.0));
    }
    let heads: Vec<f32> = ["Kills", "Deaths", "Score", "Ping"]
        .iter()
        .map(|h| list.texts.iter().find(|t| t.text == *h).unwrap().pos.x)
        .collect();
    assert!(heads.windows(2).all(|w| w[1] - w[0] >= 100.0));
}

// ---- chat -----------------------------------------------------------------------------------------------------------------

fn chat() -> ChatBox {
    ChatBox::new(ChatConfig::default())
}

fn row_texts(chat: &ChatBox) -> Vec<String> {
    chat.visible_rows().iter().map(|r| r.0.to_owned()).collect()
}

#[test]
fn lines_are_shown_with_the_sender_and_a_prefix_for_the_team() {
    let mut c = chat();
    c.push(ChatKind::Player, "Mia", "hello");
    c.push(ChatKind::Team, "Tom", "on my way");
    c.push(ChatKind::System, "ignored", "Ann joined");
    assert_eq!(
        row_texts(&c),
        vec!["Mia: hello", "[Team] Tom: on my way", "Ann joined"]
    );
    let kinds: Vec<ChatKind> = c.visible_rows().iter().map(|r| r.1).collect();
    assert_eq!(
        kinds,
        vec![ChatKind::Player, ChatKind::Team, ChatKind::System]
    );
}

#[test]
fn a_message_without_a_sender_has_no_colon() {
    let mut c = chat();
    c.push(ChatKind::Player, "  ", "anonymous");
    c.push(ChatKind::Team, "", "psst");
    assert_eq!(row_texts(&c), vec!["anonymous", "[Team] psst"]);
}

#[test]
fn empty_and_control_only_messages_are_ignored() {
    let mut c = chat();
    c.push(ChatKind::Player, "Mia", "");
    c.push(ChatKind::Player, "Mia", "   ");
    c.push(ChatKind::Player, "Mia", "\n\t\u{7}");
    assert!(c.is_empty());
    assert_eq!(c.len(), 0);
    c.push(ChatKind::Player, "Mia", "x");
    assert!(!c.is_empty());
    assert_eq!(c.len(), 1);
}

#[test]
fn a_chat_that_keeps_no_history_stores_nothing() {
    let mut c = ChatBox::new(ChatConfig {
        max_stored: 0,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "hello");
    assert!(c.is_empty());
}

#[test]
fn the_history_never_grows_past_its_limit_and_drops_the_oldest() {
    let mut c = ChatBox::new(ChatConfig {
        max_stored: 5,
        max_rows: 100,
        ..ChatConfig::default()
    });
    for i in 0..1000 {
        c.push(ChatKind::System, "", &format!("line {i}"));
    }
    assert_eq!(c.len(), 5);
    assert_eq!(
        row_texts(&c),
        vec!["line 995", "line 996", "line 997", "line 998", "line 999"]
    );
}

#[test]
fn a_received_message_is_cut_at_the_message_limit() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 1000,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", &"x".repeat(500));
    let rows = row_texts(&c);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].chars().count(), 200);
    assert!(rows[0].ends_with('\u{2026}'));
}

#[test]
fn a_long_sender_name_is_cut_like_a_name() {
    let mut c = chat();
    c.push(ChatKind::Player, &"N".repeat(50), "hi");
    let row = &row_texts(&c)[0];
    assert_eq!(
        *row,
        format!("{}\u{2026}: hi", "N".repeat(MAX_NAME_CHARS - 1))
    );
}

#[test]
fn lines_stay_fully_visible_then_fade_then_go() {
    let mut c = ChatBox::new(ChatConfig {
        visible_ticks: 100,
        fade_ticks: 40,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "hi");
    for _ in 0..100 {
        c.update();
    }
    assert_eq!(
        c.visible_rows()[0].2,
        1.0,
        "still fully visible at the limit"
    );
    c.update(); // 101
    assert!(close(c.visible_rows()[0].2, 1.0 - 1.0 / 40.0));
    for _ in 0..19 {
        c.update(); // 120
    }
    assert!(close(c.visible_rows()[0].2, 0.5));
    for _ in 0..19 {
        c.update(); // 139
    }
    assert!(close(c.visible_rows()[0].2, 1.0 / 40.0));
    c.update(); // 140
    assert!(c.visible_rows().is_empty());
}

#[test]
fn a_zero_fade_hides_a_line_the_tick_after_its_time() {
    let mut c = ChatBox::new(ChatConfig {
        visible_ticks: 3,
        fade_ticks: 0,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "hi");
    for _ in 0..3 {
        c.update();
    }
    assert_eq!(c.visible_rows().len(), 1);
    c.update();
    assert!(c.visible_rows().is_empty());
}

#[test]
fn an_old_line_does_not_hide_a_newer_one() {
    let mut c = ChatBox::new(ChatConfig {
        visible_ticks: 10,
        fade_ticks: 10,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "old");
    for _ in 0..30 {
        c.update();
    }
    c.push(ChatKind::System, "", "new");
    assert_eq!(row_texts(&c), vec!["new"]);
}

#[test]
fn an_open_input_shows_old_lines_at_full_strength_and_closing_lets_them_fade_again() {
    let mut c = chat();
    c.push(ChatKind::System, "", "old");
    for _ in 0..5000 {
        c.update();
    }
    assert!(c.visible_rows().is_empty());
    c.open(false);
    assert_eq!(c.visible_rows().len(), 1);
    assert_eq!(c.visible_rows()[0].2, 1.0);
    c.handle_key(ChatKey::Escape);
    assert!(c.visible_rows().is_empty());
}

#[test]
fn at_most_max_rows_rows_are_shown_newest_last() {
    let mut c = chat();
    for i in 0..30 {
        c.push(ChatKind::System, "", &format!("m{i}"));
    }
    let rows = row_texts(&c);
    assert_eq!(rows.len(), 8);
    assert_eq!(rows[0], "m22");
    assert_eq!(rows[7], "m29");
    let mut none = ChatBox::new(ChatConfig {
        max_rows: 0,
        ..ChatConfig::default()
    });
    none.push(ChatKind::System, "", "x");
    assert!(none.visible_rows().is_empty());
}

#[test]
fn a_long_line_wraps_at_a_space_and_breaks_a_word_that_does_not_fit() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 10,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "the quick brown fox");
    assert_eq!(row_texts(&c), vec!["the quick", "brown fox"]);
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 5,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "abcdefghijkl");
    assert_eq!(row_texts(&c), vec!["abcde", "fghij", "kl"]);
}

#[test]
fn a_row_ends_exactly_when_the_next_character_is_a_space() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 5,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "ab cd ef");
    assert_eq!(row_texts(&c), vec!["ab cd", "ef"]);
}

#[test]
fn wrapping_keeps_every_word_and_no_row_is_too_long() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 12,
        max_rows: 100,
        ..ChatConfig::default()
    });
    let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda";
    c.push(ChatKind::System, "", text);
    let rows = row_texts(&c);
    assert!(
        rows.iter()
            .all(|r| r.chars().count() <= 12 && !r.is_empty()),
        "{rows:?}"
    );
    assert_eq!(rows.join(" "), text);
}

#[test]
fn a_zero_wrap_width_does_not_hang_or_panic() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 0,
        max_rows: 100,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "abc");
    assert_eq!(row_texts(&c), vec!["a", "b", "c"]);
    c.open(false);
    c.handle_text("xyz");
    let _ = c.input_line();
}

#[test]
fn when_the_rows_do_not_all_fit_the_tail_of_a_wrapped_line_is_shown() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 5,
        max_rows: 2,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "abcdefghijkl");
    assert_eq!(row_texts(&c), vec!["fghij", "kl"]);
}

#[test]
fn typing_is_ignored_and_keys_give_nothing_while_the_input_is_closed() {
    let mut c = chat();
    c.handle_text("hello");
    assert_eq!(c.input(), "");
    assert_eq!(c.handle_key(ChatKey::Enter), None);
    assert_eq!(c.handle_key(ChatKey::Escape), None);
    assert!(!c.is_open());
}

#[test]
fn typing_edits_at_the_cursor() {
    let mut c = chat();
    c.open(false);
    c.handle_text("helo");
    assert_eq!((c.input(), c.cursor()), ("helo", 4));
    c.handle_key(ChatKey::Left);
    c.handle_key(ChatKey::Left);
    c.handle_text("l");
    assert_eq!((c.input(), c.cursor()), ("hello", 3));
    c.handle_key(ChatKey::Home);
    assert_eq!(c.cursor(), 0);
    c.handle_key(ChatKey::Delete);
    assert_eq!(c.input(), "ello");
    c.handle_key(ChatKey::Backspace);
    assert_eq!(
        (c.input(), c.cursor()),
        ("ello", 0),
        "backspace at the start does nothing"
    );
    c.handle_key(ChatKey::End);
    assert_eq!(c.cursor(), 4);
    c.handle_key(ChatKey::Delete);
    assert_eq!(c.input(), "ello", "delete at the end does nothing");
    c.handle_key(ChatKey::Right);
    assert_eq!(c.cursor(), 4, "right at the end stays");
    c.handle_key(ChatKey::Backspace);
    assert_eq!((c.input(), c.cursor()), ("ell", 3));
    for _ in 0..4 {
        c.handle_key(ChatKey::Left);
    }
    assert_eq!(c.cursor(), 0, "left at the start stays");
    c.handle_key(ChatKey::Right);
    assert_eq!(c.cursor(), 1);
}

#[test]
fn typing_handles_multibyte_characters_and_leaves_out_control_characters() {
    let mut c = chat();
    c.open(false);
    c.handle_text("\u{43f}\u{440}\n\u{438}\t");
    assert_eq!(c.input().chars().count(), 3);
    c.handle_key(ChatKey::Left);
    c.handle_key(ChatKey::Backspace);
    assert_eq!(c.input(), "\u{43f}\u{438}");
    assert_eq!(c.cursor(), 1);
}

#[test]
fn the_draft_stops_at_the_message_limit() {
    let mut c = ChatBox::new(ChatConfig {
        max_message_chars: 10,
        ..ChatConfig::default()
    });
    c.open(false);
    c.handle_text(&"a".repeat(25));
    assert_eq!(c.input().chars().count(), 10);
    c.handle_key(ChatKey::Backspace);
    c.handle_text("bb");
    assert_eq!(c.input(), format!("{}b", "a".repeat(9)));
}

#[test]
fn enter_sends_the_trimmed_message_to_everybody_and_closes() {
    let mut c = chat();
    c.open(false);
    c.handle_text("  hi there  ");
    assert_eq!(
        c.handle_key(ChatKey::Enter),
        Some(ChatCommand::Send {
            text: "hi there".into(),
            team: false
        })
    );
    assert!(!c.is_open());
    assert_eq!(c.input(), "");
}

#[test]
fn tab_switches_to_team_and_the_message_goes_to_the_team() {
    let mut c = chat();
    c.open(false);
    assert!(!c.is_team());
    c.handle_key(ChatKey::Tab);
    assert!(c.is_team());
    c.handle_text("push left");
    assert_eq!(
        c.handle_key(ChatKey::Enter),
        Some(ChatCommand::Send {
            text: "push left".into(),
            team: true
        })
    );
    c.open(true);
    assert!(c.is_team());
    c.handle_key(ChatKey::Tab);
    assert!(!c.is_team());
}

#[test]
fn enter_on_an_empty_or_blank_draft_cancels_and_escape_throws_the_draft_away() {
    let mut c = chat();
    c.open(false);
    assert_eq!(c.handle_key(ChatKey::Enter), Some(ChatCommand::Cancel));
    assert!(!c.is_open());
    c.open(false);
    c.handle_text("   ");
    assert_eq!(c.handle_key(ChatKey::Enter), Some(ChatCommand::Cancel));
    c.open(false);
    c.handle_text("draft");
    assert_eq!(c.handle_key(ChatKey::Escape), Some(ChatCommand::Cancel));
    assert!(!c.is_open());
    c.open(false);
    assert_eq!(c.input(), "", "a new input starts empty");
    assert_eq!(c.cursor(), 0);
}

#[test]
fn opening_an_open_input_keeps_the_draft() {
    let mut c = chat();
    c.open(false);
    c.handle_text("keep me");
    c.open(true);
    assert_eq!(c.input(), "keep me");
    assert_eq!(c.cursor(), 7);
    assert!(c.is_team());
}

#[test]
fn the_input_line_shows_the_target_the_text_and_the_cursor() {
    let mut c = chat();
    c.open(false);
    assert_eq!(c.input_line(), "[All] > |");
    c.handle_text("abc");
    assert_eq!(c.input_line(), "[All] > abc|");
    c.handle_key(ChatKey::Left);
    assert_eq!(c.input_line(), "[All] > ab|c");
    c.handle_key(ChatKey::Tab);
    assert_eq!(c.input_line(), "[Team] > ab|c");
}

#[test]
fn a_long_draft_is_shown_as_a_window_that_follows_the_cursor() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 20,
        ..ChatConfig::default()
    });
    c.open(false);
    let text: String = ('a'..='z').cycle().take(100).collect();
    c.handle_text(&text);
    let line = c.input_line();
    assert!(line.chars().count() <= 20, "{line}");
    assert!(line.ends_with('|'));
    c.handle_key(ChatKey::Home);
    let line = c.input_line();
    assert!(line.chars().count() <= 20, "{line}");
    assert!(line.starts_with("[All] > |abc"), "{line}");
    for _ in 0..50 {
        c.handle_key(ChatKey::Right);
    }
    let line = c.input_line();
    assert!(line.chars().count() <= 20, "{line}");
    assert_eq!(line.matches('|').count(), 1);
    let expected = text.chars().nth(49).unwrap();
    let at = line.find('|').unwrap();
    assert_eq!(line[..at].chars().last(), Some(expected));
}

#[test]
fn drawing_an_empty_closed_chat_draws_nothing() {
    let c = chat();
    let mut list = DrawList::default();
    c.draw_into(&mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
}

#[test]
fn drawing_puts_the_rows_in_stripes_with_the_newest_at_the_bottom_and_fades_them() {
    let mut c = ChatBox::new(ChatConfig {
        visible_ticks: 10,
        fade_ticks: 10,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "first");
    for _ in 0..15 {
        c.update();
    }
    c.push(ChatKind::Team, "Tom", "second");
    let mut list = DrawList::default();
    c.draw_into(&mut list);
    assert_eq!(list.quads.len(), 2);
    assert_eq!(list.texts.len(), 2);
    assert!(
        list.texts[0].pos.y < list.texts[1].pos.y,
        "the newest is lowest"
    );
    assert!(
        list.quads[0].color.0[3] < list.quads[1].color.0[3],
        "the older stripe is fainter"
    );
    assert!(close(list.texts[0].color.0[3], 0.5));
    assert!(close(list.texts[1].color.0[3], 1.0));
    assert!(list.quads.iter().all(|q| inside_canvas(q.rect)));
}

#[test]
fn an_open_input_is_drawn_below_the_rows_and_a_closed_one_is_not() {
    let mut c = chat();
    c.push(ChatKind::System, "", "hello");
    let mut closed = DrawList::default();
    c.draw_into(&mut closed);
    assert!(!texts(&closed).iter().any(|t| t.contains('>')));
    c.open(true);
    c.handle_text("typing");
    let mut open = DrawList::default();
    c.draw_into(&mut open);
    let input = open
        .texts
        .iter()
        .find(|t| t.text.contains("typing"))
        .unwrap();
    let row = open.texts.iter().find(|t| t.text == "hello").unwrap();
    assert!(
        input.pos.y > row.pos.y + 24.0,
        "below the row without touching it"
    );
    assert!(input.text.contains("[Team] > typing|"));
    assert!(open.quads.iter().all(|q| inside_canvas(q.rect)));
    let last_stripe = open.quads[0].rect;
    let input_box = open.quads[1].rect;
    assert!(last_stripe.y + last_stripe.h <= input_box.y);
}

// ---- nameplates -----------------------------------------------------------------------------------------------------------

fn plate(id: u32, x: f32, y: f32, distance: f32) -> Nameplate {
    Nameplate {
        id,
        name: format!("P{id}"),
        health: 100.0,
        max_health: 100.0,
        armor: 0.0,
        max_armor: 100.0,
        talking: false,
        at: Projected {
            screen: Vec2 { x, y },
            distance,
            in_front: true,
        },
    }
}

fn layout(players: &[Nameplate]) -> Vec<PlateLayout> {
    let mut out = Vec::new();
    Nameplates::default().layout_into(players, &mut out);
    out
}

#[test]
fn a_near_teammate_gets_a_full_size_plate_standing_on_the_anchor() {
    let l = layout(&[plate(1, 960.0, 500.0, 5.0)]);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].id, 1);
    assert_eq!(l[0].scale, 1.0);
    assert_eq!(l[0].alpha, 1.0);
    assert!(
        close(l[0].rect.x + l[0].rect.w / 2.0, 960.0),
        "centred on the anchor"
    );
    assert!(
        close(l[0].rect.y + l[0].rect.h, 500.0),
        "the bottom edge on the anchor"
    );
}

#[test]
fn no_players_no_plates_and_the_output_is_cleared_first() {
    let mut out = layout(&[plate(1, 960.0, 500.0, 5.0)]);
    assert_eq!(out.len(), 1);
    Nameplates::default().layout_into(&[], &mut out);
    assert!(out.is_empty());
    let mut list = DrawList::default();
    Nameplates::default().draw_into(&[], &mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
}

#[test]
fn a_player_behind_the_camera_has_no_plate_even_if_the_position_is_on_screen() {
    let mut p = plate(1, 960.0, 500.0, 5.0);
    p.at.in_front = false;
    assert!(layout(&[p]).is_empty());
}

#[test]
fn off_screen_and_bad_positions_have_no_plate() {
    let cases = [
        (-1.0, 500.0),
        (1921.0, 500.0),
        (960.0, -1.0),
        (960.0, 1081.0),
        (f32::NAN, 500.0),
        (960.0, f32::NAN),
        (f32::INFINITY, 500.0),
        (960.0, f32::NEG_INFINITY),
    ];
    for (x, y) in cases {
        assert!(layout(&[plate(1, x, y, 5.0)]).is_empty(), "{x},{y}");
    }
    for d in [f32::NAN, f32::INFINITY, -1.0, 61.0, 1.0e9] {
        assert!(
            layout(&[plate(1, 960.0, 500.0, d)]).is_empty(),
            "distance {d}"
        );
    }
    assert_eq!(layout(&[plate(1, 0.0, 0.0, 5.0)]).len(), 1);
    assert_eq!(layout(&[plate(1, 1920.0, 1080.0, 5.0)]).len(), 1);
}

#[test]
fn plates_shrink_and_fade_with_distance_between_near_and_far() {
    let at = |d: f32| layout(&[plate(1, 960.0, 500.0, d)])[0];
    assert_eq!((at(0.0).scale, at(15.0).scale), (1.0, 1.0));
    let mid = at(37.5);
    assert!(close(mid.scale, 0.8) && close(mid.alpha, 0.675), "{mid:?}");
    let far = at(60.0);
    assert!(
        close(far.scale, 0.6) && close(far.alpha, 0.35),
        "exactly at the far limit it is still drawn: {far:?}"
    );
    assert!(close(far.rect.w, 168.0 * 0.6));
    assert!(at(30.0).scale > at(40.0).scale && at(40.0).scale > at(50.0).scale);
    assert!(at(30.0).alpha > at(40.0).alpha);
}

#[test]
fn a_far_limit_at_or_below_the_near_limit_does_not_divide_by_zero() {
    let n = Nameplates::new(NameplateConfig {
        near: 20.0,
        far: 20.0,
        ..NameplateConfig::default()
    });
    let mut out = Vec::new();
    n.layout_into(
        &[plate(1, 500.0, 500.0, 10.0), plate(2, 600.0, 500.0, 20.0)],
        &mut out,
    );
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|l| l.scale == 1.0 && l.alpha == 1.0));
}

#[test]
fn only_the_nearest_plates_up_to_the_limit_are_kept_nearest_first() {
    let players: Vec<Nameplate> = (0..16)
        .map(|i| plate(i, 100.0 + 100.0 * i as f32, 500.0, 59.0 - i as f32))
        .collect();
    let l = layout(&players);
    assert_eq!(l.len(), 8);
    let got: Vec<u32> = l.iter().map(|p| p.id).collect();
    assert_eq!(got, vec![15, 14, 13, 12, 11, 10, 9, 8]);
    let few = Nameplates::new(NameplateConfig {
        max_count: 0,
        ..NameplateConfig::default()
    });
    let mut out = Vec::new();
    few.layout_into(&players, &mut out);
    assert!(out.is_empty());
}

#[test]
fn players_at_the_same_distance_keep_their_list_order() {
    let l = layout(&[
        plate(7, 100.0, 500.0, 10.0),
        plate(3, 400.0, 500.0, 10.0),
        plate(9, 800.0, 500.0, 10.0),
    ]);
    assert_eq!(l.iter().map(|p| p.id).collect::<Vec<_>>(), vec![7, 3, 9]);
    assert_eq!(l.iter().map(|p| p.index).collect::<Vec<_>>(), vec![0, 1, 2]);
}

#[test]
fn a_hidden_player_does_not_use_up_a_place() {
    let mut players: Vec<Nameplate> = (0..8)
        .map(|i| plate(i, 100.0 + 100.0 * i as f32, 500.0, 10.0 + i as f32))
        .collect();
    let mut behind = plate(99, 500.0, 500.0, 1.0);
    behind.at.in_front = false;
    players.insert(0, behind);
    assert_eq!(layout(&players).len(), 8);
}

#[test]
fn a_plate_near_an_edge_is_moved_inside_the_safe_area() {
    for (x, y) in [
        (0.0, 0.0),
        (1920.0, 1080.0),
        (0.0, 1080.0),
        (1920.0, 0.0),
        (10.0, 540.0),
        (960.0, 5.0),
        (960.0, 1075.0),
    ] {
        for d in [5.0, 60.0] {
            let l = layout(&[plate(1, x, y, d)]);
            assert_eq!(l.len(), 1);
            assert!(inside(l[0].rect, SAFE_AREA), "{x},{y},{d}: {:?}", l[0].rect);
        }
    }
    let l = layout(&[plate(1, 960.0, 500.0, 5.0)]);
    assert!(
        close(l[0].rect.x, 960.0 - 84.0),
        "an inside plate is not moved"
    );
    assert!(close(l[0].rect.y, 500.0 - 52.0));
    let l = layout(&[plate(1, 0.0, 500.0, 5.0)]);
    assert!(close(l[0].rect.x, SAFE_AREA.x));
    let l = layout(&[plate(1, 1920.0, 500.0, 5.0)]);
    assert!(close(l[0].rect.x + l[0].rect.w, SAFE_AREA.x + SAFE_AREA.w));
    let l = layout(&[plate(1, 960.0, 0.0, 5.0)]);
    assert!(close(l[0].rect.y, SAFE_AREA.y));
    let l = layout(&[plate(1, 960.0, 1080.0, 5.0)]);
    assert!(close(l[0].rect.y + l[0].rect.h, SAFE_AREA.y + SAFE_AREA.h));
}

#[test]
fn a_safe_area_smaller_than_a_plate_or_not_a_number_does_not_panic() {
    let n = Nameplates::new(NameplateConfig {
        safe: Rect {
            x: 100.0,
            y: 100.0,
            w: 10.0,
            h: 10.0,
        },
        ..NameplateConfig::default()
    });
    let mut out = Vec::new();
    n.layout_into(&[plate(1, 960.0, 500.0, 5.0)], &mut out);
    assert_eq!(out.len(), 1);
    assert!(close(out[0].rect.x, 100.0) && close(out[0].rect.y, 100.0));
    let nan = Nameplates::new(NameplateConfig {
        safe: Rect {
            x: f32::NAN,
            y: f32::NAN,
            w: f32::NAN,
            h: f32::NAN,
        },
        ..NameplateConfig::default()
    });
    nan.layout_into(&[plate(1, 960.0, 500.0, 5.0)], &mut out);
}

#[test]
fn health_and_armor_are_fractions_clamped_to_zero_and_one() {
    let mut p = plate(1, 960.0, 500.0, 5.0);
    p.health = 50.0;
    p.armor = 25.0;
    let l = layout(&[p.clone()])[0];
    assert!(close(l.health, 0.5) && close(l.armor, 0.25));
    p.health = 500.0;
    p.armor = -5.0;
    let l = layout(&[p.clone()])[0];
    assert_eq!((l.health, l.armor), (1.0, 0.0));
    p.health = f32::NAN;
    p.max_armor = 0.0;
    p.armor = 10.0;
    let l = layout(&[p.clone()])[0];
    assert_eq!((l.health, l.armor), (0.0, 0.0));
    p.health = 10.0;
    p.max_health = 0.0;
    assert_eq!(layout(&[p.clone()])[0].health, 0.0);
    p.max_health = f32::NAN;
    assert_eq!(layout(&[p.clone()])[0].health, 0.0);
    p.max_health = 100.0;
    p.health = f32::INFINITY;
    assert_eq!(layout(&[p])[0].health, 1.0);
}

#[test]
fn a_plate_draws_its_name_a_health_bar_in_proportion_and_an_armor_bar_only_with_armor() {
    let mut p = plate(1, 960.0, 500.0, 5.0);
    p.name = "Mia".into();
    p.health = 60.0;
    let mut list = DrawList::default();
    Nameplates::default().draw_into(&[p.clone()], &mut list);
    assert!(has_text(&list, "Mia"));
    let fill = quads_of(&list, HEALTH_OK);
    assert_eq!(fill.len(), 1);
    let track = list
        .quads
        .iter()
        .find(|q| q.color == Color([0.1, 0.1, 0.1, 1.0]))
        .unwrap()
        .rect;
    assert!(close(fill[0].w, track.w * 0.6));
    assert!(close(fill[0].x, track.x) && close(fill[0].y, track.y));
    assert!(quads_of(&list, ARMOR_BAR).is_empty());
    p.armor = 50.0;
    list.clear();
    Nameplates::default().draw_into(&[p], &mut list);
    let armor = quads_of(&list, ARMOR_BAR);
    assert_eq!(armor.len(), 1);
    assert!(close(armor[0].w, track.w * 0.5));
    assert!(armor[0].y >= track.y + track.h, "under the health bar");
}

#[test]
fn the_health_bar_changes_colour_at_half_and_at_a_quarter() {
    let colour_at = |h: f32| {
        let mut p = plate(1, 960.0, 500.0, 5.0);
        p.health = h;
        let mut list = DrawList::default();
        Nameplates::default().draw_into(&[p], &mut list);
        [HEALTH_OK, HEALTH_LOW, HEALTH_CRITICAL]
            .iter()
            .position(|c| !quads_of(&list, *c).is_empty())
    };
    assert_eq!(colour_at(100.0), Some(0));
    assert_eq!(colour_at(51.0), Some(0));
    assert_eq!(colour_at(50.0), Some(1));
    assert_eq!(colour_at(26.0), Some(1));
    assert_eq!(colour_at(25.0), Some(2));
    assert_eq!(colour_at(1.0), Some(2));
    assert_eq!(colour_at(0.0), None, "nothing to fill");
}

#[test]
fn a_dead_teammate_is_drawn_in_grey() {
    let mut p = plate(1, 960.0, 500.0, 5.0);
    p.health = 0.0;
    let mut list = DrawList::default();
    Nameplates::default().draw_into(&[p], &mut list);
    let name = &list.texts[0];
    assert!(name.color.0[0] < 0.7 && close(name.color.0[0], name.color.0[1]));
    let alive = plate(2, 960.0, 500.0, 5.0);
    list.clear();
    Nameplates::default().draw_into(&[alive], &mut list);
    assert_eq!(list.texts[0].color, Color([1.0, 1.0, 1.0, 1.0]));
}

#[test]
fn a_talking_teammate_gets_a_green_strip_and_a_long_name_is_cut() {
    let mut p = plate(1, 960.0, 500.0, 5.0);
    p.talking = true;
    p.name = "A very long player name indeed".into();
    let mut list = DrawList::default();
    Nameplates::default().draw_into(&[p], &mut list);
    assert_eq!(quads_of(&list, VOICE_ON).len(), 1);
    assert_eq!(list.texts[0].text.chars().count(), 16);
    assert!(list.texts[0].text.ends_with('\u{2026}'));
    let quiet = plate(2, 960.0, 500.0, 5.0);
    list.clear();
    Nameplates::default().draw_into(&[quiet], &mut list);
    assert!(quads_of(&list, VOICE_ON).is_empty());
}

#[test]
fn a_distant_plate_is_smaller_text_and_fainter_and_the_nearest_is_drawn_last() {
    let near = plate(1, 400.0, 500.0, 5.0);
    let far = plate(2, 1200.0, 500.0, 50.0);
    let mut list = DrawList::default();
    Nameplates::default().draw_into(&[near, far], &mut list);
    assert_eq!(list.texts.len(), 2);
    assert!(list.texts[0].size < list.texts[1].size);
    assert!(list.texts[0].color.0[3] < list.texts[1].color.0[3]);
    assert_eq!(list.texts[1].size, 24.0);
    assert_eq!(
        list.texts[0].text, "P2",
        "the far one first, so the near one is on top"
    );
}

// ---- directions -----------------------------------------------------------------------------------------------------------

#[test]
fn directions_are_the_nearest_of_eight() {
    use Direction::*;
    let cases = [
        (1.0, 0.0, East),
        (1.0, 1.0, SouthEast),
        (0.0, 1.0, South),
        (-1.0, 1.0, SouthWest),
        (-1.0, 0.0, West),
        (-1.0, -1.0, NorthWest),
        (0.0, -1.0, North),
        (1.0, -1.0, NorthEast),
        (10.0, 1.0, East),
        (1.0, 10.0, South),
        (-10.0, -1.0, West),
        (1.0, -10.0, North),
        (-1.0, -0.0, West),
    ];
    for (dx, dy, expected) in cases {
        assert_eq!(Direction::from_vector(dx, dy), expected, "{dx},{dy}");
    }
    assert_eq!(Direction::from_vector(0.0, 0.0), South);
    assert_eq!(Direction::from_vector(f32::NAN, 1.0), South);
    assert_eq!(Direction::from_vector(1.0, f32::INFINITY), South);
}

#[test]
fn every_direction_has_its_own_arrow() {
    use Direction::*;
    let all = [
        East, SouthEast, South, SouthWest, West, NorthWest, North, NorthEast,
    ];
    let glyphs: Vec<&str> = all.iter().map(|d| d.glyph()).collect();
    for (i, a) in glyphs.iter().enumerate() {
        assert_eq!(a.chars().count(), 1);
        for b in &glyphs[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert_eq!(East.glyph(), "\u{2192}");
    assert_eq!(North.glyph(), "\u{2191}");
}

// ---- ping markers ---------------------------------------------------------------------------------------------------------

fn proj(id: u32, x: f32, y: f32, distance: f32, in_front: bool) -> PingProjection {
    PingProjection {
        id,
        at: Projected {
            screen: Vec2 { x, y },
            distance,
            in_front,
        },
    }
}

fn ping_layout(p: &PingMarkers, projections: &[PingProjection]) -> Vec<PingLayout> {
    let mut out = Vec::new();
    p.layout_into(projections, &mut out);
    out
}

fn one_ping(ttl: u32) -> (PingMarkers, u32) {
    let mut p = PingMarkers::new();
    let id = p.add(1, "Mia", PingKind::Look, "Here", ttl).unwrap();
    (p, id)
}

#[test]
fn a_ping_gets_an_id_and_lives_exactly_its_lifetime() {
    let (mut p, id) = one_ping(3);
    assert_eq!(p.pings().len(), 1);
    assert_eq!(p.pings()[0].id, id);
    p.update();
    p.update();
    assert_eq!(p.pings().len(), 1);
    assert_eq!(p.pings()[0].ticks_left, 1);
    p.update();
    assert!(p.pings().is_empty());
    p.update();
}

#[test]
fn a_ping_with_no_lifetime_is_not_added() {
    let mut p = PingMarkers::new();
    assert_eq!(p.add(1, "Mia", PingKind::Look, "x", 0), None);
    assert!(p.pings().is_empty());
}

#[test]
fn a_new_ping_by_the_same_player_replaces_the_old_one_and_ids_are_distinct() {
    let mut p = PingMarkers::new();
    let a = p.add(1, "Mia", PingKind::Look, "one", 100).unwrap();
    let b = p.add(2, "Tom", PingKind::Danger, "two", 100).unwrap();
    let c = p.add(1, "Mia", PingKind::Item, "three", 100).unwrap();
    assert!(a != b && b != c && a != c);
    let labels: Vec<&str> = p.pings().iter().map(|x| x.label.as_str()).collect();
    assert_eq!(labels, vec!["two", "three"]);
}

#[test]
fn at_most_sixteen_pings_live_and_the_oldest_goes() {
    let mut p = PingMarkers::new();
    for owner in 0..20u32 {
        p.add(owner, "x", PingKind::Go, &format!("l{owner}"), 100);
    }
    assert_eq!(p.pings().len(), MAX_PINGS);
    assert_eq!(p.pings()[0].label, "l4");
    assert_eq!(p.pings()[15].label, "l19");
}

#[test]
fn a_label_and_an_owner_name_are_cleaned_and_cut() {
    let mut p = PingMarkers::new();
    p.add(
        1,
        &"N".repeat(40),
        PingKind::Look,
        &format!("a\nb{}", "c".repeat(40)),
        10,
    );
    assert_eq!(p.pings()[0].label.chars().count(), MAX_PING_LABEL_CHARS);
    assert!(p.pings()[0].label.starts_with("abc"));
    assert_eq!(p.pings()[0].owner_name.chars().count(), MAX_NAME_CHARS);
}

#[test]
fn a_ping_on_screen_sits_on_its_target() {
    let (p, id) = one_ping(100);
    let l = ping_layout(&p, &[proj(id, 700.0, 300.0, 42.0, true)]);
    assert_eq!(l.len(), 1);
    assert!(l[0].on_screen);
    assert_eq!(l[0].arrow, None);
    assert_eq!(l[0].center, Vec2 { x: 700.0, y: 300.0 });
    assert_eq!(l[0].alpha, 1.0);
}

#[test]
fn a_ping_fades_during_its_last_thirty_ticks() {
    let (mut p, id) = one_ping(100);
    let alpha = |p: &PingMarkers| ping_layout(p, &[proj(id, 700.0, 300.0, 1.0, true)])[0].alpha;
    for _ in 0..69 {
        p.update();
    }
    assert_eq!(alpha(&p), 1.0, "31 ticks left");
    p.update();
    assert_eq!(alpha(&p), 1.0, "30 ticks left");
    p.update();
    assert!(close(alpha(&p), 29.0 / 30.0));
    for _ in 0..14 {
        p.update();
    }
    assert!(close(alpha(&p), 0.5), "15 ticks left");
}

#[test]
fn a_ping_that_was_not_projected_is_not_drawn() {
    let (p, id) = one_ping(100);
    assert!(ping_layout(&p, &[]).is_empty());
    assert!(ping_layout(&p, &[proj(id + 1, 700.0, 300.0, 1.0, true)]).is_empty());
    let mut list = DrawList::default();
    p.draw_into(&[], &mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
}

#[test]
fn a_ping_off_the_side_is_pinned_to_the_edge_of_the_safe_area_with_an_arrow() {
    let (p, id) = one_ping(100);
    let right = ping_layout(&p, &[proj(id, 2500.0, 540.0, 10.0, true)])[0];
    assert!(!right.on_screen);
    assert_eq!(right.arrow, Some(Direction::East));
    assert!(close(right.center.x, SAFE_AREA.x + SAFE_AREA.w - 16.0));
    assert!(close(right.center.y, 540.0));
    let left = ping_layout(&p, &[proj(id, -500.0, 540.0, 10.0, true)])[0];
    assert_eq!(left.arrow, Some(Direction::West));
    assert!(close(left.center.x, SAFE_AREA.x + 16.0));
    let up = ping_layout(&p, &[proj(id, 960.0, -900.0, 10.0, true)])[0];
    assert_eq!(up.arrow, Some(Direction::North));
    assert!(close(up.center.y, SAFE_AREA.y + 16.0) && close(up.center.x, 960.0));
    let down = ping_layout(&p, &[proj(id, 960.0, 3000.0, 10.0, true)])[0];
    assert_eq!(down.arrow, Some(Direction::South));
    assert!(close(down.center.y, SAFE_AREA.y + SAFE_AREA.h - 16.0));
}

#[test]
fn a_ping_off_a_corner_is_pinned_on_the_line_to_its_target() {
    let (p, id) = one_ping(100);
    let l = ping_layout(&p, &[proj(id, 2960.0, 1620.0, 10.0, true)])[0];
    assert_eq!(l.arrow, Some(Direction::SouthEast));
    let ratio = (l.center.x - 960.0) / (l.center.y - 540.0);
    assert!(close(ratio, 2000.0 / 1080.0), "{ratio}");
    let right = SAFE_AREA.x + SAFE_AREA.w - 16.0;
    let bottom = SAFE_AREA.y + SAFE_AREA.h - 16.0;
    assert!(close(l.center.x, right) || close(l.center.y, bottom));
    assert!(l.center.x <= right + 1e-3 && l.center.y <= bottom + 1e-3);
    // Steeper than the screen: it leaves through the bottom.
    let l = ping_layout(&p, &[proj(id, 1060.0, 5000.0, 10.0, true)])[0];
    assert!(close(l.center.y, bottom), "{:?}", l.center);
    assert!(l.center.x > 960.0 && l.center.x < 1000.0);
}

#[test]
fn a_ping_on_the_screen_but_in_the_margin_is_pinned_inside_it() {
    let (p, id) = one_ping(100);
    let l = ping_layout(&p, &[proj(id, 20.0, 540.0, 10.0, true)])[0];
    assert!(!l.on_screen);
    assert!(close(l.center.x, SAFE_AREA.x + 16.0));
    let l = ping_layout(&p, &[proj(id, SAFE_AREA.x + 16.0, 540.0, 10.0, true)])[0];
    assert!(l.on_screen);
    let l = ping_layout(&p, &[proj(id, SAFE_AREA.x + 15.0, 540.0, 10.0, true)])[0];
    assert!(!l.on_screen);
    let right = SAFE_AREA.x + SAFE_AREA.w - 16.0;
    assert!(ping_layout(&p, &[proj(id, right, 540.0, 10.0, true)])[0].on_screen);
    assert!(!ping_layout(&p, &[proj(id, right + 1.0, 540.0, 10.0, true)])[0].on_screen);
    let top = SAFE_AREA.y + 16.0;
    assert!(ping_layout(&p, &[proj(id, 960.0, top, 10.0, true)])[0].on_screen);
    assert!(!ping_layout(&p, &[proj(id, 960.0, top - 1.0, 10.0, true)])[0].on_screen);
    let bottom = SAFE_AREA.y + SAFE_AREA.h - 16.0;
    assert!(ping_layout(&p, &[proj(id, 960.0, bottom, 10.0, true)])[0].on_screen);
    assert!(!ping_layout(&p, &[proj(id, 960.0, bottom + 1.0, 10.0, true)])[0].on_screen);
}

#[test]
fn a_ping_behind_the_camera_points_the_way_to_turn() {
    let (p, id) = one_ping(100);
    let l = ping_layout(&p, &[proj(id, 1400.0, 540.0, 10.0, false)])[0];
    assert!(!l.on_screen);
    assert_eq!(l.arrow, Some(Direction::West));
    assert!(close(l.center.x, SAFE_AREA.x + 16.0));
    let l = ping_layout(&p, &[proj(id, 960.0, 540.0, 10.0, false)])[0];
    assert_eq!(l.arrow, Some(Direction::South));
    assert!(close(l.center.y, SAFE_AREA.y + SAFE_AREA.h - 16.0));
    let l = ping_layout(&p, &[proj(id, 700.0, 300.0, 10.0, false)])[0];
    assert!(!l.on_screen);
    assert_eq!(l.arrow, Some(Direction::SouthEast));
}

#[test]
fn a_ping_in_front_at_the_exact_centre_is_on_screen() {
    let (p, id) = one_ping(100);
    let l = ping_layout(&p, &[proj(id, 960.0, 540.0, 10.0, true)])[0];
    assert!(l.on_screen);
}

#[test]
fn a_ping_at_a_bad_position_is_not_drawn_and_does_not_panic() {
    let (p, id) = one_ping(100);
    for (x, y) in [
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (f32::INFINITY, 0.0),
        (0.0, f32::NEG_INFINITY),
    ] {
        assert!(ping_layout(&p, &[proj(id, x, y, 1.0, true)]).is_empty());
        assert!(ping_layout(&p, &[proj(id, x, y, 1.0, false)]).is_empty());
        let mut list = DrawList::default();
        p.draw_into(&[proj(id, x, y, f32::NAN, true)], &mut list);
        assert!(list.quads.is_empty());
    }
    // Almost the centre, behind the camera: still pinned somewhere finite.
    let l = ping_layout(&p, &[proj(id, 960.0 + 1e-30, 540.0, 1.0, false)])[0];
    assert!(l.center.x.is_finite() && l.center.y.is_finite());
    // A bad distance only loses the distance text.
    let mut list = DrawList::default();
    p.draw_into(&[proj(id, 700.0, 300.0, f32::NAN, true)], &mut list);
    assert_eq!(list.quads.len(), 1);
    assert!(!texts(&list).iter().any(|t| t.contains(" m")));
    assert!(has_text(&list, "Mia"));
    list.clear();
    p.draw_into(&[proj(id, 700.0, 300.0, -3.0, true)], &mut list);
    assert!(!texts(&list).iter().any(|t| t.contains(" m")));
}

#[test]
fn a_marker_is_a_square_with_its_symbol_the_label_and_the_distance() {
    let mut p = PingMarkers::new();
    let id = p.add(1, "Mia", PingKind::Danger, "Boss", 100).unwrap();
    let mut list = DrawList::default();
    p.draw_into(&[proj(id, 700.0, 300.0, 41.6, true)], &mut list);
    assert_eq!(list.quads.len(), 1);
    assert!(close(list.quads[0].rect.x + 16.0, 700.0));
    assert!(close(list.quads[0].rect.y + 16.0, 300.0));
    assert!(close(list.quads[0].rect.w, 32.0) && close(list.quads[0].rect.h, 32.0));
    assert!(has_text(&list, "!"));
    assert!(has_text(&list, "Boss"));
    assert!(has_text(&list, "Mia, 42 m"));
    let mut seen = Vec::new();
    for kind in [
        PingKind::Look,
        PingKind::Danger,
        PingKind::Item,
        PingKind::Go,
    ] {
        let mut p = PingMarkers::new();
        let id = p.add(1, "", kind, "", 100).unwrap();
        let mut list = DrawList::default();
        p.draw_into(&[proj(id, 700.0, 300.0, 1.0, true)], &mut list);
        seen.push((
            list.quads[0].color.0.to_vec(),
            list.texts[0].text.to_string(),
        ));
    }
    for i in 0..seen.len() {
        for j in i + 1..seen.len() {
            assert_ne!(seen[i].0, seen[j].0);
            assert_ne!(seen[i].1, seen[j].1);
        }
    }
}

#[test]
fn an_empty_label_and_name_leave_only_the_distance() {
    let mut p = PingMarkers::new();
    let id = p.add(1, "", PingKind::Look, "", 100).unwrap();
    let mut list = DrawList::default();
    p.draw_into(&[proj(id, 700.0, 300.0, 5.0, true)], &mut list);
    assert_eq!(texts(&list), vec!["*", "5 m"]);
}

#[test]
fn a_pinned_marker_shows_an_arrow_in_place_of_the_symbol() {
    let (p, id) = one_ping(100);
    let mut list = DrawList::default();
    p.draw_into(&[proj(id, 3000.0, 540.0, 10.0, true)], &mut list);
    assert!(has_text(&list, Direction::East.glyph()));
    assert!(!has_text(&list, "*"));
}

#[test]
fn a_fading_marker_is_fainter() {
    let (mut p, id) = one_ping(100);
    for _ in 0..85 {
        p.update();
    }
    let mut list = DrawList::default();
    p.draw_into(&[proj(id, 700.0, 300.0, 10.0, true)], &mut list);
    assert!(close(list.quads[0].color.0[3], 0.5));
    assert!(list.texts.iter().all(|t| t.color.0[3] <= 0.5 + 1e-3));
}

#[test]
fn marker_text_goes_toward_the_middle_of_the_screen_and_stays_on_the_canvas() {
    let (p, id) = one_ping(100);
    for (y, below) in [(200.0, true), (900.0, false)] {
        let mut list = DrawList::default();
        p.draw_into(&[proj(id, 960.0, y, 10.0, true)], &mut list);
        let label = list.texts.iter().find(|t| t.text == "Here").unwrap();
        assert_eq!(label.pos.y > y, below, "y={y}");
    }
    for x in [-5000.0, 0.0, 100.0, 1900.0, 9000.0] {
        let mut list = DrawList::default();
        p.draw_into(&[proj(id, x, 540.0, 10.0, true)], &mut list);
        for t in list.texts.iter().filter(|t| t.align == TextAlign::Center) {
            let half = estimate_text_width(t.text.chars().count(), t.size) / 2.0;
            assert!(
                t.pos.x - half >= 0.0 && t.pos.x + half <= CANVAS_W,
                "{x}: {t:?}"
            );
        }
    }
}

#[test]
fn the_text_of_a_marker_does_not_overlap_the_marker_or_itself() {
    let (p, id) = one_ping(100);
    for y in [200.0, 900.0] {
        let mut list = DrawList::default();
        p.draw_into(&[proj(id, 960.0, y, 10.0, true)], &mut list);
        let marker = list.quads[0].rect;
        let mut spans: Vec<(f32, f32)> = list
            .texts
            .iter()
            .skip(1)
            .map(|t| (t.pos.y, t.pos.y + t.size * 1.25))
            .collect();
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(spans.len(), 2);
        assert!(spans[0].1 <= spans[1].0 + 1e-3, "{spans:?}");
        for (top, bottom) in spans {
            assert!(
                bottom <= marker.y + 1e-3 || top >= marker.y + marker.h - 1e-3,
                "{y}: {marker:?} {top} {bottom}"
            );
        }
    }
}

// ---- voice indicators -----------------------------------------------------------------------------------------------------

fn sample(id: u32, level: f32) -> VoiceSample {
    VoiceSample {
        id,
        name: format!("v{id}"),
        level,
        local: false,
    }
}

fn speaker_ids(v: &VoiceIndicators) -> Vec<u32> {
    v.speakers().iter().map(|s| s.id).collect()
}

#[test]
fn nobody_talking_lists_nobody_and_draws_nothing() {
    let mut v = VoiceIndicators::new();
    v.update(&[]);
    assert!(v.speakers().is_empty());
    let mut list = DrawList::default();
    v.draw_into(&mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
}

#[test]
fn a_level_below_the_threshold_is_silence() {
    let mut v = VoiceIndicators::new();
    v.update(&[
        sample(1, 0.019),
        sample(2, 0.0),
        sample(3, -1.0),
        sample(4, f32::NAN),
    ]);
    assert!(v.speakers().is_empty());
    v.update(&[sample(1, 0.02)]);
    assert_eq!(speaker_ids(&v), vec![1]);
}

#[test]
fn a_speaker_stays_listed_for_the_hold_time_after_the_voice_stops() {
    let mut v = VoiceIndicators::new();
    v.update(&[sample(1, 0.5)]);
    assert!(close(v.speakers()[0].level, 0.5));
    for tick in 1..=VOICE_HOLD_TICKS {
        v.update(&[]);
        assert_eq!(speaker_ids(&v), vec![1], "silent tick {tick}");
        assert_eq!(v.speakers()[0].level, 0.0);
    }
    v.update(&[]);
    assert!(v.speakers().is_empty());
}

#[test]
fn talking_again_during_the_hold_restarts_it() {
    let mut v = VoiceIndicators::new();
    v.update(&[sample(1, 0.5)]);
    for _ in 0..15 {
        v.update(&[]);
    }
    v.update(&[sample(1, 0.3)]);
    assert!(close(v.speakers()[0].level, 0.3));
    for _ in 0..VOICE_HOLD_TICKS {
        v.update(&[]);
    }
    assert_eq!(speaker_ids(&v), vec![1]);
    v.update(&[]);
    assert!(v.speakers().is_empty());
}

#[test]
fn speakers_are_listed_by_id_and_a_player_heard_twice_counts_once_at_the_louder_level() {
    let mut v = VoiceIndicators::new();
    v.update(&[
        sample(9, 0.4),
        sample(2, 0.3),
        sample(9, 0.8),
        sample(5, 0.1),
        sample(5, 0.05),
    ]);
    assert_eq!(speaker_ids(&v), vec![2, 5, 9]);
    assert!(close(v.speakers()[2].level, 0.8));
    assert!(close(v.speakers()[1].level, 0.1));
    // The louder one first, then the quieter: the same result.
    let mut w = VoiceIndicators::new();
    w.update(&[sample(9, 0.8), sample(9, 0.4)]);
    assert!(close(w.speakers()[0].level, 0.8));
}

#[test]
fn levels_are_clamped_names_are_cut_and_the_local_player_is_marked() {
    let mut v = VoiceIndicators::new();
    let mut loud = sample(1, 7.0);
    loud.name = "N".repeat(60);
    loud.local = true;
    v.update(&[loud]);
    assert_eq!(v.speakers()[0].level, 1.0);
    assert_eq!(v.speakers()[0].name.chars().count(), MAX_VOICE_NAME_CHARS);
    assert!(v.speakers()[0].local);
    assert_eq!(v.speakers()[0].hold, VOICE_HOLD_TICKS);
}

#[test]
fn at_most_six_are_listed_and_the_quietest_go_first_with_holding_speakers_before_talking_ones() {
    let mut v = VoiceIndicators::new();
    v.update(&[sample(100, 0.9)]);
    let talkers: Vec<VoiceSample> = (1..=8).map(|i| sample(i, i as f32 / 10.0)).collect();
    v.update(&talkers);
    assert_eq!(v.speakers().len(), MAX_SPEAKERS);
    assert_eq!(speaker_ids(&v), vec![3, 4, 5, 6, 7, 8]);
    let mut v = VoiceIndicators::new();
    let same: Vec<VoiceSample> = (1..=7).map(|i| sample(i, 0.5)).collect();
    v.update(&same);
    assert_eq!(speaker_ids(&v), vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn drawing_lists_a_row_per_speaker_with_a_meter_in_proportion_to_the_level() {
    let mut v = VoiceIndicators::new();
    let mut me = sample(1, 0.5);
    me.local = true;
    v.update(&[me, sample(2, 1.0)]);
    let mut list = DrawList::default();
    v.draw_into(&mut list);
    assert!(has_text(&list, "v1") && has_text(&list, "v2"));
    let meters = quads_of(&list, VOICE_ON);
    assert_eq!(meters.len(), 2);
    assert!(close(meters[0].h, 12.0) && close(meters[1].h, 24.0));
    // The meter fills from the bottom: both end on the same line relative to their row.
    let rows: Vec<Rect> = list
        .quads
        .iter()
        .filter(|q| q.rect.w > 200.0)
        .map(|q| q.rect)
        .collect();
    assert_eq!(rows.len(), 2);
    assert!(close(
        meters[0].y + meters[0].h - rows[0].y,
        meters[1].y + meters[1].h - rows[1].y
    ));
    assert!(rows[1].y >= rows[0].y + rows[0].h);
    assert!(rows.iter().all(|r| inside(*r, SAFE_AREA)));
    let strips = list.quads.iter().filter(|q| q.rect.w == 4.0).count();
    assert_eq!(strips, 1, "only the local player has the orange strip");
}

#[test]
fn a_holding_speaker_is_drawn_dimmed_with_an_empty_meter() {
    let mut v = VoiceIndicators::new();
    v.update(&[sample(1, 0.8)]);
    let mut talking = DrawList::default();
    v.draw_into(&mut talking);
    v.update(&[]);
    let mut holding = DrawList::default();
    v.draw_into(&mut holding);
    assert!(holding.texts[0].color.0[3] < talking.texts[0].color.0[3]);
    assert!(holding.quads[0].color.0[3] < talking.quads[0].color.0[3]);
    assert_eq!(holding.quads.last().unwrap().rect.h, 0.0);
}

// ---- all together ---------------------------------------------------------------------------------------------------------

#[test]
fn nothing_showing_draws_nothing() {
    let all = CoopOverlays::default();
    let mut list = DrawList::default();
    all.draw_into(&[], &[], &mut list);
    assert!(list.quads.is_empty() && list.texts.is_empty());
}

#[test]
fn the_update_ages_the_chat_and_the_pings_and_follows_the_voice() {
    let mut all = CoopOverlays::new(
        ChatConfig {
            visible_ticks: 2,
            fade_ticks: 1,
            ..ChatConfig::default()
        },
        NameplateConfig::default(),
    );
    all.chat.push(ChatKind::System, "", "hi");
    all.pings.add(1, "Mia", PingKind::Look, "x", 2);
    all.update(&[sample(1, 0.5)]);
    assert_eq!(all.chat.visible_rows().len(), 1);
    assert_eq!(all.pings.pings().len(), 1);
    assert_eq!(all.voice.speakers().len(), 1);
    all.update(&[]);
    all.update(&[]);
    assert!(all.chat.visible_rows().is_empty());
    assert!(all.pings.pings().is_empty());
}

#[test]
fn a_busy_game_draws_everything_inside_the_canvas_in_the_documented_order() {
    let mut all = CoopOverlays::default();
    all.scoreboard.set_held(true);
    let players: Vec<ScoreEntry> = (0..16)
        .map(|i| entry(i, "Player", i, 0, i as i64))
        .collect();
    all.scoreboard.set_entries(&players);
    for i in 0..12 {
        all.chat
            .push(ChatKind::Player, "Mia", &format!("message {i}"));
    }
    all.chat.open(false);
    let id = all
        .pings
        .add(1, "Mia", PingKind::Danger, "Boss", 100)
        .unwrap();
    all.update(&[sample(1, 0.5), sample(2, 0.9)]);
    let plates: Vec<Nameplate> = (0..10)
        .map(|i| plate(i, 100.0 + 150.0 * i as f32, 600.0, 5.0 + 5.0 * i as f32))
        .collect();
    let mut list = DrawList::default();
    all.draw_into(&plates, &[proj(id, 4000.0, 100.0, 80.0, true)], &mut list);
    assert!(list.quads.len() > 20 && list.texts.len() > 40);
    assert!(list.quads.iter().all(|q| inside_canvas(q.rect)));
    let at = |text: &str| {
        list.texts
            .iter()
            .position(|t| t.text.starts_with(text))
            .unwrap()
    };
    let (nameplate, ping, voice, chat, board) = (
        at("P0"),
        at("Boss"),
        at("v1"),
        at("[All] >"),
        at("Scoreboard"),
    );
    assert!(nameplate < ping && ping < voice && voice < chat && chat < board);
}

// ---- properties -----------------------------------------------------------------------------------------------------------

fn any_f32() -> impl Strategy<Value = f32> {
    prop_oneof![
        any::<f32>(),
        Just(f32::NAN),
        Just(f32::INFINITY),
        Just(f32::NEG_INFINITY),
        Just(0.0),
        -3000.0f32..3000.0,
    ]
}

proptest! {
    #[test]
    fn nameplates_never_panic_and_always_land_inside_the_safe_area(
        x in any_f32(), y in any_f32(), d in any_f32(), health in any_f32(), front in any::<bool>(),
    ) {
        let mut p = plate(1, x, y, d);
        p.health = health;
        p.at.in_front = front;
        let mut out = Vec::new();
        Nameplates::default().layout_into(&[p.clone()], &mut out);
        for l in &out {
            prop_assert!(inside(l.rect, SAFE_AREA), "{:?}", l);
            prop_assert!((0.0..=1.0).contains(&l.health));
            prop_assert!(l.scale >= 0.6 - 1e-6 && l.scale <= 1.0);
        }
        let mut list = DrawList::default();
        Nameplates::default().draw_into(&[p], &mut list);
    }

    #[test]
    fn ping_markers_never_panic_and_always_land_inside_the_safe_area(
        x in any_f32(), y in any_f32(), d in any_f32(), front in any::<bool>(),
    ) {
        let (p, id) = one_ping(50);
        let projections = [proj(id, x, y, d, front)];
        let mut out = Vec::new();
        p.layout_into(&projections, &mut out);
        for l in &out {
            prop_assert!(l.center.x >= SAFE_AREA.x && l.center.x <= SAFE_AREA.x + SAFE_AREA.w, "{:?}", l);
            prop_assert!(l.center.y >= SAFE_AREA.y && l.center.y <= SAFE_AREA.y + SAFE_AREA.h, "{:?}", l);
            prop_assert_eq!(l.on_screen, l.arrow.is_none());
        }
        let mut list = DrawList::default();
        p.draw_into(&projections, &mut list);
        prop_assert!(list.texts.iter().all(|t| t.pos.x.is_finite() && t.pos.y.is_finite()));
    }

    #[test]
    fn the_chat_input_never_panics_and_never_exceeds_its_limit(
        ops in proptest::collection::vec((0u8..12, "\\PC{0,12}"), 0..60),
    ) {
        let mut c = ChatBox::new(ChatConfig { max_message_chars: 20, wrap_chars: 14, ..ChatConfig::default() });
        c.open(false);
        for (op, text) in ops {
            match op {
                0 => c.handle_text(&text),
                1 => { c.handle_key(ChatKey::Left); }
                2 => { c.handle_key(ChatKey::Right); }
                3 => { c.handle_key(ChatKey::Home); }
                4 => { c.handle_key(ChatKey::End); }
                5 => { c.handle_key(ChatKey::Backspace); }
                6 => { c.handle_key(ChatKey::Delete); }
                7 => { c.handle_key(ChatKey::Tab); }
                8 => c.push(ChatKind::Player, &text, &text),
                9 => { c.handle_key(ChatKey::Enter); c.open(true); }
                10 => c.update(),
                _ => { c.handle_key(ChatKey::Escape); c.open(false); }
            }
            prop_assert!(c.input().chars().count() <= 20);
            prop_assert!(c.cursor() <= c.input().chars().count());
            prop_assert!(c.input_line().chars().count() <= 14);
            prop_assert!(c.len() <= 100);
            prop_assert!(c.visible_rows().len() <= 8);
        }
        let mut list = DrawList::default();
        c.draw_into(&mut list);
    }

    #[test]
    fn the_scoreboard_is_sorted_bounded_and_keeps_the_local_player(
        rows in proptest::collection::vec((0u32..50, 0u32..30, 0u32..30, -100i64..100, any::<bool>()), 0..40),
    ) {
        let entries: Vec<ScoreEntry> = rows
            .iter()
            .enumerate()
            .map(|(i, (id, k, d, s, local))| {
                let mut e = entry(*id * 100 + i as u32, "n", *k, *d, *s);
                e.local = *local && i == 3;
                e
            })
            .collect();
        let mut sb = Scoreboard::new();
        sb.set_held(true);
        sb.set_entries(&entries);
        prop_assert!(sb.rows().len() <= MAX_SCOREBOARD_ROWS);
        prop_assert_eq!(sb.rows().len() + sb.hidden(), entries.len());
        if entries.iter().any(|e| e.local) {
            prop_assert!(sb.rows().iter().any(|e| e.local));
        }
        let mut list = DrawList::default();
        sb.draw_into(&mut list);
        prop_assert!(list.quads.iter().all(|q| inside_canvas(q.rect)));
        let body = if sb.hidden() > 0 { &sb.rows()[..sb.rows().len() - 1] } else { sb.rows() };
        for pair in body.windows(2) {
            prop_assert!(pair[0].score >= pair[1].score);
        }
    }

    #[test]
    fn the_voice_list_is_bounded_and_ordered(
        frames in proptest::collection::vec(proptest::collection::vec((0u32..12, any_f32()), 0..12), 0..30),
    ) {
        let mut v = VoiceIndicators::new();
        for frame in frames {
            let samples: Vec<VoiceSample> = frame.into_iter().map(|(id, level)| sample(id, level)).collect();
            v.update(&samples);
            prop_assert!(v.speakers().len() <= MAX_SPEAKERS);
            prop_assert!(v.speakers().windows(2).all(|w| w[0].id < w[1].id));
            prop_assert!(v.speakers().iter().all(|s| (0.0..=1.0).contains(&s.level)));
        }
    }
}

// ---- added after the first mutation run (each kills a mutant that survived it) ---------------------------------------------

#[test]
fn the_scoreboard_panel_holds_the_more_line_and_zebra_stripes_alternate() {
    let mut sb = Scoreboard::new();
    sb.set_held(true);
    let players: Vec<ScoreEntry> = (0..17)
        .map(|i| entry(i, "x", 0, 0, 100 - i as i64))
        .collect();
    sb.set_entries(&players);
    let mut list = DrawList::default();
    sb.draw_into(&mut list);
    assert!(
        has_text(&list, "+1 more"),
        "one player cut is still counted"
    );
    let panel = list.quads[0].rect;
    let more = list.texts.iter().find(|t| t.text == "+1 more").unwrap();
    let last_stripe = list.quads.last().unwrap().rect;
    assert!(
        more.pos.y >= last_stripe.y + last_stripe.h,
        "below the last row"
    );
    assert!(
        more.pos.y + more.size <= panel.y + panel.h,
        "inside the panel"
    );
    sb.set_entries(&players[..3]);
    list.clear();
    sb.draw_into(&mut list);
    // The two stripe colours alternate for rows that are not the local player's.
    let colours: Vec<Color> = list.quads.iter().skip(1).map(|q| q.color).collect();
    assert_eq!(colours.len(), 3);
    assert_ne!(colours[0], colours[1]);
    assert_eq!(colours[0], colours[2]);
}

#[test]
fn the_scoreboard_panel_height_follows_the_rows() {
    let panel_h = |n: u32| {
        let mut sb = Scoreboard::new();
        sb.set_held(true);
        let players: Vec<ScoreEntry> = (0..n).map(|i| entry(i, "x", 0, 0, i as i64)).collect();
        sb.set_entries(&players);
        let mut list = DrawList::default();
        sb.draw_into(&mut list);
        list.quads[0].rect.h
    };
    assert!(close(panel_h(0), 96.0 + 16.0));
    assert!(close(panel_h(1) - panel_h(0), 44.0));
    assert!(close(panel_h(16), 96.0 + 16.0 * 44.0 + 16.0));
    assert!(
        close(panel_h(17) - panel_h(16), 44.0),
        "the more line adds a row of space"
    );
}

#[test]
fn closing_the_chat_puts_the_cursor_back_to_the_start() {
    let mut c = chat();
    c.open(false);
    c.handle_text("abc");
    assert_eq!(c.cursor(), 3);
    c.handle_key(ChatKey::Escape);
    assert_eq!(c.cursor(), 0);
    c.open(false);
    c.handle_text("abc");
    c.handle_key(ChatKey::Enter);
    assert_eq!(c.cursor(), 0);
}

#[test]
fn a_tiny_wrap_width_still_shows_the_cursor_and_the_last_characters() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 0,
        ..ChatConfig::default()
    });
    c.open(false);
    c.handle_text("xyz");
    assert_eq!(c.input_line(), "[All] > z|");
    c.handle_key(ChatKey::Home);
    assert_eq!(c.input_line(), "[All] > |x");
}

#[test]
fn the_input_box_makes_room_below_the_rows_and_each_kind_has_its_colour() {
    let mut c = chat();
    c.push(ChatKind::Player, "Tom", "plain");
    c.push(ChatKind::Team, "Mia", "team");
    c.push(ChatKind::System, "", "system");
    let mut closed = DrawList::default();
    c.draw_into(&mut closed);
    let last = closed.quads.last().unwrap().rect;
    assert!(close(last.y + last.h, 880.0), "rows end at the bottom line");
    c.open(false);
    let mut open = DrawList::default();
    c.draw_into(&mut open);
    let last_row = open.quads[2].rect;
    assert!(
        close(last_row.y + last_row.h, 880.0 - 38.0),
        "rows move up by the input line"
    );
    let colour = |list: &DrawList, text: &str| {
        list.texts
            .iter()
            .find(|t| t.text.contains(text))
            .unwrap()
            .color
    };
    let (plain, team, system) = (
        colour(&closed, "plain"),
        colour(&closed, "team"),
        colour(&closed, "system"),
    );
    assert_ne!(plain, team);
    assert_ne!(plain, system);
    assert_ne!(team, system);
    assert!(
        team.0[1] > team.0[0] && team.0[1] > team.0[2],
        "team is green"
    );
    assert!(system.0[0] > system.0[2], "system is yellowish");
    // The input line is white for everybody and green for the team.
    let all_input = colour(&open, "[All] >");
    assert_eq!(all_input, plain);
    c.handle_key(ChatKey::Tab);
    let mut team_open = DrawList::default();
    c.draw_into(&mut team_open);
    assert_eq!(colour(&team_open, "[Team] >"), team);
}

#[test]
fn a_word_after_a_single_letter_wraps_at_the_space_before_it() {
    let mut c = ChatBox::new(ChatConfig {
        wrap_chars: 5,
        ..ChatConfig::default()
    });
    c.push(ChatKind::System, "", "a bcdefgh");
    assert_eq!(row_texts(&c), vec!["a", "bcdef", "gh"]);
}

#[test]
fn the_plate_is_laid_out_inside_its_box_and_a_far_plate_is_fainter_all_through() {
    let mut list = DrawList::default();
    Nameplates::default().draw_into(&[plate(1, 960.0, 500.0, 5.0)], &mut list);
    let r = list.quads[0].rect;
    let track = list
        .quads
        .iter()
        .find(|q| q.color == Color([0.1, 0.1, 0.1, 1.0]))
        .unwrap()
        .rect;
    assert!(close(track.x, r.x + 6.0) && close(track.w, r.w - 12.0));
    // Far away: the box, the bars and the text are all fainter than up close.
    let mut far = DrawList::default();
    Nameplates::default().draw_into(&[plate(1, 960.0, 500.0, 60.0)], &mut far);
    assert!(far.quads[0].color.0[3] < list.quads[0].color.0[3]);
    assert!(close(far.quads[0].color.0[3], 0.45 * 0.35));
}

#[test]
fn a_ping_projected_to_an_infinite_distance_has_no_distance_text() {
    let (p, id) = one_ping(100);
    let mut list = DrawList::default();
    p.draw_into(&[proj(id, 700.0, 300.0, f32::INFINITY, true)], &mut list);
    assert!(!texts(&list).iter().any(|t| t.contains(" m")));
    assert_eq!(
        Direction::from_vector(1.0, f32::NEG_INFINITY),
        Direction::South
    );
    assert_eq!(
        Direction::from_vector(f32::NEG_INFINITY, 1.0),
        Direction::South
    );
}

#[test]
fn the_label_and_the_details_are_stacked_the_right_way_round_and_a_long_label_stays_on_the_canvas()
{
    let (p, id) = one_ping(100);
    let at =
        |list: &DrawList, text: &str| list.texts.iter().find(|t| t.text == text).unwrap().pos.y;
    let mut top = DrawList::default();
    p.draw_into(&[proj(id, 960.0, 200.0, 10.0, true)], &mut top);
    assert!(
        at(&top, "Here") < at(&top, "Mia, 10 m"),
        "label first when the text is below"
    );
    let mut bottom = DrawList::default();
    p.draw_into(&[proj(id, 960.0, 900.0, 10.0, true)], &mut bottom);
    assert!(
        at(&bottom, "Here") > at(&bottom, "Mia, 10 m"),
        "details first when the text is above"
    );

    let mut long = PingMarkers::new();
    let id = long
        .add(1, "Mia", PingKind::Look, &"L".repeat(24), 100)
        .unwrap();
    for (x, left) in [(-500.0, true), (9000.0, false)] {
        let mut list = DrawList::default();
        long.draw_into(&[proj(id, x, 540.0, 10.0, true)], &mut list);
        let t = list
            .texts
            .iter()
            .find(|t| t.text.starts_with("LLL"))
            .unwrap();
        let half = estimate_text_width(24, 22.0) / 2.0;
        if left {
            assert!(close(t.pos.x, half + 8.0), "{t:?}");
        } else {
            assert!(close(t.pos.x, CANVAS_W - half - 8.0), "{t:?}");
        }
    }
}

#[test]
fn the_voice_list_sits_at_the_right_edge_with_gaps_and_marks_the_local_row() {
    let mut v = VoiceIndicators::new();
    let mut me = sample(1, 0.5);
    me.local = true;
    v.update(&[me, sample(2, 1.0)]);
    let mut list = DrawList::default();
    v.draw_into(&mut list);
    let rows: Vec<Rect> = list
        .quads
        .iter()
        .filter(|q| q.rect.w > 200.0)
        .map(|q| q.rect)
        .collect();
    assert!(close(rows[0].x, SAFE_AREA.x + SAFE_AREA.w - 300.0));
    assert!(close(rows[1].y - rows[0].y, 46.0));
    let strip = list.quads.iter().find(|q| q.rect.w == 4.0).unwrap().rect;
    assert!(
        close(strip.y, rows[0].y),
        "the orange strip is on the local row (the first)"
    );
    assert!(close(strip.x, rows[0].x));
}

#[test]
fn the_documented_defaults_and_limits_are_what_the_documentation_says() {
    let c = ChatConfig::default();
    assert_eq!(
        (
            c.visible_ticks,
            c.fade_ticks,
            c.max_stored,
            c.max_rows,
            c.wrap_chars,
            c.max_message_chars
        ),
        (600, 60, 100, 8, 56, 200)
    );
    let n = NameplateConfig::default();
    assert_eq!(
        (n.max_count, n.near, n.far, n.min_scale, n.min_alpha),
        (8, 15.0, 60.0, 0.6, 0.35)
    );
    assert_eq!(n.safe, SAFE_AREA);
    assert_eq!(
        (
            MAX_NAME_CHARS,
            MAX_SCOREBOARD_ROWS,
            MAX_PINGS,
            MAX_PING_LABEL_CHARS,
            PING_FADE_TICKS
        ),
        (20, 16, 16, 24, 30)
    );
    assert_eq!(
        (MAX_SPEAKERS, VOICE_HOLD_TICKS, MAX_VOICE_NAME_CHARS),
        (6, 20, 14)
    );
    assert!(close(VOICE_THRESHOLD, 0.02));
}

#[test]
fn an_empty_label_leaves_no_gap_between_the_marker_and_its_details() {
    let mut p = PingMarkers::new();
    let id = p.add(1, "Mia", PingKind::Look, "", 100).unwrap();
    let detail_y = |y: f32| {
        let mut list = DrawList::default();
        p.draw_into(&[proj(id, 960.0, y, 10.0, true)], &mut list);
        list.texts
            .iter()
            .find(|t| t.text == "Mia, 10 m")
            .unwrap()
            .pos
            .y
    };
    // Text below the marker starts 4 under it; text above ends 4 over it, the details being 24 tall.
    assert!(close(detail_y(200.0), 200.0 + 16.0 + 4.0));
    assert!(close(detail_y(900.0), 900.0 - 16.0 - 4.0 - 24.0));
}
