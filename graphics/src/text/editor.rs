use crate::core::text::editor::{self, Action, Cursor, Direction, Motion};
use crate::core::text::LineHeight;
use crate::core::{Font, Pixels, Point, Rectangle, Size};
use crate::text;

use cosmic_text::Edit;

use std::fmt;
use std::sync::{self, Arc};

#[derive(Debug, PartialEq)]
pub struct Editor(Option<Arc<Internal>>);

struct Internal {
    editor: cosmic_text::Editor,
    font: Font,
    bounds: Size,
    min_bounds: Size,
    version: text::Version,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffer(&self) -> &cosmic_text::Buffer {
        self.internal().editor.buffer()
    }

    pub fn downgrade(&self) -> Weak {
        let editor = self.internal();

        Weak {
            raw: Arc::downgrade(editor),
            bounds: editor.bounds,
        }
    }

    fn internal(&self) -> &Arc<Internal> {
        self.0
            .as_ref()
            .expect("editor should always be initialized")
    }
}

impl editor::Editor for Editor {
    type Font = Font;

    fn with_text(text: &str) -> Self {
        let mut buffer = cosmic_text::Buffer::new_empty(cosmic_text::Metrics {
            font_size: 1.0,
            line_height: 1.0,
        });

        let mut font_system =
            text::font_system().write().expect("Write font system");

        buffer.set_text(
            font_system.raw(),
            text,
            cosmic_text::Attrs::new(),
            cosmic_text::Shaping::Advanced,
        );

        Editor(Some(Arc::new(Internal {
            editor: cosmic_text::Editor::new(buffer),
            version: font_system.version(),
            ..Default::default()
        })))
    }

