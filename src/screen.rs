use core::fmt;
use core::ops::{Deref, DerefMut};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::lazy_lock::LazyLock;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_time::{Duration, Ticker};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyleBuilder};
use embedded_graphics::pixelcolor::{Rgb565, Rgb888};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::primitives::PrimitiveStyle;
use embedded_graphics::Pixel;
use embedded_graphics::text::Text;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::SPI1;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9488Rgb565;

extern crate alloc;
use alloc::vec::Vec;
use alloc::vec;

pub const SCREEN_HEIGHT: u16 = 320;
pub const SCREEN_WIDTH: u16 = 320;

// Define PicoCalcDisplay here so it can be used in main.rs and here
pub type PicoCalcDisplay<'a> = mipidsi::Display<
    SpiInterface<
        'a,
        SpiDeviceWithConfig<
            'a,
            NoopRawMutex,
            embassy_rp::spi::Spi<'a, SPI1, embassy_rp::spi::Blocking>,
            Output<'a>,
        >,
        Output<'a>,
    >,
    ILI9488Rgb565,
    Output<'a>,
>;

static FONTS: &[&MonoFont] = &[
    &profont::PROFONT_7_POINT,
    &profont::PROFONT_9_POINT,
    &profont::PROFONT_10_POINT,
    &profont::PROFONT_12_POINT,
    &profont::PROFONT_14_POINT,
    &profont::PROFONT_18_POINT,
    &profont::PROFONT_24_POINT,
];

pub static SCREEN: LazyLock<AsyncMutex<CriticalSectionRawMutex, Screen>> =
    LazyLock::new(|| AsyncMutex::new(Screen::new()));

pub struct Screen {
    model: ScreenModel,
    parser: vte::Parser,
}

impl Deref for Screen {
    type Target = ScreenModel;
    fn deref(&self) -> &ScreenModel {
        &self.model
    }
}

impl DerefMut for Screen {
    fn deref_mut(&mut self) -> &mut ScreenModel {
        &mut self.model
    }
}

impl Screen {
    pub fn new() -> Self {
        Self {
            model: ScreenModel::default(),
            parser: vte::Parser::new(),
        }
    }

    pub fn parse_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.parser.advance(&mut self.model, *byte);
        }
    }

    pub fn print(&mut self, text: &str) {
        self.parse_bytes(text.as_bytes())
    }
    
    pub fn clear(&mut self) {
        self.model.clear();
    }
}

impl fmt::Write for Screen {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.print(s);
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    DefaultFg,
    DefaultBg,
    Rgb(u8, u8, u8),
    Indexed(u8),
}

