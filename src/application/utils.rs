use ratatui::{
    layout::Layout,
    macros::{span, text},
    style::Style,
    widgets::Widget,
};

const BRAILLE_BITS: [[u8; 2]; 4] = [
    [0x01, 0x08],
    [0x02, 0x10],
    [0x04, 0x20],
    [0x40, 0x80],
];

pub struct BrailleSparkline<'a> {
    data: &'a [usize],
    style: Style,
    max: Option<usize>,
}

impl<'a> BrailleSparkline<'a> {
    pub fn new(data: &'a [usize]) -> Self {
        Self {
            data,
            style: Style::new(),
            max: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }
}

impl Widget for BrailleSparkline<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        buf.set_style(area, self.style);

        if area.width == 0 || area.height == 0 || self.data.is_empty() {
            return;
        }

        let n = area.width as usize;
        let m = area.height as usize;
        let sub_cols = 2 * n;
        let sub_rows = 4 * m;

        let slice = if self.data.len() > sub_cols {
            &self.data[self.data.len() - sub_cols..]
        } else {
            self.data
        };

        let max = self
            .max
            .unwrap_or_else(|| slice.iter().copied().max().unwrap_or(0));
        if max == 0 {
            return;
        }

        let mut masks = vec![0u8; n * m];

        let max_f = max as f64;
        let top_bin = (sub_rows - 1) as f64;

        for (sx, &v) in slice.iter().enumerate() {
            let raw = (v as f64 / max_f * top_bin).round();
            let bin = if raw.is_nan() || raw < 0.0 {
                0
            } else if raw > top_bin {
                sub_rows - 1
            } else {
                raw as usize
            };
            let sy = (sub_rows - 1) - bin;

            let cell_col = sx / 2;
            let cell_row = sy / 4;
            let local_col = sx % 2;
            let local_row = sy % 4;

            masks[cell_row * n + cell_col] |= BRAILLE_BITS[local_row][local_col];
        }

        for cell_row in 0..m {
            for cell_col in 0..n {
                let mask = masks[cell_row * n + cell_col];
                if mask == 0 {
                    continue;
                }
                let glyph = char::from_u32(0x2800 | mask as u32).unwrap();
                let x = area.x + cell_col as u16;
                let y = area.y + cell_row as u16;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(glyph).set_style(self.style);
                }
            }
        }
    }
}

pub struct WidgetList<T> {
    children: Vec<T>,
    layout: Layout,
}

impl<T: Widget> WidgetList<T> {
    pub fn new(layout: Layout, children: impl IntoIterator<Item = T>) -> Self {
        Self {
            layout,
            children: children.into_iter().collect(),
        }
    }
}

impl<T: Widget> Widget for WidgetList<T> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let areas = self.layout.split(area);

        self.children
            .into_iter()
            .zip(areas.iter().cloned())
            .for_each(|(w, a)| {
                w.render(a, buf);
            });
    }
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Widget, R: Widget> Widget for Either<L, R> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match self {
            Either::Left(l) => l.render(area, buf),
            Either::Right(r) => r.render(area, buf),
        }
    }
}

pub struct Separator {
    pub char: char,
    pub style: Style,
}

impl Separator {
    pub const fn new(c: char) -> Self {
        Self {
            char: c,
            style: Style::new(),
        }
    }

    pub const fn styled(self, style: Style) -> Self {
        Self {
            char: self.char,
            style,
        }
    }
}

impl Widget for Separator {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let text: String = std::iter::repeat_n(self.char, area.width as usize).collect();

        let text = span![self.style; text];

        text.render(area, buf);
    }
}
