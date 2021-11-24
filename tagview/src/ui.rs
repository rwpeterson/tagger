use crate::app::{App, SettingsMode};
#[allow(unused_imports)]
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Clear, Dataset, Gauge, LineGauge, List,
        ListItem, Paragraph, Row, Sparkline, Table, Tabs, Wrap,
    },
    Frame,
};

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());
    draw_titlebar(f, app, chunks[0]);
    draw_tabbar(f, app, chunks[1]);
    match app.tabs.index {
        0 => draw_counts_tab(f, app, chunks[2]),
        1 => draw_settings_tab(f, app, chunks[2]),
        _ => {}
    }
    draw_footer(f, app, chunks[3]);
}

fn draw_settings_tab<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    if app.live_settings == false {
        let text = vec![
            Spans::from("User input required:"),
            Spans::from(""),
            Spans::from("x - Get current channel settings from tagger"),
            Spans::from("    (Gets only channels with a singles subscription in your config)"),
            Spans::from(""),
            Spans::from("m - Set tagger to use channel settings specified in your config"),
            Spans::from("    (Sets all channel_settings in config)"),
        ];
        f.render_widget(Paragraph::new(text), area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(13),
                    Constraint::Length(12),
                    Constraint::Min(1),
                ]
                .as_ref(),
            )
            .split(area);
        let state = app.settings_state.as_mut().unwrap();
        let channel_items = state
            .channel_settings
            .iter()
            .map(|rs| ListItem::new(format!("Channel {: >2}", rs.ch)))
            .collect::<Vec<ListItem>>();
        let channel_list = List::new(channel_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
            .highlight_symbol(if state.mode == SettingsMode::ChannelSelect {
                ">"
            } else {
                " "
            });
        f.render_stateful_widget(channel_list, chunks[0], &mut state.ch_state);
        let settings_items = vec![
            ListItem::new("Delay"),
            ListItem::new("Threshold"),
            ListItem::new("Invert"),
        ];
        let settings_list = List::new(settings_items)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
            .highlight_symbol(if state.mode == SettingsMode::SettingSelect {
                ">"
            } else {
                " "
            });
        if state.mode != SettingsMode::ChannelSelect {
            f.render_stateful_widget(settings_list, chunks[1], &mut state.st_state);
        } else {
            f.render_widget(Clear, chunks[1]);
        }
    }
}

fn draw_counts_tab<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(1)].as_ref())
        .split(area);
    draw_singles(f, app, chunks[0]);
    draw_coincidences(f, app, chunks[1]);
}

fn draw_titlebar<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let text = vec![Spans::from(vec![
        Span::styled(
            if app.save {
                " Time Tagger RECORDING "
            } else {
                " Time Tagger "
            },
            Style::default()
                .fg(if app.save { Color::Red } else { Color::Green })
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        ),
        Span::raw(" Keys:  Ctrl+C "),
        Span::styled("quit", Style::default().add_modifier(Modifier::DIM)),
        Span::raw("  c "),
        Span::styled(
            "clear error flags",
            Style::default().add_modifier(Modifier::DIM),
        ),
        Span::raw("  r "),
        Span::styled("record tags", Style::default().add_modifier(Modifier::DIM)),
        Span::raw("  ←/→ "),
        Span::styled("cycle tabs", Style::default().add_modifier(Modifier::DIM)),
    ])];
    f.render_widget(Paragraph::new(text), area);
}

fn draw_tabbar<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let titles = app
        .tabs
        .titles
        .iter()
        //.map(|&t| Spans::from(Span::styled(t, Style::default().fg(Color::Gray))))
        .map(|&t| Spans::from(Span::raw(t)))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
        .select(app.tabs.index);
    f.render_widget(tabs, area);
}

fn _draw_sparkline<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let data = app
        .coincs
        .iter()
        .map(|h| *h.get(&(1, 2)).unwrap_or(&0.0))
        .collect::<Vec<f64>>();
    let udata = data.iter().map(|&r| r as u64).collect::<Vec<u64>>();
    let sparkline = Sparkline::default()
        .block(Block::default().title(format!(
            "Coincidences (1, 2): {:>7.0}",
            data.last().unwrap_or(&0.0)
        )))
        .style(Style::default().fg(Color::Red))
        .data(&udata);
    f.render_widget(sparkline, area);
}

