use ratatui::prelude::*;
use ratatui::symbols::border;
use ratatui::widgets::{Block, Widget};
use ratatui::{layout::Flex, widgets::Paragraph};
use tui_big_text::{BigText, PixelSize};

use crate::app::App;

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
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
            Constraint::Max(3),
        ])
        .split(tuner_area);
        let pitches_layout = Layout::horizontal([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
        ])
        .flex(Flex::SpaceBetween)
        .split(tuner_layout[0]);
        for i in 0..3 {
            let pitch_data = &self.tuner_data.pitches[i];
            let pitch_layout = Layout::horizontal([
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .flex(Flex::Center)
            .split(pitches_layout[i]);

            let pitch_details = Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                .split(pitch_layout[2]);

            let color = if i == 1 { Color::Green } else { Color::Gray };
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

        let up_arrow = Paragraph::new("▲").alignment(Alignment::Center);
        // let down_arrow = Paragraph::new("▼").alignment(Alignment::Center);
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
        let tuner_bar_layout = Layout::horizontal([Constraint::Length(34), Constraint::Length(1)])
            .split(tuner_bar_area);
        let tune_indicator = Paragraph::new("█").alignment(Alignment::Left);

        tuner_bar.render(tuner_layout[1], buf);
        tune_indicator.render(tuner_bar_layout[1], buf);
        up_arrow.render(tuner_layout[2], buf);
    }
}