impl Color {
    fn to_rgb565(self, is_bg: bool) -> Rgb565 {
        match self {
            Color::Black => Rgb565::BLACK,
            Color::Red => Rgb565::RED,
            Color::Green => Rgb565::GREEN,
            Color::Yellow => Rgb565::YELLOW,
            Color::Blue => Rgb565::BLUE,
            Color::Magenta => Rgb565::MAGENTA,
            Color::Cyan => Rgb565::CYAN,
            Color::White => Rgb565::WHITE,
            Color::BrightBlack => Rgb565::new(10, 20, 10), // Approx
            Color::BrightRed => Rgb565::new(31, 20, 20),
            Color::BrightGreen => Rgb565::new(20, 63, 20),
            Color::BrightYellow => Rgb565::new(31, 63, 20),
            Color::BrightBlue => Rgb565::new(20, 20, 31),
            Color::BrightMagenta => Rgb565::new(31, 20, 31),
            Color::BrightCyan => Rgb565::new(20, 63, 31),
            Color::BrightWhite => Rgb565::WHITE,
            Color::DefaultFg => Rgb565::CSS_LIGHT_GRAY,
            Color::DefaultBg => Rgb565::BLACK,
            Color::Rgb(r, g, b) => Rgb888::new(r, g, b).into(),
            Color::Indexed(i) => {
                // Simple mapping for first 16 colors, else default
                if i < 8 {
                    // map to standard colors
                    match i {
                        0 => Rgb565::BLACK,
                        1 => Rgb565::RED,
                        2 => Rgb565::GREEN,
                        3 => Rgb565::YELLOW,
                        4 => Rgb565::BLUE,
                        5 => Rgb565::MAGENTA,
                        6 => Rgb565::CYAN,
                        7 => Rgb565::CSS_LIGHT_GRAY,
                        _ => Rgb565::WHITE,
                    }
                } else if i < 16 {
                    // brights
                     match i {
                        8 => Rgb565::new(10, 20, 10),
                        9 => Rgb565::new(31, 20, 20),
                        10 => Rgb565::new(20, 63, 20),
                        11 => Rgb565::new(31, 63, 20),
                        12 => Rgb565::new(20, 20, 31),
                        13 => Rgb565::new(31, 20, 31),
                        14 => Rgb565::new(20, 63, 31),
                        15 => Rgb565::WHITE,
                        _ => Rgb565::WHITE,
                    }
                } else {
                    if is_bg { Rgb565::BLACK } else { Rgb565::WHITE }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Attrs {
    fg: Color,
    bg: Color,
    bold: bool,
    underline: bool,
    reverse: bool,
}

impl Default for Attrs {
    fn default() -> Self {
        Self {
            fg: Color::DefaultFg,
            bg: Color::DefaultBg,
            bold: false,
            underline: false,
            reverse: false,
        }
    }
}

#[derive(Clone)]
struct ScreenLine {
    chars: Vec<char>,
    attrs: Vec<Attrs>,
    dirty: bool,
}

impl ScreenLine {
    fn new(width: usize) -> Self {
        Self {
            chars: vec![' '; width],
            attrs: vec![Attrs::default(); width],
            dirty: true,
        }
    }
    
    fn clear(&mut self) {
        for c in self.chars.iter_mut() { *c = ' '; }
        for a in self.attrs.iter_mut() { *a = Attrs::default(); }
        self.dirty = true;
    }
}

pub struct ScreenModel {
    lines: Vec<ScreenLine>,
    cursor_x: usize,
    cursor_y: usize,
    current_attrs: Attrs,
    font: &'static MonoFont<'static>,
    rows: usize,
    cols: usize,
    full_repaint: bool,
}

impl Default for ScreenModel {
    fn default() -> Self {
        let font = FONTS[2];
        let cols = ((SCREEN_WIDTH as u32) / (font.character_size.width + font.character_spacing)) as usize;
        let rows = ((SCREEN_HEIGHT as u32) / font.character_size.height) as usize;
        
        // Initialize lines
        let mut lines = Vec::with_capacity(rows);
        for _ in 0..rows {
            lines.push(ScreenLine::new(cols));
        }

        Self {
            lines,
            cursor_x: 0,
            cursor_y: 0,
            current_attrs: Attrs::default(),
            font,
            rows,
            cols,
            full_repaint: true,
        }
    }
}

impl ScreenModel {
    pub fn width(&self) -> u16 {
        self.cols as u16
    }

    pub fn height(&self) -> u16 {
        self.rows as u16
    }
    
    pub fn clear(&mut self) {
        for line in self.lines.iter_mut() {
            line.clear();
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.full_repaint = true;
    }

    pub fn increase_font(&mut self) {
        // TODO: implement font resizing
    }

    pub fn decrease_font(&mut self) {
        // TODO: implement font resizing
    }

    fn scroll_up(&mut self) {
        // Remove first line, add new line at end
        if !self.lines.is_empty() {
            self.lines.remove(0);
            self.lines.push(ScreenLine::new(self.cols));
            self.full_repaint = true; // Simple repaint for scroll
        }
    }

    pub fn update_display(&mut self, display: &mut PicoCalcDisplay) {
        if self.full_repaint {
            display.clear(Rgb565::BLACK).unwrap();
        }

        let font = self.font;
        let cell_width = font.character_size.width + font.character_spacing;
        let cell_height = font.character_size.height;

        for (y, line) in self.lines.iter_mut().enumerate() {
            if !line.dirty && !self.full_repaint {
                continue;
            }
            
            let row_y = y as u32 * cell_height as u32;
            if row_y >= SCREEN_HEIGHT as u32 { break; }

            for (x, (char, attr)) in line.chars.iter().zip(line.attrs.iter()).enumerate() {
                let col_x = x as u32 * cell_width;
                if col_x >= SCREEN_WIDTH as u32 { break; }

                let mut fg = attr.fg.to_rgb565(false);
                let mut bg = attr.bg.to_rgb565(true);
                
                if attr.reverse {
                    core::mem::swap(&mut fg, &mut bg);
                }
                
                if attr.bold {
                    // Brighten fg?
                    if fg == Rgb565::CSS_LIGHT_GRAY { fg = Rgb565::WHITE; }
                }

                // Draw background
                display.fill_solid(
                    &Rectangle::new(
                        Point::new(col_x as i32, row_y as i32),
                        Size::new(cell_width, cell_height as u32),
                    ),
                    bg,
                ).unwrap();

                // Draw text
                if *char != ' ' {
                     let style = MonoTextStyleBuilder::new()
                        .font(font)
                        .text_color(fg)
                        .background_color(bg)
                        .build();
                    
                    // We need to handle char string
                    let mut buf = [0u8; 4];
                    let s = char.encode_utf8(&mut buf);

                    // Check for box drawing characters (U+2500 - U+259F)
                    if ('\u{2500}'..='\u{259F}').contains(char) {
                        draw_box_char(display, *char, col_x as i32, row_y as i32, cell_width, cell_height as u32, fg);
                    } else {
                        Text::new(
                            s,
                            Point::new(col_x as i32, (row_y as i32 + font.baseline as i32)),
                            style,
                        )
                        .draw(display)
                        .ok(); // Ignore errors for missing glyphs
                    }
                }
                
                if attr.underline {
                     display.fill_solid(
                        &Rectangle::new(
                            Point::new(col_x as i32, (row_y + cell_height as u32 - 1) as i32),
                            Size::new(cell_width, 1),
                        ),
                        fg,
                    ).unwrap();
                }
            }
            line.dirty = false;
        }
        self.full_repaint = false;

        // Draw cursor
        let cx = self.cursor_x as u32 * cell_width;
        let cy = self.cursor_y as u32 * cell_height as u32;
        if cx < SCREEN_WIDTH as u32 && cy < SCREEN_HEIGHT as u32 {
             display.fill_solid(
                &Rectangle::new(
                    Point::new(cx as i32, cy as i32),
                    Size::new(cell_width, cell_height as u32),
                ),
                Rgb565::WHITE, 
            ).ok();
        }
    }
}

impl vte::Perform for ScreenModel {
    fn print(&mut self, c: char) {
        if self.cursor_y >= self.rows {
            self.scroll_up();
            self.cursor_y = self.rows - 1;
        }
        if self.cursor_x >= self.cols {
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= self.rows {
                self.scroll_up();
                self.cursor_y = self.rows - 1;
            }
        }
        
        let line = &mut self.lines[self.cursor_y];
        if self.cursor_x < line.chars.len() {
            line.chars[self.cursor_x] = c;
            line.attrs[self.cursor_x] = self.current_attrs;
            line.dirty = true;
            self.cursor_x += 1;
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => { // LF
                self.cursor_y += 1;
                if self.cursor_y >= self.rows {
                    self.scroll_up();
                    self.cursor_y = self.rows - 1;
                }
            }
            b'\r' => { // CR
                self.cursor_x = 0;
            }
            b'\x08' => { // BS
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore || !intermediates.is_empty() { return; }

        match action {
            'A' => { // Cursor Up
                let n = params.iter().next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_y = self.cursor_y.saturating_sub(n);
            }
            'B' => { // Cursor Down
                let n = params.iter().next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_y = (self.cursor_y + n).min(self.rows - 1);
            }
            'C' => { // Cursor Forward
                let n = params.iter().next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_x = (self.cursor_x + n).min(self.cols - 1);
            }
            'D' => { // Cursor Backward
                let n = params.iter().next().map(|p| p[0]).unwrap_or(1) as usize;
                self.cursor_x = self.cursor_x.saturating_sub(n);
            }
            'H' | 'f' => { // Cursor Position
                let mut iter = params.iter();
                let row = iter.next().map(|p| p[0]).unwrap_or(1).max(1) as usize - 1;
                let col = iter.next().map(|p| p[0]).unwrap_or(1).max(1) as usize - 1;
                self.cursor_y = row.min(self.rows - 1);
                self.cursor_x = col.min(self.cols - 1);
            }
            'J' => { // Erase in Display
                let n = params.iter().next().map(|p| p[0]).unwrap_or(0);
                match n {
                    0 => { // Cursor to end
                        // Clear current line from cursor
                        let line = &mut self.lines[self.cursor_y];
                        for i in self.cursor_x..self.cols {
                            line.chars[i] = ' ';
                            line.attrs[i] = self.current_attrs;
                        }
                        line.dirty = true;
                        // Clear lines below
                        for i in (self.cursor_y + 1)..self.rows {
                            self.lines[i].clear();
                        }
                    }
                    1 => { // Beginning to cursor
                        // Clear lines above
                        for i in 0..self.cursor_y {
                            self.lines[i].clear();
                        }
                        // Clear current line up to cursor
                        let line = &mut self.lines[self.cursor_y];
                        for i in 0..=self.cursor_x {
                            line.chars[i] = ' ';
                            line.attrs[i] = self.current_attrs;
                        }
                        line.dirty = true;
                    }
                    2 => { // Entire screen
                        self.clear();
                    }
                    _ => {}
                }
            }
            'K' => { // Erase in Line
                let n = params.iter().next().map(|p| p[0]).unwrap_or(0);
                let line = &mut self.lines[self.cursor_y];
                match n {
                    0 => { // Cursor to end
                        for i in self.cursor_x..self.cols {
                            line.chars[i] = ' ';
                            line.attrs[i] = self.current_attrs;
                        }
                    }
                    1 => { // Beginning to cursor
                        for i in 0..=self.cursor_x {
                            line.chars[i] = ' ';
                            line.attrs[i] = self.current_attrs;
                        }
                    }
                    2 => { // Entire line
                        for i in 0..self.cols {
                            line.chars[i] = ' ';
                            line.attrs[i] = self.current_attrs;
                        }
                    }
                    _ => {}
                }
                line.dirty = true;
            }
            'm' => { // SGR
                for param in params.iter() {
                    let p = param[0];
                    match p {
                        0 => self.current_attrs = Attrs::default(),
                        1 => self.current_attrs.bold = true,
                        4 => self.current_attrs.underline = true,
                        7 => self.current_attrs.reverse = true,
                        22 => self.current_attrs.bold = false,
                        24 => self.current_attrs.underline = false,
                        27 => self.current_attrs.reverse = false,
                        30..=37 => self.current_attrs.fg = Color::Indexed((p - 30) as u8),
                        39 => self.current_attrs.fg = Color::DefaultFg,
                        40..=47 => self.current_attrs.bg = Color::Indexed((p - 40) as u8),
                        49 => self.current_attrs.bg = Color::DefaultBg,
                        90..=97 => self.current_attrs.fg = Color::Indexed((p - 90 + 8) as u8),
                        100..=107 => self.current_attrs.bg = Color::Indexed((p - 100 + 8) as u8),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    
    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

#[embassy_executor::task]
pub async fn screen_painter(mut display: PicoCalcDisplay<'static>) {
    display.clear(Rgb565::BLACK).unwrap();
    if let Err(err) = display.set_vertical_scroll_region(0, 0) {
        // log::error!("failed to set_vertical_scroll_region: {err:?}");
    }

    let mut ticker = Ticker::every(Duration::from_millis(200));
    loop {
        SCREEN.get().lock().await.update_display(&mut display);
        ticker.next().await;
    }
}

pub async fn cls_command(_args: &[&str]) {
    SCREEN.get().lock().await.clear();
}

fn draw_box_char(
    display: &mut PicoCalcDisplay,
    c: char,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: Rgb565,
) {
    let cx = x + (w / 2) as i32;
    let cy = y + (h / 2) as i32;
    let stroke = 1; // Line thickness

    // Helper to draw line
    let line = |display: &mut PicoCalcDisplay, x0, y0, x1, y1| {
        Line::new(Point::new(x0, y0), Point::new(x1, y1))
            .into_styled(PrimitiveStyle::with_stroke(color, stroke))
            .draw(display)
            .ok();
    };

    match c {
        // Light horizontal
        '\u{2500}' => line(display, x, cy, x + w as i32, cy),
        // Light vertical
        '\u{2502}' => line(display, cx, y, cx, y + h as i32),
        // Light down and right
        '\u{250C}' => {
            line(display, cx, cy, x + w as i32, cy);
            line(display, cx, cy, cx, y + h as i32);
        }
        // Light down and left
        '\u{2510}' => {
            line(display, x, cy, cx, cy);
            line(display, cx, cy, cx, y + h as i32);
        }
        // Light up and right
        '\u{2514}' => {
            line(display, cx, cy, x + w as i32, cy);
            line(display, cx, y, cx, cy);
        }
        // Light up and left
        '\u{2518}' => {
            line(display, x, cy, cx, cy);
            line(display, cx, y, cx, cy);
        }
        // Light vertical and right
        '\u{251C}' => {
            line(display, cx, y, cx, y + h as i32);
            line(display, cx, cy, x + w as i32, cy);
        }
        // Light vertical and left
        '\u{2524}' => {
            line(display, cx, y, cx, y + h as i32);
            line(display, x, cy, cx, cy);
        }
        // Light horizontal and down
        '\u{252C}' => {
            line(display, x, cy, x + w as i32, cy);
            line(display, cx, cy, cx, y + h as i32);
        }
        // Light horizontal and up
        '\u{2534}' => {
            line(display, x, cy, x + w as i32, cy);
            line(display, cx, y, cx, cy);
        }
        // Light vertical and horizontal
        '\u{253C}' => {
            line(display, x, cy, x + w as i32, cy);
            line(display, cx, y, cx, y + h as i32);
        }
        // Heavy horizontal
        '\u{2501}' => {
             Line::new(Point::new(x, cy), Point::new(x + w as i32, cy))
            .into_styled(PrimitiveStyle::with_stroke(color, 2))
            .draw(display)
            .ok();
        }
         // Heavy vertical
        '\u{2503}' => {
             Line::new(Point::new(cx, y), Point::new(cx, y + h as i32))
            .into_styled(PrimitiveStyle::with_stroke(color, 2))
            .draw(display)
            .ok();
        }
        // Block
        '\u{2588}' => {
            display.fill_solid(
                &Rectangle::new(Point::new(x, y), Size::new(w, h)),
                color
            ).ok();
        }
        // Upper half block
        '\u{2580}' => {
            display.fill_solid(
                &Rectangle::new(Point::new(x, y), Size::new(w, h / 2)),
                color
            ).ok();
        }
        // Lower half block
        '\u{2584}' => {
            display.fill_solid(
                &Rectangle::new(Point::new(x, y + (h / 2) as i32), Size::new(w, h - h / 2)),
                color
            ).ok();
        }
        // Shades
        '\u{2591}' => draw_shade(display, x, y, w, h, color, 1),
        '\u{2592}' => draw_shade(display, x, y, w, h, color, 2),
        '\u{2593}' => draw_shade(display, x, y, w, h, color, 3),

        // Rounded corners
        '\u{256D}' => { // Top-left
            Arc::new(Point::new(x + w as i32 / 2, y + h as i32 / 2), w, Angle::from_degrees(180.0), Angle::from_degrees(90.0))
                .into_styled(PrimitiveStyle::with_stroke(color, stroke))
                .draw(display).ok();
             line(display, cx, cy + h as i32 / 2, cx, y + h as i32); // Extend down
             line(display, cx + w as i32 / 2, cy, x + w as i32, cy); // Extend right
        }
        '\u{256E}' => { // Top-right
             Arc::new(Point::new(x - w as i32 / 2, y + h as i32 / 2), w, Angle::from_degrees(270.0), Angle::from_degrees(90.0))
                .into_styled(PrimitiveStyle::with_stroke(color, stroke))
                .draw(display).ok();
             line(display, cx, cy + h as i32 / 2, cx, y + h as i32); // Extend down
             line(display, x, cy, cx - w as i32 / 2, cy); // Extend left
        }
        '\u{2570}' => { // Bottom-left
             Arc::new(Point::new(x + w as i32 / 2, y - h as i32 / 2), w, Angle::from_degrees(90.0), Angle::from_degrees(90.0))
                .into_styled(PrimitiveStyle::with_stroke(color, stroke))
                .draw(display).ok();
             line(display, cx, y, cx, cy - h as i32 / 2); // Extend up
             line(display, cx + w as i32 / 2, cy, x + w as i32, cy); // Extend right
        }
        '\u{256F}' => { // Bottom-right
             Arc::new(Point::new(x - w as i32 / 2, y - h as i32 / 2), w, Angle::from_degrees(0.0), Angle::from_degrees(90.0))
                .into_styled(PrimitiveStyle::with_stroke(color, stroke))
                .draw(display).ok();
             line(display, cx, y, cx, cy - h as i32 / 2); // Extend up
             line(display, x, cy, cx - w as i32 / 2, cy); // Extend left
        }

        // Double lines
        '\u{2550}' => { // Horizontal double
            Line::new(Point::new(x, cy - 1), Point::new(x + w as i32, cy - 1))
                .into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(x, cy + 1), Point::new(x + w as i32, cy + 1))
                .into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
        }
        '\u{2551}' => { // Vertical double
            Line::new(Point::new(cx - 1, y), Point::new(cx - 1, y + h as i32))
                .into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx + 1, y), Point::new(cx + 1, y + h as i32))
                .into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
        }
        // Double corners (simplified as single heavy for now to save space/complexity, or proper implementation)
        '\u{2554}' => { // Double down-right
            Line::new(Point::new(cx - 1, cy), Point::new(cx - 1, y + h as i32)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx + 1, cy), Point::new(cx + 1, y + h as i32)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx, cy - 1), Point::new(x + w as i32, cy - 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx, cy + 1), Point::new(x + w as i32, cy + 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
        }
        '\u{2557}' => { // Double down-left
            Line::new(Point::new(cx - 1, cy), Point::new(cx - 1, y + h as i32)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx + 1, cy), Point::new(cx + 1, y + h as i32)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(x, cy - 1), Point::new(cx, cy - 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(x, cy + 1), Point::new(cx, cy + 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
        }
        '\u{255A}' => { // Double up-right
            Line::new(Point::new(cx - 1, y), Point::new(cx - 1, cy)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx + 1, y), Point::new(cx + 1, cy)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx, cy - 1), Point::new(x + w as i32, cy - 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx, cy + 1), Point::new(x + w as i32, cy + 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
        }
        '\u{255D}' => { // Double up-left
            Line::new(Point::new(cx - 1, y), Point::new(cx - 1, cy)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(cx + 1, y), Point::new(cx + 1, cy)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(x, cy - 1), Point::new(cx, cy - 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
            Line::new(Point::new(x, cy + 1), Point::new(cx, cy + 1)).into_styled(PrimitiveStyle::with_stroke(color, 1)).draw(display).ok();
        }

        _ => {
            // Fallback for unhandled box chars: draw a small rectangle
             Rectangle::new(Point::new(x + 2, y + 2), Size::new(w - 4, h - 4))
                .into_styled(PrimitiveStyle::with_stroke(color, 1))
                .draw(display)
                .ok();
        }
    }
}

fn draw_shade(display: &mut PicoCalcDisplay, x: i32, y: i32, w: u32, h: u32, color: Rgb565, density: u8) {
    for py in 0..h {
        for px in 0..w {
            let on = match density {
                1 => (px % 2 == 0) && (py % 2 == 0), // 25%
                2 => (px + py) % 2 == 0, // 50%
                3 => !((px % 2 == 0) && (py % 2 == 0)), // 75%
                _ => false
            };
            if on {
                Pixel(Point::new(x + px as i32, y + py as i32), color).draw(display).ok();
            }
        }
    }
}