fn draw_singles<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let pats = app.pats.lock();
    let nch = pats.len();
    let ncols = 8;
    let nrows = match nch % ncols {
        0 => nch / ncols,
        _ => nch / ncols + 1,
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage((100.0 / nrows as f32) as u16);
            nrows
        ])
        .split(area);

    let mut rc: Vec<Vec<Rect>> = Vec::new();
    for row in rows {
        rc.push(
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Percentage((100.0 / ncols as f32) as u16);
                    ncols
                ])
                .split(row),
        );
    }

    let dur = app.duration;
    let mut singlesvec = pats
        .iter()
        .filter(|(m, _)| m.count_ones() == 1)
        .collect::<Vec<_>>();
    singlesvec.sort();
    let mut chan_iter = singlesvec.iter();
    for row in rc {
        for elem in row {
            if let Some((&m, &ct)) = chan_iter.next() {
                let ch = bit_iter::BitIter::from(m).next().unwrap() + 1;
                let rate = ct as f64 / (dur as f64 * 5e-9);
                let text = Paragraph::new(Spans::from(vec![
                    Span::styled(
                        format!("{:>2}", ch),
                        Style::default().add_modifier(Modifier::BOLD | Modifier::DIM),
                    ),
                    Span::styled(format!("{:>7.0}", rate), Style::default()),
                ]));
                f.render_widget(text, elem);
            }
        }
    }
}

fn draw_coincidences<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let pats = app.pats.lock();
    let nch = pats.len();
    let ncols = 8;
    let nrows = match nch % ncols {
        0 => nch / ncols,
        _ => nch / ncols + 1,
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage((100.0 / nrows as f32) as u16);
            nrows
        ])
        .split(area);

    let mut rc: Vec<Vec<Rect>> = Vec::new();
    for row in rows {
        rc.push(
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Percentage((100.0 / ncols as f32) as u16);
                    ncols
                ])
                .split(row),
        );
    }

    let dur = app.duration;
    let mut coincvec = pats
        .iter()
        .filter(|(m, _)| m.count_ones() == 2)
        .collect::<Vec<_>>();
    coincvec.sort();
    let mut chan_iter = coincvec.iter();
    for row in rc {
        for elem in row {
            if let Some((&m, &ct)) = chan_iter.next() {
                let mut bi = bit_iter::BitIter::from(m);
                let ch_b = bi.next().unwrap() + 1;
                let ch_a = bi.next().unwrap() + 1;
                let rate = ct as f64 / (dur as f64 / 5e-9);
                let text = Paragraph::new(Spans::from(vec![
                    Span::styled(
                        format!("{0}-{1}", ch_b, ch_a),
                        Style::default().add_modifier(Modifier::BOLD | Modifier::DIM),
                    ),
                    Span::styled(format!("{:>7.0}", rate), Style::default()),
                ]));
                f.render_widget(text, elem);
            }
        }
    }
}

fn draw_footer<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let mut errtxt: Vec<Span> = match app.flags.is_empty() {
        true => {
            vec![Span::styled("None", Style::default().fg(Color::Green))]
        }
        false => app
            .flags
            .iter()
            .map(|f| {
                vec![
                    Span::styled(
                        f,
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::REVERSED),
                    ),
                    Span::raw(" "),
                ]
            })
            .flatten()
            .collect(),
    };
    let mut text = vec![
        Span::styled(
            format!(
                "Working dir: {}",
                std::env::current_dir().unwrap().to_str().unwrap()
            ),
            Style::default(),
        ),
        Span::raw(" "),
        Span::styled("Errors: ", Style::default()),
        Span::raw(""),
    ];
    text.append(&mut errtxt);

    f.render_widget(Paragraph::new(Spans::from(text)), area);
}

/// Human-readable string encoding size in bytes with largest metric prefix to three decimal places
pub fn sizefmt(bytes: usize) -> String {
    let oom = (bytes as f64).log10().floor() as u32;
    let pfx = oom as u32 / 3 * 3;
    let rpfx = pfx.saturating_sub(3);
    let value = bytes / 10usize.pow(rpfx);
    let unit = match pfx {
        0 => Some("B"),
        3 => Some("kB"),
        6 => Some("MB"),
        9 => Some("GB"),
        12 => Some("TB"),
        15 => Some("PB"),
        _ => None,
    };
    let int_part = value / 10usize.pow(3);
    let frac_part = value % 10usize.pow(3);
    let space = match unit {
        Some(u) => format!("{0:>3}.{1:03} {2}", int_part, frac_part, u),
        None => format!("{} B", bytes),
    };
    return space;
}

/// Human-readable string encoding size in p, n, µ, m, _, k, M, G, T.
pub fn numfmt(num: f64, dec: usize) -> String {
    match num.is_normal() {
        false => return format!("{:>7.*}", dec, num),
        true => {
            let sgn = num.signum();
            let num = num.abs();
            let oom = num.log10().floor() as i32;
            let pfx = oom as i32 / 3 * 3;
            let value = num / 10f64.powi(pfx);
            let unit = match pfx {
                -12 => Some("p"),
                -9 => Some("n"),
                -6 => Some("µ"),
                -3 => Some("m"),
                0 => None,
                3 => Some("k"),
                6 => Some("M"),
                9 => Some("G"),
                12 => Some("T"),
                15 => Some("P"),
                _ => None,
            };
            let repr = match unit {
                Some(p) => format!("{:>7.*} {}", dec, sgn * value, p),
                None => format!("{:>7.*}", dec, value),
            };
            return repr;
        }
    }
}