    fn cursor(&self) -> editor::Cursor {
        let internal = self.internal();

        let cursor = internal.editor.cursor();
        let buffer = internal.editor.buffer();

        match internal.editor.select_opt() {
            Some(selection) => {
                let (start, end) = if cursor < selection {
                    (cursor, selection)
                } else {
                    (selection, cursor)
                };

                let line_height = buffer.metrics().line_height;
                let selected_lines = end.line - start.line + 1;

                let visual_lines_offset =
                    visual_lines_offset(start.line, buffer);

                let regions = buffer
                    .lines
                    .iter()
                    .skip(start.line)
                    .take(selected_lines)
                    .enumerate()
                    .flat_map(|(i, line)| {
                        highlight_line(
                            line,
                            if i == 0 { start.index } else { 0 },
                            if i == selected_lines - 1 {
                                end.index
                            } else {
                                line.text().len()
                            },
                        )
                    })
                    .enumerate()
                    .filter_map(|(visual_line, (x, width))| {
                        if width > 0.0 {
                            Some(Rectangle {
                                x,
                                width,
                                y: (visual_line as i32 + visual_lines_offset)
                                    as f32
                                    * line_height,
                                height: line_height,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                Cursor::Selection(regions)
            }
            _ => {
                let line_height = buffer.metrics().line_height;

                let visual_lines_offset =
                    visual_lines_offset(cursor.line, buffer);

                let line = buffer
                    .lines
                    .get(cursor.line)
                    .expect("Cursor line should be present");

                let layout = line
                    .layout_opt()
                    .as_ref()
                    .expect("Line layout should be cached");

                let mut lines = layout.iter().enumerate();

                let (visual_line, offset) = lines
                    .find_map(|(i, line)| {
                        let start = line
                            .glyphs
                            .first()
                            .map(|glyph| glyph.start)
                            .unwrap_or(0);
                        let end = line
                            .glyphs
                            .last()
                            .map(|glyph| glyph.end)
                            .unwrap_or(0);

                        let is_cursor_before_start = start > cursor.index;

                        let is_cursor_before_end = match cursor.affinity {
                            cosmic_text::Affinity::Before => {
                                cursor.index <= end
                            }
                            cosmic_text::Affinity::After => cursor.index < end,
                        };

                        if is_cursor_before_start {
                            // Sometimes, the glyph we are looking for is right
                            // between lines. This can happen when a line wraps
                            // on a space.
                            // In that case, we can assume the cursor is at the
                            // end of the previous line.
                            // i is guaranteed to be > 0 because `start` is always
                            // 0 for the first line, so there is no way for the
                            // cursor to be before it.
                            Some((i - 1, layout[i - 1].w))
                        } else if is_cursor_before_end {
                            let offset = line
                                .glyphs
                                .iter()
                                .take_while(|glyph| cursor.index > glyph.start)
                                .map(|glyph| glyph.w)
                                .sum();

                            Some((i, offset))
                        } else {
                            None
                        }
                    })
                    .unwrap_or((
                        layout.len().saturating_sub(1),
                        layout.last().map(|line| line.w).unwrap_or(0.0),
                    ));

                Cursor::Caret(Point::new(
                    offset,
                    (visual_lines_offset + visual_line as i32) as f32
                        * line_height,
                ))
            }
        }
    }

    fn perform(&mut self, action: Action) {
        let mut font_system =
            text::font_system().write().expect("Write font system");

        let editor =
            self.0.take().expect("Editor should always be initialized");

        // TODO: Handle multiple strong references somehow
        let mut internal = Arc::try_unwrap(editor)
            .expect("Editor cannot have multiple strong references");

        let editor = &mut internal.editor;

        match action {
            // Motion events
            Action::Move(motion) => {
                if let Some(selection) = editor.select_opt() {
                    let cursor = editor.cursor();

                    let (left, right) = if cursor < selection {
                        (cursor, selection)
                    } else {
                        (selection, cursor)
                    };

                    editor.set_select_opt(None);

                    match motion {
                        // These motions are performed as-is even when a selection
                        // is present
                        Motion::Home
                        | Motion::End
                        | Motion::DocumentStart
                        | Motion::DocumentEnd => {
                            editor.action(
                                font_system.raw(),
                                motion_to_action(motion),
                            );
                        }
                        // Other motions simply move the cursor to one end of the selection
                        _ => editor.set_cursor(match motion.direction() {
                            Direction::Left => left,
                            Direction::Right => right,
                        }),
                    }
                } else {
                    editor.action(font_system.raw(), motion_to_action(motion));
                }
            }

            // Selection events
            Action::Select(motion) => {
                let cursor = editor.cursor();

                if editor.select_opt().is_none() {
                    editor.set_select_opt(Some(cursor));
                }

                editor.action(font_system.raw(), motion_to_action(motion));

                // Deselect if selection matches cursor position
                if let Some(selection) = editor.select_opt() {
                    let cursor = editor.cursor();

                    if cursor.line == selection.line
                        && cursor.index == selection.index
                    {
                        editor.set_select_opt(None);
                    }
                }
            }
            Action::SelectWord => {
                use unicode_segmentation::UnicodeSegmentation;

                let cursor = editor.cursor();

                if let Some(line) = editor.buffer().lines.get(cursor.line) {
                    let (start, end) =
                        UnicodeSegmentation::unicode_word_indices(line.text())
                            // Split words with dots
                            .flat_map(|(i, word)| {
                                word.split('.').scan(i, |current, word| {
                                    let start = *current;
                                    *current += word.len() + 1;

                                    Some((start, word))
                                })
                            })
                            // Turn words into ranges
                            .map(|(i, word)| (i, i + word.len()))
                            // Find the word at cursor
                            .find(|&(start, end)| {
                                start <= cursor.index && cursor.index < end
                            })
                            // Cursor is not in a word. Let's select its punctuation cluster.
                            .unwrap_or_else(|| {
                                let start = line.text()[..cursor.index]
                                    .char_indices()
                                    .rev()
                                    .take_while(|(_, c)| {
                                        c.is_ascii_punctuation()
                                    })
                                    .map(|(i, _)| i)
                                    .last()
                                    .unwrap_or(cursor.index);

                                let end = line.text()[cursor.index..]
                                    .char_indices()
                                    .skip_while(|(_, c)| {
                                        c.is_ascii_punctuation()
                                    })
                                    .map(|(i, _)| i + cursor.index)
                                    .next()
                                    .unwrap_or(cursor.index);

                                (start, end)
                            });

                    if start != end {
                        editor.set_cursor(cosmic_text::Cursor {
                            index: start,
                            ..cursor
                        });

                        editor.set_select_opt(Some(cosmic_text::Cursor {
                            index: end,
                            ..cursor
                        }));
                    }
                }
            }
            Action::SelectLine => {
                let cursor = editor.cursor();

                if let Some(line_length) = editor
                    .buffer()
                    .lines
                    .get(cursor.line)
                    .map(|line| line.text().len())
                {
                    editor
                        .set_cursor(cosmic_text::Cursor { index: 0, ..cursor });

                    editor.set_select_opt(Some(cosmic_text::Cursor {
                        index: line_length,
                        ..cursor
                    }));
                }
            }

            // Editing events
            Action::Insert(c) => {
                editor
                    .action(font_system.raw(), cosmic_text::Action::Insert(c));
            }
            Action::Enter => {
                editor.action(font_system.raw(), cosmic_text::Action::Enter);
            }
            Action::Backspace => {
                editor
                    .action(font_system.raw(), cosmic_text::Action::Backspace);
            }
            Action::Delete => {
                editor.action(font_system.raw(), cosmic_text::Action::Delete);
            }

            // Mouse events
            Action::Click(position) => {
                editor.action(
                    font_system.raw(),
                    cosmic_text::Action::Click {
                        x: position.x as i32,
                        y: position.y as i32,
                    },
                );
            }
            Action::Drag(position) => {
                editor.action(
                    font_system.raw(),
                    cosmic_text::Action::Drag {
                        x: position.x as i32,
                        y: position.y as i32,
                    },
                );

                // Deselect if selection matches cursor position
                if let Some(selection) = editor.select_opt() {
                    let cursor = editor.cursor();

                    if cursor.line == selection.line
                        && cursor.index == selection.index
                    {
                        editor.set_select_opt(None);
                    }
                }
            }
        }

        editor.shape_as_needed(font_system.raw());

        self.0 = Some(Arc::new(internal));
    }

    fn bounds(&self) -> Size {
        self.internal().bounds
    }

    fn update(
        &mut self,
        new_bounds: Size,
        new_font: Font,
        new_size: Pixels,
        new_line_height: LineHeight,
    ) {
        let editor =
            self.0.take().expect("editor should always be initialized");

        let mut internal = Arc::try_unwrap(editor)
            .expect("Editor cannot have multiple strong references");

        let mut font_system =
            text::font_system().write().expect("Write font system");

        let mut changed = false;

        if font_system.version() != internal.version {
            for line in internal.editor.buffer_mut().lines.iter_mut() {
                line.reset();
            }

            changed = true;
        }

        if new_font != internal.font {
            for line in internal.editor.buffer_mut().lines.iter_mut() {
                let _ = line.set_attrs_list(cosmic_text::AttrsList::new(
                    text::to_attributes(new_font),
                ));
            }

            changed = true;
        }

        let metrics = internal.editor.buffer().metrics();
        let new_line_height = new_line_height.to_absolute(new_size);

        if new_size.0 != metrics.font_size
            || new_line_height.0 != metrics.line_height
        {
            internal.editor.buffer_mut().set_metrics(
                font_system.raw(),
                cosmic_text::Metrics::new(new_size.0, new_line_height.0),
            );

            changed = true;
        }

        if new_bounds != internal.bounds {
            internal.editor.buffer_mut().set_size(
                font_system.raw(),
                new_bounds.width,
                new_bounds.height,
            );

            internal.bounds = new_bounds;
            changed = true;
        }

        if changed {
            internal.min_bounds = text::measure(internal.editor.buffer());
        }

        self.0 = Some(Arc::new(internal));
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self(Some(Arc::new(Internal::default())))
    }
}

impl PartialEq for Internal {
    fn eq(&self, other: &Self) -> bool {
        self.font == other.font
            && self.bounds == other.bounds
            && self.min_bounds == other.min_bounds
            && self.editor.buffer().metrics() == other.editor.buffer().metrics()
    }
}

impl Default for Internal {
    fn default() -> Self {
        Self {
            editor: cosmic_text::Editor::new(cosmic_text::Buffer::new_empty(
                cosmic_text::Metrics {
                    font_size: 1.0,
                    line_height: 1.0,
                },
            )),
            font: Font::default(),
            bounds: Size::ZERO,
            min_bounds: Size::ZERO,
            version: text::Version::default(),
        }
    }
}

impl fmt::Debug for Internal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Internal")
            .field("font", &self.font)
            .field("bounds", &self.bounds)
            .field("min_bounds", &self.min_bounds)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Weak {
    raw: sync::Weak<Internal>,
    pub bounds: Size,
}

impl Weak {
    pub fn upgrade(&self) -> Option<Editor> {
        self.raw.upgrade().map(Some).map(Editor)
    }
}

impl PartialEq for Weak {
    fn eq(&self, other: &Self) -> bool {
        match (self.raw.upgrade(), other.raw.upgrade()) {
            (Some(p1), Some(p2)) => p1 == p2,
            _ => false,
        }
    }
}

fn highlight_line(
    line: &cosmic_text::BufferLine,
    from: usize,
    to: usize,
) -> impl Iterator<Item = (f32, f32)> + '_ {
    let layout = line
        .layout_opt()
        .as_ref()
        .expect("Line layout should be cached");

    layout.iter().map(move |visual_line| {
        let start = visual_line
            .glyphs
            .first()
            .map(|glyph| glyph.start)
            .unwrap_or(0);
        let end = visual_line
            .glyphs
            .last()
            .map(|glyph| glyph.end)
            .unwrap_or(0);

        let range = start.max(from)..end.min(to);

        if range.is_empty() {
            (0.0, 0.0)
        } else if range.start == start && range.end == end {
            (0.0, visual_line.w)
        } else {
            let first_glyph = visual_line
                .glyphs
                .iter()
                .position(|glyph| range.start <= glyph.start)
                .unwrap_or(0);

            let mut glyphs = visual_line.glyphs.iter();

            let x =
                glyphs.by_ref().take(first_glyph).map(|glyph| glyph.w).sum();

            let width: f32 = glyphs
                .take_while(|glyph| range.end > glyph.start)
                .map(|glyph| glyph.w)
                .sum();

            (x, width)
        }
    })
}

fn visual_lines_offset(line: usize, buffer: &cosmic_text::Buffer) -> i32 {
    let visual_lines_before_start: usize = buffer
        .lines
        .iter()
        .take(line)
        .map(|line| {
            line.layout_opt()
                .as_ref()
                .expect("Line layout should be cached")
                .len()
        })
        .sum();

    visual_lines_before_start as i32 - buffer.scroll()
}

fn motion_to_action(motion: Motion) -> cosmic_text::Action {
    match motion {
        Motion::Left => cosmic_text::Action::Left,
        Motion::Right => cosmic_text::Action::Right,
        Motion::Up => cosmic_text::Action::Up,
        Motion::Down => cosmic_text::Action::Down,
        Motion::WordLeft => cosmic_text::Action::LeftWord,
        Motion::WordRight => cosmic_text::Action::RightWord,
        Motion::Home => cosmic_text::Action::Home,
        Motion::End => cosmic_text::Action::End,
        Motion::PageUp => cosmic_text::Action::PageUp,
        Motion::PageDown => cosmic_text::Action::PageDown,
        Motion::DocumentStart => cosmic_text::Action::BufferStart,
        Motion::DocumentEnd => cosmic_text::Action::BufferEnd,
    }
}
