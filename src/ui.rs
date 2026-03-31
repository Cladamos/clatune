use derive_setters::Setters;
use ratatui::prelude::*;
use ratatui::symbols::border;

use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Widget};
use ratatui::{layout::Flex, widgets::Paragraph};
use tui_big_text::{BigText, PixelSize};

use crate::app::{App, TunerData};
struct CenterOpts {
    width: u16,
    height: u16,
    margin: u16,
}

fn centered_rect(opts: CenterOpts, r: Rect) -> Rect {
    let padding_vertical = r.height.saturating_sub(opts.height) / 2;
    let padding_horizontal = r.width.saturating_sub(opts.width) / 2;

    Rect {
        x: r.x + padding_horizontal,
        y: r.y + padding_vertical,
        width: opts.width.min(r.width),
        height: opts.height.min(r.height),
    }
    .inner(Margin {
        horizontal: opts.margin,
        vertical: 0,
    })
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let instructions = Line::from(vec![
            "[i -> Input Devices] ".fg(Color::DarkGray),
            "[q, ctrl+c -> Quit] ".fg(Color::DarkGray),
        ]);
        let instruction_block = Block::default()
            .title_bottom(instructions)
            .title_alignment(Alignment::Center);
        instruction_block.render(area, buf);

        let tuner = Tuner {
            data: self.tuner_data.clone(),
        };
        tuner.render(area, buf);

        let popup_area = centered_rect(
            CenterOpts {
                width: area.width / 2,
                height: area.height / 3,
                margin: 0,
            },
            area,
        );

        let popup_instracttions = Line::from(vec![
            "[↑/k -> Up] ".fg(Color::Blue),
            "[↓/j -> Down] ".fg(Color::Blue),
            "[Enter -> Select]".fg(Color::Blue),
        ]);

        let mut popup_list_state = ListState::default();
        popup_list_state.select(Some(self.list_selected_index));

        let mut popup = Popup::default()
            .content(self.devices.clone())
            .title("Input Devices")
            .bottom_title(popup_instracttions)
            .title_style(Style::new().blue().bold())
            .border_style(Style::new().blue())
            .list_state(popup_list_state);
        if self.is_popup_open {
            popup.render(popup_area, buf);
        }
    }
}
#[derive(Debug, Default)]
struct Tuner {
    data: TunerData,
}

impl Widget for &Tuner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let tuner_area = centered_rect(
            CenterOpts {
                width: 71,
                height: 9,
                margin: 0,
            },
            area,
        );
        let tuner_layout = Layout::vertical([
            Constraint::Max(2),
            Constraint::Max(3),
            Constraint::Max(1),
            Constraint::Length(1),
            Constraint::Max(1),
        ])
        .flex(Flex::Center)
        .split(tuner_area);

        let pitches_layout = Layout::horizontal([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
        ])
        .flex(Flex::SpaceBetween)
        .split(tuner_layout[0]);
        let color_according_to_cent = if self.data.cent.abs() > 50 {
            Color::Red
        } else if self.data.cent.abs() > 25 {
            Color::Yellow
        } else {
            Color::Green
        };

        for i in 0..3 {
            let color = if i == 1 {
                color_according_to_cent
            } else {
                Color::Gray
            };
            let pitch_data = &self.data.pitches[i];
            let pitch_layout = Layout::horizontal([
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .split(pitches_layout[i]);

            let pitch_details = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                .split(pitch_layout[2]);

            let note = BigText::builder()
                .pixel_size(PixelSize::Octant)
                .lines(vec![pitch_data.note.clone().fg(color).into()])
                .centered()
                .build();
            let octave = Paragraph::new(pitch_data.octave.to_string()).fg(color);
            if pitch_data.is_sharp {
                let sharp = Paragraph::new("#").fg(color).bold();
                sharp.render(pitch_details[0], buf);
            }
            note.render(pitch_layout[1], buf);
            octave.render(pitch_details[1], buf);
        }

        // let down_arrow = Paragraph::new("▼").alignment(Alignment::Center);
        let tune_indicator = Paragraph::new("█").alignment(Alignment::Left);
        let up_arrow = Paragraph::new("▲").alignment(Alignment::Center);

        let tuner_bar = Block::bordered().border_set(border::ROUNDED);
        let tuner_bar_area = centered_rect(
            CenterOpts {
                width: 71,
                height: 1,
                margin: 1,
            },
            tuner_layout[1],
        );

        // We have 69 character long bar LMAO so
        // 0 margin means it's most left
        // 34 left margin means it's centered
        // 68 left margin means it's most right
        // First constraint is the left margin, second is the bar itself
        let cent = self.data.cent;
        let indicator_margin: u16 = if cent < 0 {
            34 - (cent * -34 / 100) as u16
        } else {
            34 + (cent * 34 / 100) as u16
        };
        let tuner_bar_layout =
            Layout::horizontal([Constraint::Length(indicator_margin), Constraint::Length(1)])
                .split(tuner_bar_area);
        let cent_paragraph = Paragraph::new(format!("{:+} cents", cent))
            .fg(color_according_to_cent)
            .alignment(Alignment::Center);

        tune_indicator.render(tuner_bar_layout[1], buf);
        up_arrow.render(tuner_layout[2], buf);
        tuner_bar.render(tuner_layout[1], buf);
        cent_paragraph.render(tuner_layout[4], buf);
    }
}

#[derive(Debug, Default, Setters)]
struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    bottom_title: Line<'a>,
    content: Vec<String>,
    border_style: Style,
    title_style: Style,
    list_state: ListState,
}

impl Widget for &mut Popup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let block = Block::new()
            .title(self.title.clone())
            .title_bottom(self.bottom_title.clone())
            .title_alignment(Alignment::Center)
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(self.border_style)
            .border_type(BorderType::Rounded);

        let list_items = self.content.iter().map(|s| ListItem::new(s.clone()));
        let list = List::new(list_items)
            .block(block)
            .highlight_style(Style::new().reversed())
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);
        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}
